//! Completion Service - auto-completion operations
//!
//! Functions for LSP completion requests and contextual auto-completion.

use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;
use tracing::{debug, info, warn, Span};

use bsl_shared::domain::metadata_constants::get_collection_kind;
use bsl_shared::domain::{CompletionItem, CompletionKind, TypeMetadataLookup, TypeResolution};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::ir::{ScopeId, SemanticProgram};
use bsl_shared::utils::hash::hash_content;

use super::super::extractors::symbol_extractor::{
    extract_word_at_position, is_identifier_char, utf16_to_byte_offset,
};
use super::completion_ranking::{rank_candidates_with_trace, RankingCandidate};
use super::hover_service::find_variable_type_at_position;
use crate::system::keyword_index::DEFAULT_KEYWORDS;
use crate::system::{
    IndexItemKind, IndexSnapshot, IntellisenseIndexStore, IrCache, ParserCoordinator, SymbolScope,
    TypeKind,
};

pub trait IndexSnapshotSource: Sync {
    fn snapshot(&self) -> IndexSnapshot;
}

impl IndexSnapshotSource for IntellisenseIndexStore {
    fn snapshot(&self) -> IndexSnapshot {
        IntellisenseIndexStore::snapshot(self)
    }
}

impl IndexSnapshotSource for IndexSnapshot {
    fn snapshot(&self) -> IndexSnapshot {
        self.clone()
    }
}

pub const COMPLETION_MAX_ITEMS: usize = 200;
const CONTEXT_WINDOW_CHARS: usize = 256;
static COMPLETION_TRACE_ENABLED: OnceLock<bool> = OnceLock::new();
static COMPLETION_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn completion_trace_enabled() -> bool {
    *COMPLETION_TRACE_ENABLED
        .get_or_init(|| std::env::var("BSL_COMPLETION_TRACE").is_ok())
}

fn next_completion_request_id() -> u64 {
    COMPLETION_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    pub item: CompletionItem,
    pub owner_type: Option<String>,
    pub score: f32,
    pub origin_sources: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct CompletionStats {
    pub total_candidates: usize,
    pub dedup_removed: usize,
    pub score_samples: Vec<f32>,
    pub prefix_exact: usize,
    pub prefix_starts: usize,
    pub prefix_contains: usize,
    pub prefix_none: usize,
    pub member_access: usize,
    pub has_owner: usize,
}

#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub items: Vec<CompletionCandidate>,
    pub is_incomplete: bool,
    pub stats: CompletionStats,
}

pub(crate) struct CompletionAnalysisContext<'a> {
    pub parser: Option<&'a ParserCoordinator>,
    pub ir_cache: Option<&'a IrCache>,
    pub ir_program: Option<Arc<SemanticProgram>>,
    pub resolver: &'a TypeResolver,
    pub file_path: &'a str,
}

/// LSP operations - get completion at position
///
/// # Arguments
/// * `file_content` - File content
/// * `line` - Line number (0-based)
/// * `column` - Column number (UTF-16)
/// * `index` - IntelliSense indexes snapshot store
/// * `metadata_lookup` - Access to type metadata for methods lookup
///
/// # Returns
/// CompletionResult with items and isIncomplete flag
pub async fn get_completion(
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index: &IntellisenseIndexStore,
    metadata_lookup: &TypeMetadataLookup,
) -> Result<CompletionResult> {
    get_completion_with_analysis(
        file_content,
        line,
        column,
        file_uri,
        index,
        metadata_lookup,
        None,
    )
    .await
}

pub async fn get_completion_with_semantic_program(
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index: &IntellisenseIndexStore,
    metadata_lookup: &TypeMetadataLookup,
    file_path: &str,
    resolver: &TypeResolver,
    ir_program: Arc<SemanticProgram>,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        parser: None,
        ir_cache: None,
        ir_program: Some(ir_program),
        resolver,
        file_path,
    };

    get_completion_with_analysis(
        file_content,
        line,
        column,
        file_uri,
        index,
        metadata_lookup,
        Some(&analysis),
    )
    .await
}

pub async fn get_completion_with_semantic_program_snapshot(
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index_snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
    file_path: &str,
    resolver: &TypeResolver,
    ir_program: Arc<SemanticProgram>,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        parser: None,
        ir_cache: None,
        ir_program: Some(ir_program),
        resolver,
        file_path,
    };

    get_completion_with_analysis(
        file_content,
        line,
        column,
        file_uri,
        index_snapshot,
        metadata_lookup,
        Some(&analysis),
    )
    .await
}

pub(crate) async fn get_completion_with_analysis(
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index: &dyn IndexSnapshotSource,
    metadata_lookup: &TypeMetadataLookup,
    analysis: Option<&CompletionAnalysisContext<'_>>,
) -> Result<CompletionResult> {
    let trace_request_id = if completion_trace_enabled() {
        Some(next_completion_request_id())
    } else {
        None
    };
    let context = analyze_completion_context(file_content, line, column);
    let snapshot = index.snapshot();

    if let Some(request_id) = trace_request_id {
        info!(
            request_id = request_id,
            line = line,
            column = column,
            "Completion request"
        );
    } else {
        info!("Completion request: line {}, column {}", line, column);
    }

    let request_span = if let Some(request_id) = trace_request_id {
        tracing::debug_span!(
            "completion.request",
            request_id = request_id,
            line = line,
            column = column,
            member_access = context.member_access,
            trigger_char = ?context.trigger_char
        )
    } else {
        Span::none()
    };
    let _request_guard = request_span.enter();

    let mut candidates: Vec<Candidate> = Vec::new();

    let collect_span = if let Some(request_id) = trace_request_id {
        tracing::debug_span!("completion.collect", request_id = request_id)
    } else {
        Span::none()
    };
    let _collect_guard = collect_span.enter();
    let collect_started = if trace_request_id.is_some() {
        Some(Instant::now())
    } else {
        None
    };

    if context.member_access {
        if let Some(receiver_chain) = extract_member_receiver_chain(file_content, line, column) {
            if receiver_chain.len() == 1 {
                let base_name = receiver_chain[0].as_str();
                if let Some(type_name) = resolve_type_name(&snapshot, base_name, metadata_lookup) {
                    let resolution = TypeResolution::explicit(&type_name);
                    add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                    add_properties_from_resolution(
                        metadata_lookup,
                        &resolution,
                        &mut candidates,
                        1,
                    );
                } else if let Some(kind) = get_collection_kind(base_name) {
                    add_metadata_items(&snapshot, Some(kind), &mut candidates, 1);
                } else if let Some(resolution) = resolve_member_owner_type(
                    analysis,
                    file_content,
                    line,
                    column,
                    base_name,
                )
                .await
                {
                    add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                    add_properties_from_resolution(metadata_lookup, &resolution, &mut candidates, 1);
                }
            } else if let Some(resolution) = resolve_member_chain_owner_type(
                analysis,
                file_content,
                line,
                column,
                &receiver_chain,
                &snapshot,
                metadata_lookup,
            )
            .await
            {
                add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                add_properties_from_resolution(metadata_lookup, &resolution, &mut candidates, 1);
            }
        } else if let Some(base_name) = context.member_base.as_deref() {
            if let Some(type_name) = resolve_type_name(&snapshot, base_name, metadata_lookup) {
                let resolution = TypeResolution::explicit(&type_name);
                add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                add_properties_from_resolution(
                    metadata_lookup,
                    &resolution,
                    &mut candidates,
                    1,
                );
            } else if let Some(kind) = get_collection_kind(base_name) {
                add_metadata_items(&snapshot, Some(kind), &mut candidates, 1);
            } else if let Some(resolution) = resolve_member_owner_type(
                analysis,
                file_content,
                line,
                column,
                base_name,
            )
            .await
            {
                add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                add_properties_from_resolution(metadata_lookup, &resolution, &mut candidates, 1);
            }
        }

        if candidates.is_empty() {
            add_keywords(&snapshot, &mut candidates, 4);
        }
    } else {
        add_symbols(&snapshot, file_uri, &mut candidates, 0);
        add_module_symbols(&snapshot, &mut candidates, 1);
        add_metadata_items(&snapshot, None, &mut candidates, 2);
        add_types(&snapshot, &mut candidates, 3);
        add_keywords(&snapshot, &mut candidates, 4);
    }
    if let (Some(request_id), Some(started)) = (trace_request_id, collect_started) {
        debug!(
            request_id = request_id,
            stage = "collect",
            elapsed_ms = started.elapsed().as_millis(),
            candidates = candidates.len()
        );
    }
    drop(_collect_guard);

    let ranking_input: Vec<RankingCandidate> = candidates
        .into_iter()
        .map(|candidate| RankingCandidate {
            item: candidate.item,
            owner_type: candidate.owner_type,
            label_lower: candidate.label_lower,
            source_priority: candidate.source_priority,
            scope: candidate.scope,
        })
        .collect();

    let ranked = rank_candidates_with_trace(ranking_input, &context, trace_request_id);
    let is_incomplete = ranked.candidates.len() > COMPLETION_MAX_ITEMS;
    let limited = ranked.candidates.into_iter().take(COMPLETION_MAX_ITEMS);

    let format_span = if let Some(request_id) = trace_request_id {
        tracing::debug_span!("completion.format", request_id = request_id)
    } else {
        Span::none()
    };
    let _format_guard = format_span.enter();
    let format_started = if trace_request_id.is_some() {
        Some(Instant::now())
    } else {
        None
    };

    let items: Vec<CompletionCandidate> = limited
        .map(|candidate| CompletionCandidate {
            item: with_sort_text(
                candidate.item,
                candidate.score,
                candidate.source_priority,
                &candidate.label_lower,
            ),
            owner_type: candidate.owner_type,
            score: candidate.score,
            origin_sources: candidate.origin_sources,
        })
        .collect();

    if let (Some(request_id), Some(started)) = (trace_request_id, format_started) {
        debug!(
            request_id = request_id,
            stage = "format",
            elapsed_ms = started.elapsed().as_millis(),
            returned = items.len(),
            is_incomplete = is_incomplete
        );
    }
    drop(_format_guard);

    Ok(CompletionResult {
        items,
        is_incomplete,
        stats: CompletionStats {
            total_candidates: ranked.total_candidates,
            dedup_removed: ranked.dedup_removed,
            score_samples: ranked.score_samples,
            prefix_exact: ranked.summary.prefix_exact,
            prefix_starts: ranked.summary.prefix_starts,
            prefix_contains: ranked.summary.prefix_contains,
            prefix_none: ranked.summary.prefix_none,
            member_access: ranked.summary.member_access,
            has_owner: ranked.summary.has_owner,
        },
    })
}

/// Analyzes context for smart auto-completion
///
/// # Arguments
/// * `content` - File content
/// * `line` - Line number (0-based)
/// * `column` - Column number (UTF-16)
///
/// # Returns
/// CompletionContext with analysis results
pub fn analyze_completion_context(content: &str, line: u32, column: u32) -> CompletionContext {
    let lines: Vec<&str> = content.lines().collect();
    let line_index = line as usize;

    // Get current line and prefix
    let (_current_line, line_prefix) = if line_index < lines.len() {
        let line_content = lines[line_index];
        // Convert UTF-16 offset -> UTF-8 byte offset
        let column_index = utf16_to_byte_offset(line_content, column);
        (line_content, &line_content[..column_index])
    } else {
        ("", "")
    };

    let line_prefix = trim_to_window(line_prefix, CONTEXT_WINDOW_CHARS);
    let line_trimmed = line_prefix.trim_end();

    let trigger_char = line_trimmed.chars().last().filter(|ch| *ch == '.' || *ch == '(');
    let member_base = extract_member_base(line_trimmed);

    // Extract current word
    let current_word = extract_word_at_position(content, line, column).unwrap_or_default();

    CompletionContext {
        current_word,
        member_access: member_base.is_some(),
        member_base,
        trigger_char,
        can_add_statements: can_add_statements(line_trimmed),
        expects_type: expects_type_context(line_trimmed),
        can_add_functions: can_add_functions(line_trimmed),
    }
}

/// Checks if statements can be added at this position
fn can_add_statements(line_prefix: &str) -> bool {
    line_prefix.is_empty()
        || line_prefix.ends_with(';')
        || line_prefix.ends_with("Тогда")
        || line_prefix.ends_with("Иначе")
        || line_prefix.ends_with("КонецЕсли")
        || line_prefix.ends_with("КонецЦикла")
        || line_prefix.trim_start().is_empty()
}

/// Checks if a type is expected at this position
fn expects_type_context(line_prefix: &str) -> bool {
    line_prefix.contains(":")
        || line_prefix.contains("Тип(")
        || line_prefix.contains("ТипЗнч(")
        || line_prefix.contains("// ")
}

/// Checks if functions can be added at this position
fn can_add_functions(line_prefix: &str) -> bool {
    !line_prefix.contains("Процедура") && !line_prefix.contains("Функция")
}

fn add_keywords(snapshot: &IndexSnapshot, target: &mut Vec<Candidate>, priority: u8) {
    if snapshot.keyword_index.is_empty() {
        for keyword in DEFAULT_KEYWORDS {
            target.push(Candidate::new(
                CompletionItem::new((*keyword).to_string(), CompletionKind::Keyword),
                priority,
                None,
                None,
            ));
        }
        return;
    }

    for item in &snapshot.keyword_index {
        target.push(Candidate::new(
            CompletionItem::new(item.name.clone(), CompletionKind::Keyword),
            priority,
            None,
            None,
        ));
    }
}

fn add_types(snapshot: &IndexSnapshot, target: &mut Vec<Candidate>, priority: u8) {
    for item in snapshot.type_index.values() {
        if matches!(
            item.kind,
            IndexItemKind::Type(
                TypeKind::Platform
                    | TypeKind::Primitive
                    | TypeKind::Configuration
                    | TypeKind::Generic
                    | TypeKind::Faceted
            )
        ) {
            target.push(Candidate::new(
                CompletionItem::new(item.name.clone(), CompletionKind::Type),
                priority,
                None,
                None,
            ));
        }
    }
}

fn resolve_type_name(
    snapshot: &IndexSnapshot,
    name: &str,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<String> {
    let lowered = name.to_lowercase();
    let from_index = snapshot
        .type_index
        .values()
        .find(|item| item.name.to_lowercase() == lowered)
        .map(|item| item.name.clone());
    if from_index.is_some() {
        return from_index;
    }

    let resolution = TypeResolution::explicit(name);
    metadata_lookup
        .get_raw_type(&resolution)
        .map(|raw| raw.name)
}

fn extract_member_base(line_prefix: &str) -> Option<String> {
    let trimmed = line_prefix.trim_end();
    let dot_pos = trimmed.rfind('.')?;
    let before_dot = &trimmed[..dot_pos];
    let chars: Vec<char> = before_dot.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut end = chars.len();
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }
    if start == end {
        return None;
    }
    Some(chars[start..end].iter().collect())
}

fn extract_member_receiver_chain(
    content: &str,
    line: u32,
    column: u32,
) -> Option<Vec<String>> {
    let lines: Vec<&str> = content.lines().collect();
    let line_content = *lines.get(line as usize)?;
    let column_index = utf16_to_byte_offset(line_content, column);
    let line_prefix = trim_to_window(&line_content[..column_index], CONTEXT_WINDOW_CHARS);
    let trimmed = line_prefix.trim_end();
    let dot_pos = trimmed.rfind('.')?;
    let receiver_expr = trimmed[..dot_pos].trim_end();
    if receiver_expr.is_empty() {
        return None;
    }
    extract_identifier_chain_tail(receiver_expr)
}

fn extract_identifier_chain_tail(expr: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = expr.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut end = chars.len();
    let mut parts_rev: Vec<String> = Vec::new();

    loop {
        while end > 0 && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        if end == 0 {
            break;
        }

        let mut start = end;
        while start > 0 && is_identifier_char(chars[start - 1]) {
            start -= 1;
        }
        if start == end {
            return None;
        }
        parts_rev.push(chars[start..end].iter().collect());

        end = start;
        while end > 0 && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        if end == 0 {
            break;
        }
        if chars[end - 1] != '.' {
            break;
        }
        end -= 1;
    }

    if parts_rev.is_empty() {
        return None;
    }
    parts_rev.reverse();
    Some(parts_rev)
}

async fn resolve_member_chain_owner_type(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    receiver_chain: &[String],
    snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<TypeResolution> {
    if receiver_chain.is_empty() {
        return None;
    }

    let base_name = receiver_chain[0].as_str();
    let mut owner = if let Some(type_name) = resolve_type_name(snapshot, base_name, metadata_lookup) {
        TypeResolution::explicit(&type_name)
    } else {
        resolve_member_owner_type(analysis, file_content, line, column, base_name).await?
    };

    let resolver = analysis.map(|ctx| ctx.resolver);
    for member_name in receiver_chain.iter().skip(1) {
        if owner.is_unknown() {
            return None;
        }
        let member_lower = member_name.to_lowercase();

        if let Some(property) = metadata_lookup
            .get_properties(&owner)
            .into_iter()
            .find(|item| item.name.to_lowercase() == member_lower)
        {
            if property.prop_type.trim().is_empty() {
                return None;
            }
            owner = if let Some(resolver) = resolver {
                resolver.resolve_expression_sync(&property.prop_type)
            } else {
                TypeResolution::explicit(&property.prop_type)
            };
            continue;
        }

        if let Some(method) = metadata_lookup
            .get_methods(&owner)
            .into_iter()
            .find(|item| item.name.to_lowercase() == member_lower)
        {
            if method.return_type.trim().is_empty() {
                return None;
            }
            owner = if let Some(resolver) = resolver {
                resolver.resolve_expression_sync(&method.return_type)
            } else {
                TypeResolution::explicit(&method.return_type)
            };
            continue;
        }

        return None;
    }

    if owner.is_unknown() {
        None
    } else {
        Some(owner)
    }
}

fn trim_to_window(line_prefix: &str, window: usize) -> String {
    let mut chars: Vec<char> = line_prefix.chars().collect();
    if chars.len() > window {
        chars.drain(0..(chars.len() - window));
    }
    chars.into_iter().collect()
}

fn with_sort_text(
    mut item: CompletionItem,
    score: f32,
    source_priority: u8,
    label_lower: &str,
) -> CompletionItem {
    let score_rank = ((1.0 - score).clamp(0.0, 1.0) * 1000.0) as u32;
    item.sort_text = Some(format!("{:04}-{:02}-{}", score_rank, source_priority, label_lower));
    item
}

fn add_methods_from_resolution(
    metadata_lookup: &TypeMetadataLookup,
    resolution: &TypeResolution,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    let owner_type = resolution.type_name();
    let methods = metadata_lookup.get_methods(resolution);
    for method in methods {
        target.push(Candidate::new(
            CompletionItem::new(method.name, CompletionKind::Method),
            priority,
            Some(owner_type.clone()),
            None,
        ));
    }
}

fn add_properties_from_resolution(
    metadata_lookup: &TypeMetadataLookup,
    resolution: &TypeResolution,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    let owner_type = resolution.type_name();
    let properties = metadata_lookup.get_properties(resolution);
    for property in properties {
        target.push(Candidate::new(
            CompletionItem::new(property.name, CompletionKind::Property),
            priority,
            Some(owner_type.clone()),
            None,
        ));
    }
}

async fn resolve_member_owner_type(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    base_name: &str,
) -> Option<TypeResolution> {
    let ctx = analysis?;
    let ir_program = if let Some(program) = &ctx.ir_program {
        program.clone()
    } else {
        let cache_key = hash_content(&format!("{}\n{}", ctx.file_path, file_content));
        let ir_cache = ctx.ir_cache?;
        let parser = ctx.parser?;

        if let Some(cached) = ir_cache.get(cache_key).await {
            cached
        } else {
            let ir = match parser.parse_to_ir(file_content, ctx.file_path) {
                Ok(program) => program,
                Err(err) => {
                    warn!("Completion IR parse failed: {}", err);
                    return None;
                }
            };
            let ir_arc = Arc::new(ir);
            ir_cache.put(cache_key, ir_arc.clone()).await;
            ir_arc
        }
    };

    let scope_id = resolve_scope_for_member(&ir_program, line, column);
    if let Some(flow_type) =
        find_variable_type_at_position(&ir_program, base_name, scope_id, line)
    {
        return Some(flow_type);
    }

    let mut resolved =
        ctx.resolver.resolve_variable_with_context(base_name, &ir_program.symbols, scope_id);
    if resolved.is_unknown() {
        if let Some(best_effort) =
            find_variable_resolution_best_effort(&ir_program.symbols, base_name, line)
        {
            resolved = best_effort;
        }
    }

    Some(resolved)
}

fn resolve_scope_for_member(ir_program: &SemanticProgram, line: u32, column: u32) -> ScopeId {
    ir_program
        .find_node_at_position(line, column)
        .map(|node| node.scope_id)
        .or_else(|| {
            column
                .checked_sub(1)
                .and_then(|col| ir_program.find_node_at_position(line, col).map(|node| node.scope_id))
        })
        .or_else(|| find_scope_by_line(ir_program, line))
        .or_else(|| find_scope_before_position(ir_program, line, column))
        .unwrap_or(ir_program.symbols.root_scope)
}

fn find_scope_by_line(ir_program: &SemanticProgram, line: u32) -> Option<ScopeId> {
    ir_program
        .nodes
        .iter()
        .filter(|node| node.span.start_line <= line && line <= node.span.end_line)
        .min_by_key(|node| {
            let lines = node.span.end_line.saturating_sub(node.span.start_line);
            let cols = node.span.end_column.saturating_sub(node.span.start_column);
            (lines, cols)
        })
        .map(|node| node.scope_id)
}

fn find_scope_before_position(
    ir_program: &SemanticProgram,
    line: u32,
    column: u32,
) -> Option<ScopeId> {
    ir_program
        .nodes
        .iter()
        .filter(|node| {
            node.span.end_line < line
                || (node.span.end_line == line && node.span.end_column <= column)
        })
        .max_by_key(|node| (node.span.end_line, node.span.end_column))
        .map(|node| node.scope_id)
}

fn find_variable_resolution_best_effort(
    symbols: &bsl_shared::ir::SymbolTable,
    name: &str,
    line: u32,
) -> Option<TypeResolution> {
    let target = name.to_lowercase();
    let mut best: Option<(u32, TypeResolution)> = None;

    for scope in symbols.scopes.values() {
        for (var_name, state) in &scope.variables {
            if var_name.to_lowercase() != target {
                continue;
            }
            if state.declaration_span.start_line > line {
                continue;
            }
            let candidate_line = state.declaration_span.start_line;
            match &best {
                Some((best_line, _)) if *best_line > candidate_line => continue,
                _ => {
                    best = Some((candidate_line, state.resolution.clone()));
                }
            }
        }
    }

    best.map(|(_, resolution)| resolution)
}

fn add_symbols(
    snapshot: &IndexSnapshot,
    file_uri: Option<&str>,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    let Some(uri) = file_uri else {
        return;
    };
    let Some(items) = snapshot.symbol_index.get(uri) else {
        return;
    };

    for item in items {
        let kind = completion_kind_from_index_item(item);
        target.push(Candidate::new(
            CompletionItem::new(item.name.clone(), kind),
            priority,
            None,
            item.scope,
        ));
    }
}

fn add_module_symbols(snapshot: &IndexSnapshot, target: &mut Vec<Candidate>, priority: u8) {
    for items in snapshot.module_index.values() {
        for item in items {
            let kind = completion_kind_from_index_item(item);
            target.push(Candidate::new(
                CompletionItem::new(item.name.clone(), kind),
                priority,
                None,
                item.scope,
            ));
        }
    }
}

fn add_metadata_items(
    snapshot: &IndexSnapshot,
    kind: Option<bsl_shared::domain::types::MetadataKind>,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    match kind {
        Some(kind) => {
            if let Some(items) = snapshot.metadata_index.get(&kind) {
                for item in items {
                    target.push(Candidate::new(
                        CompletionItem::new(item.name.clone(), CompletionKind::Type),
                        priority,
                        None,
                        None,
                    ));
                }
            }
        }
        None => {
            for items in snapshot.metadata_index.values() {
                for item in items {
                    target.push(Candidate::new(
                        CompletionItem::new(item.name.clone(), CompletionKind::Type),
                        priority,
                        None,
                        None,
                    ));
                }
            }
        }
    }
}

fn completion_kind_from_index_item(item: &crate::system::IndexItem) -> CompletionKind {
    match &item.kind {
        IndexItemKind::Keyword => CompletionKind::Keyword,
        IndexItemKind::Type(_) => CompletionKind::Type,
        IndexItemKind::Metadata(kind) => match kind {
            bsl_shared::domain::types::MetadataKind::Catalog => CompletionKind::Catalog,
            bsl_shared::domain::types::MetadataKind::Document => CompletionKind::Document,
            bsl_shared::domain::types::MetadataKind::Enum => CompletionKind::Enum,
            _ => CompletionKind::Type,
        },
        IndexItemKind::Symbol(symbol) => match symbol {
            crate::system::SymbolKind::Function => CompletionKind::Function,
            crate::system::SymbolKind::Procedure => CompletionKind::Function,
            crate::system::SymbolKind::Method => CompletionKind::Method,
            crate::system::SymbolKind::Field => CompletionKind::Field,
            crate::system::SymbolKind::Variable => CompletionKind::Variable,
            crate::system::SymbolKind::Parameter => CompletionKind::Variable,
            crate::system::SymbolKind::Constant => CompletionKind::Constant,
            crate::system::SymbolKind::Module => CompletionKind::Module,
        },
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    item: CompletionItem,
    source_priority: u8,
    label_lower: String,
    owner_type: Option<String>,
    scope: Option<SymbolScope>,
}

impl Candidate {
    fn new(
        item: CompletionItem,
        source_priority: u8,
        owner_type: Option<String>,
        scope: Option<SymbolScope>,
    ) -> Self {
        let label_lower = item.label.to_lowercase();
        Self {
            item,
            source_priority,
            label_lower,
            owner_type,
            scope,
        }
    }
}

/// Context for auto-completion
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub current_word: String,
    pub member_access: bool,
    pub member_base: Option<String>,
    pub trigger_char: Option<char>,
    pub can_add_statements: bool,
    pub expects_type: bool,
    pub can_add_functions: bool,
}

pub fn resolve_type_details(
    type_name: &str,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<(Option<String>, Option<String>)> {
    let resolution = TypeResolution::explicit(type_name);
    let raw = metadata_lookup.get_raw_type(&resolution)?;

    let detail = if raw.category.is_empty() {
        None
    } else {
        Some(raw.category)
    };
    let documentation = if raw.description.is_empty() {
        None
    } else {
        Some(raw.description)
    };

    Some((detail, documentation))
}

#[derive(Debug, Clone)]
pub struct CompletionResolveDetails {
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

pub fn resolve_method_completion(
    owner_type: &str,
    method_name: &str,
    metadata_lookup: &TypeMetadataLookup,
    snippet_support: bool,
) -> Option<CompletionResolveDetails> {
    let resolution = TypeResolution::explicit(owner_type);
    let methods = metadata_lookup.get_methods(&resolution);
    let lowered = method_name.to_lowercase();
    let method = methods
        .into_iter()
        .find(|item| item.name.to_lowercase() == lowered)?;

    let detail = if method.return_type.is_empty() {
        None
    } else {
        Some(method.return_type.clone())
    };
    let documentation = method.description.clone();
    let insert_text = if snippet_support {
        build_method_snippet(&method)
    } else {
        None
    };

    Some(CompletionResolveDetails {
        detail,
        documentation,
        insert_text,
    })
}

pub(crate) fn build_call_snippet(name: &str, params: &[(String, bool)]) -> Option<String> {
    if params.is_empty() {
        return None;
    }

    let mut required: Vec<(String, bool)> = Vec::new();
    let mut optional: Vec<(String, bool)> = Vec::new();
    for (param_name, is_optional) in params {
        if *is_optional {
            optional.push((param_name.clone(), true));
        } else {
            required.push((param_name.clone(), false));
        }
    }

    let mut parts = Vec::with_capacity(params.len());
    let mut index = 1;
    for (param_name, is_optional) in required.into_iter().chain(optional) {
        let placeholder = if is_optional {
            format!("${{{}:}}", index)
        } else {
            let label = if param_name.is_empty() {
                format!("param{}", index)
            } else {
                param_name
            };
            format!("${{{}:{}}}", index, escape_snippet_text(&label))
        };
        parts.push(placeholder);
        index += 1;
    }

    let name = escape_snippet_text(name);
    Some(format!("{}({})$0", name, parts.join(", ")))
}

fn build_method_snippet(method: &bsl_shared::domain::types::RawMethodData) -> Option<String> {
    let mut params: Vec<(String, bool)> = Vec::with_capacity(method.params.len());
    for param in &method.params {
        params.push((param.name.clone(), param.is_optional));
    }
    build_call_snippet(&method.name, &params)
}

fn escape_snippet_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '$' => escaped.push_str("\\$"),
            '}' => escaped.push_str("\\}"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::IndexItem;
    use crate::system::IrCache;
    use std::sync::Arc;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::types::{RawDataSource, RawMethodData, RawPropertyData, RawTypeData};
    use crate::system::ParserCoordinator;

    #[test]
    fn trim_to_window_keeps_tail() {
        let input = "0123456789";
        let trimmed = trim_to_window(input, 4);
        assert_eq!(trimmed, "6789");
    }

    #[test]
    fn extract_member_base_simple() {
        let base = extract_member_base("Объект.").unwrap();
        assert_eq!(base, "Объект");
    }

    fn utf16_column(content: &str, marker: &str) -> (u32, u32) {
        let byte_index = content
            .find(marker)
            .unwrap_or_else(|| panic!("Marker not found: {}", marker));
        let before = &content[..byte_index];
        let line = before.lines().count().saturating_sub(1) as u32;
        let last_line = before.lines().last().unwrap_or("");
        let column = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        (line, column)
    }

    #[test]
    fn completion_context_detects_member_access_and_trigger_char() {
        let content = "Объект.";
        let (line, column) = utf16_column(content, ".");
        let ctx = analyze_completion_context(content, line, column + 1);

        assert!(ctx.member_access);
        assert_eq!(ctx.member_base.as_deref(), Some("Объект"));
        assert_eq!(ctx.trigger_char, Some('.'));
    }

    #[test]
    fn completion_context_detects_trigger_char_for_call() {
        let content = "Функция(";
        let (line, column) = utf16_column(content, "(");
        let ctx = analyze_completion_context(content, line, column + 1);

        assert!(!ctx.member_access);
        assert_eq!(ctx.trigger_char, Some('('));
    }

    #[test]
    fn completion_context_reads_current_word_with_utf16_column() {
        let content = "Перем a😀b";
        let (line, column) = utf16_column(content, "b");
        let ctx = analyze_completion_context(content, line, column + 1);

        assert_eq!(ctx.current_word, "b");
    }

    #[test]
    fn completion_context_can_add_statements_flags() {
        let content = "Если Истина Тогда";
        let ctx = analyze_completion_context(content, 0, content.len() as u32);
        assert!(ctx.can_add_statements);

        let content = "Перем Значение";
        let ctx = analyze_completion_context(content, 0, content.len() as u32);
        assert!(!ctx.can_add_statements);
    }

    #[test]
    fn completion_context_expects_type_flags() {
        let content = "Перем Значение: ";
        let ctx = analyze_completion_context(content, 0, content.len() as u32);
        assert!(ctx.expects_type);

        let content = "Тип(";
        let ctx = analyze_completion_context(content, 0, content.len() as u32);
        assert!(ctx.expects_type);
    }

    #[test]
    fn completion_context_can_add_functions_flags() {
        let content = "Процедура Тест()";
        let ctx = analyze_completion_context(content, 0, content.len() as u32);
        assert!(!ctx.can_add_functions);

        let content = "Функция Тест()";
        let ctx = analyze_completion_context(content, 0, content.len() as u32);
        assert!(!ctx.can_add_functions);

        let content = "Перем Значение";
        let ctx = analyze_completion_context(content, 0, content.len() as u32);
        assert!(ctx.can_add_functions);
    }

    #[tokio::test]
    async fn completion_filters_by_prefix() {
        let index = IntellisenseIndexStore::new("cfg", "platform");
        index.set_keywords(vec![
            IndexItem::new("Процедура", IndexItemKind::Keyword, crate::system::IndexKind::Keyword),
            IndexItem::new("Функция", IndexItemKind::Keyword, crate::system::IndexKind::Keyword),
        ]);
        index.upsert_type(IndexItem::new(
            "Массив",
            IndexItemKind::Type(TypeKind::Platform),
            crate::system::IndexKind::Type,
        ));

        let repository = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repository);

        let result = get_completion("Про", 0, 3, None, &index, &metadata_lookup)
            .await
            .expect("completion ok");
        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();

        assert_eq!(labels, vec!["Процедура".to_string()]);
    }

    #[test]
    fn build_call_snippet_includes_optional_placeholders() {
        let params = vec![
            ("Путь".to_string(), false),
            ("Режим".to_string(), true),
        ];
        let snippet = build_call_snippet("Открыть", &params).expect("snippet");
        assert_eq!(snippet, "Открыть(${1:Путь}, ${2:})$0");
    }

    #[test]
    fn build_call_snippet_escapes_special_chars() {
        let params = vec![("Имя}".to_string(), false)];
        let snippet = build_call_snippet("Функция$", &params).expect("snippet");
        assert_eq!(snippet, "Функция\\$(${1:Имя\\}})$0");
    }

    #[tokio::test]
    async fn completion_limits_output() {
        let index = IntellisenseIndexStore::new("cfg", "platform");
        let keywords = (0..300)
            .map(|i| {
                IndexItem::new(
                    format!("Ключ{}", i),
                    IndexItemKind::Keyword,
                    crate::system::IndexKind::Keyword,
                )
            })
            .collect();
        index.set_keywords(keywords);

        let repository = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repository);

        let result = get_completion("", 0, 0, None, &index, &metadata_lookup)
            .await
            .expect("completion ok");

        assert!(result.is_incomplete);
        assert_eq!(result.items.len(), COMPLETION_MAX_ITEMS);
    }

    #[tokio::test]
    async fn completion_resolves_variable_type_for_member_access() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![RawTypeData {
                name: "ТаблицаЗначений".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Добавить".to_string(),
                    return_type: "Булево".to_string(),
                    ..Default::default()
                }],
                properties: vec![RawPropertyData {
                    name: "Количество".to_string(),
                    prop_type: "Число".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            }])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> =
            repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let parser = ParserCoordinator::new_with_resolver(repo.clone(), resolver.clone());
        let ir_cache = IrCache::new(4);
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    ТаблЗнач = Новый ТаблицаЗначений;\n",
            "    ТаблЗнач.\n",
            "КонецПроцедуры\n"
        );
        let line = 2;
        let line_text = "    ТаблЗнач.";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let ctx = CompletionAnalysisContext {
            parser: Some(&parser),
            ir_cache: Some(&ir_cache),
            ir_program: None,
            resolver: resolver.as_ref(),
            file_path: "completion_test.bsl",
        };

        let resolved = resolve_member_owner_type(
            Some(&ctx),
            content,
            line,
            column,
            "ТаблЗнач",
        )
        .await
        .expect("member type");
        assert_eq!(resolved.type_name(), "ТаблицаЗначений");

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(
            labels.contains(&"Добавить".to_string()),
            "labels: {:?}",
            labels
        );
        assert!(
            labels.contains(&"Количество".to_string()),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_resolves_nested_member_access_chain() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "ТаблицаЗначений".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "Колонки".to_string(),
                        prop_type: "КоллекцияКолонокТаблицыЗначений".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "КоллекцияКолонокТаблицыЗначений".to_string(),
                    source: RawDataSource::Platform,
                    methods: vec![RawMethodData {
                        name: "Добавить".to_string(),
                        return_type: "КолонкаТаблицыЗначений".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let parser = ParserCoordinator::new_with_resolver(repo.clone(), resolver.clone());
        let ir_cache = IrCache::new(4);
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    ТаблЗнач = Новый ТаблицаЗначений;\n",
            "    ТаблЗнач.Колонки.\n",
            "КонецПроцедуры\n"
        );
        let line = 2;
        let line_text = "    ТаблЗнач.Колонки.";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let ctx = CompletionAnalysisContext {
            parser: Some(&parser),
            ir_cache: Some(&ir_cache),
            ir_program: None,
            resolver: resolver.as_ref(),
            file_path: "completion_nested_chain_test.bsl",
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_nested_chain_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(labels.contains(&"Добавить".to_string()), "labels: {:?}", labels);
    }
}
