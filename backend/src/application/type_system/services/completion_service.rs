//! Completion Service - auto-completion operations
//!
//! Functions for LSP completion requests and contextual auto-completion.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tracing::{debug, info, Span};

use bsl_shared::domain::metadata_constants::get_collection_kind;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{
    ConcreteType, FacetKind, MetadataKind, ResolutionResult, SpecialType,
};
use bsl_shared::domain::{CompletionItem, CompletionKind, TypeMetadataLookup, TypeResolution};
use bsl_shared::ir::{ScopeId, SemanticNodeKind, SemanticProgram};
use bsl_syntax::ast::Expression;

use super::super::extractors::symbol_extractor::{
    extract_word_at_position, is_identifier_char, utf16_to_byte_offset,
};
use super::completion_ranking::{rank_candidates_with_trace, RankingCandidate};
use super::completion_target::extract_completion_target_for_member_access;
use crate::system::keyword_index::DEFAULT_KEYWORDS;
use crate::system::{
    IndexItemKind, IndexSnapshot, IntellisenseIndexStore, LineIndex, SymbolScope, TypeKind,
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
    *COMPLETION_TRACE_ENABLED.get_or_init(|| std::env::var("BSL_COMPLETION_TRACE").is_ok())
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
    pub stage_snapshot_read: Duration,
    pub stage_collect: Duration,
    pub stage_rank: Duration,
    pub stage_format: Duration,
}

#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub items: Vec<CompletionCandidate>,
    pub is_incomplete: bool,
    pub stats: CompletionStats,
}

pub(crate) struct CompletionAnalysisContext<'a> {
    pub ir_program: Option<Arc<SemanticProgram>>,
    pub resolver: &'a TypeResolver,
    pub file_path: &'a str,
    pub parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    pub member_access_owner_type_hint: Option<TypeResolution>,
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

#[allow(clippy::too_many_arguments)]
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
    member_access_owner_type_hint: Option<TypeResolution>,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver,
        file_path,
        parse_result: None,
        member_access_owner_type_hint,
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

#[allow(clippy::too_many_arguments)]
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
    member_access_owner_type_hint: Option<TypeResolution>,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver,
        file_path,
        parse_result: None,
        member_access_owner_type_hint,
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

#[allow(clippy::too_many_arguments)]
pub async fn get_completion_with_semantic_program_snapshot_v2(
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index_snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
    file_path: &str,
    resolver: &TypeResolver,
    ir_program: Arc<SemanticProgram>,
    parse_result: Arc<bsl_syntax::ast::ParseResult>,
    member_access_owner_type_hint: Option<TypeResolution>,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver,
        file_path,
        parse_result: Some(parse_result),
        member_access_owner_type_hint,
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
    let analysis_file_path = analysis.map(|analysis| analysis.file_path);
    let context = analyze_completion_context(file_content, line, column);
    let snapshot_started = Instant::now();
    let snapshot = index.snapshot();
    let snapshot_elapsed = snapshot_started.elapsed();

    if let Some(request_id) = trace_request_id {
        info!(
            request_id = request_id,
            file_uri = ?file_uri,
            file_path = ?analysis_file_path,
            line = line,
            column = column,
            "Completion request"
        );
    } else {
        info!(
            file_uri = ?file_uri,
            file_path = ?analysis_file_path,
            line = line,
            column = column,
            "Completion request"
        );
    }

    let request_span = if let Some(request_id) = trace_request_id {
        if tracing::level_filters::STATIC_MAX_LEVEL >= tracing::level_filters::LevelFilter::DEBUG {
            tracing::debug_span!(
                "completion.request",
                request_id = request_id,
                file_uri = ?file_uri,
                file_path = ?analysis_file_path,
                line = line,
                column = column,
                member_access = context.member_access,
                trigger_char = ?context.trigger_char
            )
        } else {
            tracing::info_span!(
                "completion.request",
                request_id = request_id,
                file_uri = ?file_uri,
                file_path = ?analysis_file_path,
                line = line,
                column = column,
                member_access = context.member_access,
                trigger_char = ?context.trigger_char
            )
        }
    } else {
        Span::none()
    };
    let _request_guard = request_span.enter();

    let mut candidates: Vec<Candidate> = Vec::new();

    let collect_span = if let Some(request_id) = trace_request_id {
        if tracing::level_filters::STATIC_MAX_LEVEL >= tracing::level_filters::LevelFilter::DEBUG {
            tracing::debug_span!("completion.collect", request_id = request_id)
        } else {
            tracing::info_span!("completion.collect", request_id = request_id)
        }
    } else {
        Span::none()
    };
    let _collect_guard = collect_span.enter();
    let collect_started = Instant::now();

    if context.member_access {
        let receiver_types_from_ast = analysis
            .and_then(|analysis| {
                analysis
                    .parse_result
                    .as_ref()
                    .map(|parse_result| (analysis, parse_result))
            })
            .and_then(|(analysis, parse_result)| {
                extract_completion_target_for_member_access(
                    file_content,
                    line,
                    column,
                    parse_result,
                )
                .and_then(|target| {
                    if let Some(expr) = target.receiver_expression.as_ref() {
                        return Some(resolve_receiver_types_from_expression(
                            Some(analysis),
                            file_content,
                            line,
                            column,
                            expr,
                            &snapshot,
                            metadata_lookup,
                        ));
                    }

                    let mut out = Vec::new();
                    if let Some(exprs) = target.receiver_union_expressions.as_ref() {
                        for expr in exprs {
                            out.extend(resolve_receiver_types_from_expression(
                                Some(analysis),
                                file_content,
                                line,
                                column,
                                expr,
                                &snapshot,
                                metadata_lookup,
                            ));
                        }
                    }

                    (!out.is_empty()).then(|| dedup_resolutions(out))
                })
            })
            .filter(|types| !types.is_empty());

        if let Some(receiver_types) = receiver_types_from_ast {
            for owner in receiver_types {
                add_methods_from_resolution(metadata_lookup, &owner, &mut candidates, 0);
                add_properties_from_resolution(metadata_lookup, &owner, &mut candidates, 1);
            }
        } else if let Some(receiver_chain) =
            extract_member_receiver_chain(file_content, line, column)
        {
            if receiver_chain.len() == 1 {
                let base_name = receiver_chain[0].as_str();
                if let Some(kind) = get_collection_kind(base_name) {
                    add_metadata_items(&snapshot, Some(kind), &mut candidates, 1);
                } else if let Some(type_name) =
                    resolve_type_name(&snapshot, base_name, metadata_lookup)
                {
                    let resolution = analysis
                        .map(|ctx| ctx.resolver.resolve_expression_sync(&type_name))
                        .unwrap_or_else(|| TypeResolution::explicit(&type_name));
                    add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                    add_properties_from_resolution(
                        metadata_lookup,
                        &resolution,
                        &mut candidates,
                        1,
                    );
                } else if let Some(resolution) =
                    resolve_member_owner_type(analysis, file_content, line, column, base_name).await
                {
                    add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                    add_properties_from_resolution(
                        metadata_lookup,
                        &resolution,
                        &mut candidates,
                        1,
                    );
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
            if let Some(kind) = get_collection_kind(base_name) {
                add_metadata_items(&snapshot, Some(kind), &mut candidates, 1);
            } else if let Some(type_name) = resolve_type_name(&snapshot, base_name, metadata_lookup)
            {
                let resolution = TypeResolution::explicit(&type_name);
                add_methods_from_resolution(metadata_lookup, &resolution, &mut candidates, 0);
                add_properties_from_resolution(metadata_lookup, &resolution, &mut candidates, 1);
            } else if let Some(resolution) =
                resolve_member_owner_type(analysis, file_content, line, column, base_name).await
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
    let collect_elapsed = collect_started.elapsed();
    if let Some(request_id) = trace_request_id {
        if tracing::level_filters::STATIC_MAX_LEVEL >= tracing::level_filters::LevelFilter::DEBUG {
            debug!(
                request_id = request_id,
                stage = "collect",
                elapsed_ms = collect_elapsed.as_millis(),
                candidates = candidates.len()
            );
        } else {
            info!(
                request_id = request_id,
                stage = "collect",
                elapsed_ms = collect_elapsed.as_millis(),
                candidates = candidates.len()
            );
        }
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

    let rank_started = Instant::now();
    let ranked = rank_candidates_with_trace(ranking_input, &context, trace_request_id);
    let rank_elapsed = rank_started.elapsed();
    let is_incomplete = ranked.candidates.len() > COMPLETION_MAX_ITEMS;
    let limited = ranked.candidates.into_iter().take(COMPLETION_MAX_ITEMS);

    let format_span = if let Some(request_id) = trace_request_id {
        if tracing::level_filters::STATIC_MAX_LEVEL >= tracing::level_filters::LevelFilter::DEBUG {
            tracing::debug_span!("completion.format", request_id = request_id)
        } else {
            tracing::info_span!("completion.format", request_id = request_id)
        }
    } else {
        Span::none()
    };
    let _format_guard = format_span.enter();
    let format_started = Instant::now();

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

    let format_elapsed = format_started.elapsed();
    if let Some(request_id) = trace_request_id {
        if tracing::level_filters::STATIC_MAX_LEVEL >= tracing::level_filters::LevelFilter::DEBUG {
            debug!(
                request_id = request_id,
                stage = "format",
                elapsed_ms = format_elapsed.as_millis(),
                returned = items.len(),
                is_incomplete = is_incomplete
            );
        } else {
            info!(
                request_id = request_id,
                stage = "format",
                elapsed_ms = format_elapsed.as_millis(),
                returned = items.len(),
                is_incomplete = is_incomplete
            );
        }
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
            stage_snapshot_read: snapshot_elapsed,
            stage_collect: collect_elapsed,
            stage_rank: rank_elapsed,
            stage_format: format_elapsed,
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
    let (_current_line, line_prefix_raw) = if line_index < lines.len() {
        let line_content = lines[line_index];
        // Convert UTF-16 offset -> UTF-8 byte offset
        let column_index = utf16_to_byte_offset(line_content, column);
        (line_content, &line_content[..column_index])
    } else {
        ("", "")
    };

    let in_string_or_comment = is_in_string_or_comment(line_prefix_raw);

    let line_prefix = trim_to_window(line_prefix_raw, CONTEXT_WINDOW_CHARS);
    let line_trimmed = line_prefix.trim_end();

    let trigger_char = (!in_string_or_comment)
        .then(|| {
            line_trimmed
                .chars()
                .last()
                .filter(|ch| *ch == '.' || *ch == '(')
        })
        .flatten();
    let member_base = (!in_string_or_comment)
        .then(|| extract_member_base(line_trimmed))
        .flatten();
    let member_access = !in_string_or_comment && is_member_access_context(line_trimmed);

    // Extract current word
    let current_word = extract_word_at_position(content, line, column).unwrap_or_default();

    CompletionContext {
        current_word,
        member_access,
        member_base,
        trigger_char,
        can_add_statements: can_add_statements(line_trimmed),
        expects_type: expects_type_context(line_trimmed),
        can_add_functions: can_add_functions(line_trimmed),
    }
}

fn is_in_string_or_comment(line_prefix: &str) -> bool {
    let mut in_string = false;
    let mut chars = line_prefix.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_string = false;
                }
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            return true;
        }
    }
    in_string
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

    for item in snapshot.keyword_index.iter() {
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

fn is_member_access_context(line_prefix: &str) -> bool {
    let trimmed = line_prefix.trim_end();
    let Some(dot_pos) = trimmed.rfind('.') else {
        return false;
    };
    let after_dot = trimmed[dot_pos + 1..].trim_start();
    after_dot.is_empty() || after_dot.chars().all(is_identifier_char)
}

fn extract_member_receiver_chain(content: &str, line: u32, column: u32) -> Option<Vec<String>> {
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
    resolve_member_chain_owner_type_sync(
        analysis,
        file_content,
        line,
        column,
        receiver_chain,
        snapshot,
        metadata_lookup,
    )
}

fn resolve_member_chain_owner_type_sync(
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
    let mut start_index = 1usize;
    let mut owner = if let Some(kind) = get_collection_kind(base_name) {
        let object_name = receiver_chain.get(1)?;
        start_index = 2;
        let expr = format!("{}.{}", base_name, object_name);
        analysis
            .map(|ctx| ctx.resolver.resolve_expression_sync(&expr))
            .unwrap_or_else(|| {
                TypeResolution::metadata_type(kind, object_name, Some(FacetKind::Manager))
            })
    } else if let Some(type_name) = resolve_type_name(snapshot, base_name, metadata_lookup) {
        analysis
            .map(|ctx| ctx.resolver.resolve_expression_sync(&type_name))
            .unwrap_or_else(|| TypeResolution::explicit(&type_name))
    } else {
        resolve_member_owner_type_sync(analysis, file_content, line, column, base_name)?
    };

    let resolver = analysis.map(|ctx| ctx.resolver);
    for member_name in receiver_chain.iter().skip(start_index) {
        if owner.is_unknown() {
            return None;
        }

        if let Some(resolved) =
            resolve_property_access_type(resolver, metadata_lookup, &owner, member_name)
        {
            owner = resolved;
            continue;
        }

        if let Some(resolved) =
            resolve_method_call_return_type(resolver, metadata_lookup, &owner, member_name)
        {
            owner = resolved;
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
    item.sort_text = Some(format!(
        "{:04}-{:02}-{}",
        score_rank, source_priority, label_lower
    ));
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
    resolve_member_owner_type_sync(analysis, file_content, line, column, base_name)
}

fn resolve_member_owner_type_sync(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    base_name: &str,
) -> Option<TypeResolution> {
    let ctx = analysis?;
    if let Some(hint) = ctx.member_access_owner_type_hint.as_ref() {
        if !hint.is_unknown() && !hint.is_dynamic() {
            return Some(hint.clone());
        }
    }

    let ir_program = ctx.ir_program.as_deref()?;
    let index = LineIndex::new(file_content);
    let byte_offset = index.utf16_position_to_byte_offset(file_content, line, column);
    let byte_offset: u32 = byte_offset.try_into().ok()?;

    let scope_id = {
        let from_node = (0u32..=32)
            .filter_map(|delta| byte_offset.checked_sub(delta))
            .find_map(|offset| ir_program.find_node_at_byte_offset(offset))
            .map(|node| match &node.kind {
                SemanticNodeKind::FunctionDeclaration { body_scope, .. }
                | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => *body_scope,
                SemanticNodeKind::BlockScope { scope_id, .. } => *scope_id,
                _ => node.scope_id,
            });

        let from_enclosing_decl = || {
            ir_program
                .nodes
                .iter()
                .filter(|node| node.span.contains(byte_offset))
                .filter_map(|node| match &node.kind {
                    SemanticNodeKind::FunctionDeclaration { body_scope, .. }
                    | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => {
                        Some((node.span.len(), *body_scope))
                    }
                    _ => None,
                })
                .min_by_key(|(len, _)| *len)
                .map(|(_, scope_id)| scope_id)
        };

        let from_prev_node = || {
            ir_program
                .nodes
                .iter()
                .filter(|node| node.span.start < byte_offset)
                .max_by_key(|node| node.span.start)
                .map(|node| match &node.kind {
                    SemanticNodeKind::FunctionDeclaration { body_scope, .. }
                    | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => *body_scope,
                    SemanticNodeKind::BlockScope { scope_id, .. } => *scope_id,
                    _ => node.scope_id,
                })
        };

        from_node.or_else(from_enclosing_decl).or_else(from_prev_node)?
    };

    let mut visible_scopes = Vec::new();
    let mut current_scope_id = Some(scope_id);
    while let Some(sid) = current_scope_id {
        visible_scopes.push(sid);
        current_scope_id = ir_program.get_scope(sid).and_then(|scope| scope.parent);
    }

    let scope_rank: HashMap<ScopeId, usize> = visible_scopes
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, sid)| (sid, idx))
        .collect();

    #[derive(Debug)]
    struct BestInit {
        span_start: u32,
        scope_rank: usize,
        type_hint: Option<String>,
        initializer_node: Option<usize>,
    }

    let base_lower = base_name.to_lowercase();
    let mut best: Option<BestInit> = None;

    for node in ir_program.nodes.iter() {
        if node.span.start >= byte_offset {
            continue;
        }

        let Some(&rank) = scope_rank.get(&node.scope_id) else {
            continue;
        };

        let (type_hint, initializer_node) = match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                name,
                type_hint,
                initial_value_node,
                ..
            } if name.to_lowercase() == base_lower => {
                let hint = type_hint
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                (hint, *initial_value_node)
            }
            SemanticNodeKind::Assignment {
                variable,
                value_node,
            } if variable.to_lowercase() == base_lower => (None, *value_node),
            _ => continue,
        };

        let candidate = BestInit {
            span_start: node.span.start,
            scope_rank: rank,
            type_hint,
            initializer_node,
        };

        let replace = match &best {
            None => true,
            Some(best) => {
                candidate.span_start > best.span_start
                    || (candidate.span_start == best.span_start
                        && candidate.scope_rank < best.scope_rank)
            }
        };

        if replace {
            best = Some(candidate);
        }
    }

    let best = best?;
    if let Some(type_hint) = best.type_hint {
        let resolved = resolve_type_from_string(Some(ctx.resolver), &type_hint);
        return (!resolved.is_unknown()).then_some(resolved);
    }

    let init_index = best.initializer_node?;
    let init_node = ir_program.nodes.get(init_index)?;

    fn build_ir_expr(program: &SemanticProgram, node_index: usize, depth: u8) -> Option<String> {
        if depth == 0 {
            return None;
        }
        let node = program.nodes.get(node_index)?;
        match &node.kind {
            SemanticNodeKind::GlobalPropertyAccess { name } => Some(name.clone()),
            SemanticNodeKind::MemberAccess {
                object_node,
                object_name,
                member_name,
                ..
            } => {
                let base = if let Some(obj_node) = object_node {
                    build_ir_expr(program, *obj_node, depth - 1)?
                } else {
                    object_name.clone()?
                };
                Some(format!("{}.{}", base, member_name))
            }
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                object_node,
            } => {
                let base = if let Some(obj_node) = object_node {
                    Some(build_ir_expr(program, *obj_node, depth - 1)?)
                } else {
                    object_name.clone()
                };

                match base {
                    Some(base) => Some(format!("{}.{}()", base, function_name)),
                    None => Some(format!("{}()", function_name)),
                }
            }
            _ => None,
        }
    }

    match &init_node.kind {
        SemanticNodeKind::NewExpression { type_name, .. } => {
            let resolved = resolve_type_from_string(Some(ctx.resolver), type_name);
            (!resolved.is_unknown()).then_some(resolved)
        }
        SemanticNodeKind::MemberAccess { .. }
        | SemanticNodeKind::FunctionCall { .. }
        | SemanticNodeKind::GlobalPropertyAccess { .. } => {
            let expr = build_ir_expr(ir_program, init_index, 16)?;
            let resolved = ctx.resolver.resolve_expression_sync(&expr);
            (!resolved.is_unknown()).then_some(resolved)
        }
        _ => None,
    }
}

fn resolve_receiver_types_from_expression(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    expr: &Expression,
    snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
) -> Vec<TypeResolution> {
    match expr {
        Expression::Identifier { name, .. } => resolve_identifier_receiver_types(
            analysis,
            file_content,
            line,
            column,
            name,
            snapshot,
            metadata_lookup,
        ),
        Expression::New { type_name, .. } => vec![resolve_type_from_string(
            analysis.map(|ctx| ctx.resolver),
            type_name,
        )],
        Expression::Await { expression, .. } => resolve_receiver_types_from_expression(
            analysis,
            file_content,
            line,
            column,
            expression,
            snapshot,
            metadata_lookup,
        ),
        Expression::PropertyAccess {
            object, property, ..
        } => {
            if let Expression::Identifier { name, .. } = object.as_ref() {
                if let Some(kind) = get_collection_kind(name) {
                    let expr = format!("{}.{}", name, property);
                    let resolution = analysis
                        .map(|ctx| ctx.resolver.resolve_expression_sync(&expr))
                        .unwrap_or_else(|| {
                            TypeResolution::metadata_type(kind, property, Some(FacetKind::Manager))
                        });
                    return if !resolution.is_unknown() {
                        vec![resolution]
                    } else {
                        Vec::new()
                    };
                }
            }

            let owners = resolve_receiver_types_from_expression(
                analysis,
                file_content,
                line,
                column,
                object,
                snapshot,
                metadata_lookup,
            );
            let mut out = Vec::new();
            for owner in owners {
                if let Some(resolved) = resolve_property_access_type(
                    analysis.map(|ctx| ctx.resolver),
                    metadata_lookup,
                    &owner,
                    property,
                ) {
                    out.push(resolved);
                }
            }
            dedup_resolutions(out)
        }
        Expression::Call { function, .. } => match function.as_ref() {
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let owners = resolve_receiver_types_from_expression(
                    analysis,
                    file_content,
                    line,
                    column,
                    object,
                    snapshot,
                    metadata_lookup,
                );
                let mut out = Vec::new();
                for owner in owners {
                    if let Some(resolved) = resolve_method_call_return_type(
                        analysis.map(|ctx| ctx.resolver),
                        metadata_lookup,
                        &owner,
                        property,
                    ) {
                        out.push(resolved);
                    }
                }
                dedup_resolutions(out)
            }
            Expression::Identifier { name, .. } => resolve_global_function_return_types(
                analysis.map(|ctx| ctx.resolver),
                metadata_lookup,
                name,
            ),
            _ => Vec::new(),
        },
        Expression::IndexAccess { object, .. } => {
            let owners = resolve_receiver_types_from_expression(
                analysis,
                file_content,
                line,
                column,
                object,
                snapshot,
                metadata_lookup,
            );
            let mut out = Vec::new();
            for owner in owners {
                if let Some(resolved) = resolve_index_access_element_type(
                    analysis.map(|ctx| ctx.resolver),
                    metadata_lookup,
                    &owner,
                ) {
                    out.push(resolved);
                }
            }
            dedup_resolutions(out)
        }
        Expression::Ternary {
            then_expr,
            else_expr,
            ..
        } => {
            let mut out = resolve_receiver_types_from_expression(
                analysis,
                file_content,
                line,
                column,
                then_expr,
                snapshot,
                metadata_lookup,
            );
            out.extend(resolve_receiver_types_from_expression(
                analysis,
                file_content,
                line,
                column,
                else_expr,
                snapshot,
                metadata_lookup,
            ));
            dedup_resolutions(out)
        }
        _ => Vec::new(),
    }
}

fn resolve_identifier_receiver_types(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    name: &str,
    snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
) -> Vec<TypeResolution> {
    if get_collection_kind(name).is_some() {
        return Vec::new();
    }

    if let Some(type_name) = resolve_type_name(snapshot, name, metadata_lookup) {
        return vec![resolve_type_from_string(
            analysis.map(|ctx| ctx.resolver),
            &type_name,
        )];
    }

    resolve_member_owner_type_sync(analysis, file_content, line, column, name)
        .filter(|resolution| !resolution.is_unknown())
        .into_iter()
        .collect()
}

fn resolve_property_access_type(
    resolver: Option<&TypeResolver>,
    metadata_lookup: &TypeMetadataLookup,
    owner: &TypeResolution,
    property_name: &str,
) -> Option<TypeResolution> {
    let owner_type_name = owner.type_name();
    let lowered = property_name.to_lowercase();
    let property = metadata_lookup
        .get_properties(owner)
        .into_iter()
        .find(|item| item.name.to_lowercase() == lowered)?;
    if property.prop_type.trim().is_empty() {
        return None;
    }

    if let Some(resolver) = resolver {
        if property
            .prop_type
            .trim_start()
            .starts_with("ТабличнаяЧасть<")
        {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &owner.result {
                let tabular_sections = metadata_lookup.get_tabular_sections(owner);
                let lowered = property_name.to_lowercase();
                if let Some(ts) = tabular_sections
                    .iter()
                    .find(|ts| ts.name.to_lowercase() == lowered)
                {
                    let parent_type = if cfg.name.contains('.') {
                        cfg.name.clone()
                    } else {
                        format!("{}.{}", cfg.kind.to_prefix(), cfg.name)
                    };
                    let expr = format!("{}.{}", parent_type, ts.name);
                    let resolved = resolver.resolve_expression_sync(&expr);
                    if !resolved.is_unknown() {
                        return Some(resolved);
                    }
                }
            }
        }
    }

    let resolved_type = substitute_type_name_if_needed(&property.prop_type, &owner_type_name);
    Some(resolve_type_from_string(resolver, &resolved_type))
}

fn resolve_method_call_return_type(
    resolver: Option<&TypeResolver>,
    metadata_lookup: &TypeMetadataLookup,
    owner: &TypeResolution,
    method_name: &str,
) -> Option<TypeResolution> {
    let owner_type_name = owner.type_name();

    if matches!(owner.result, ResolutionResult::Generic(_)) {
        if let ResolutionResult::Generic(generic) = &owner.result {
            let base = generic.base_type.to_lowercase();
            let method = method_name.to_lowercase();
            if base == "табличнаячасть"
                && matches!(
                    method.as_str(),
                    "добавить" | "вставить" | "получить" | "найти"
                )
            {
                if let Some(concrete) = generic.type_params.first() {
                    if !matches!(concrete, ConcreteType::Special(SpecialType::Undefined)) {
                        if let Some(resolver) = resolver {
                            return Some(resolve_concrete_type(resolver, concrete));
                        }
                    }
                }
            }
        }

        let lowered = method_name.to_lowercase();
        let method = metadata_lookup
            .get_methods(owner)
            .into_iter()
            .find(|item| item.name.to_lowercase() == lowered)?;
        if method.return_type.trim().is_empty() {
            return None;
        }
        let resolved_type = substitute_type_name_if_needed(&method.return_type, &owner_type_name);
        return Some(resolve_type_from_string(resolver, &resolved_type));
    }

    let signature = metadata_lookup.find_method_signature_for_call(Some(owner), method_name);
    if let Some(signature) = signature {
        let return_type = signature.return_type.as_deref().unwrap_or("Неопределено");

        if return_type == "T" {
            if let ResolutionResult::Generic(generic) = &owner.result {
                if let Some(concrete) = generic.type_params.first() {
                    if !matches!(concrete, ConcreteType::Special(SpecialType::Undefined)) {
                        if let Some(resolver) = resolver {
                            return Some(resolve_concrete_type(resolver, concrete));
                        }
                    }
                }
            }
        }

        let resolved_type = substitute_type_name_if_needed(return_type, &owner_type_name);
        return Some(resolve_type_from_string(resolver, &resolved_type));
    }

    let lowered = method_name.to_lowercase();
    let method = metadata_lookup
        .get_methods(owner)
        .into_iter()
        .find(|item| item.name.to_lowercase() == lowered)?;
    if method.return_type.trim().is_empty() {
        return None;
    }

    let resolved_type = substitute_type_name_if_needed(&method.return_type, &owner_type_name);
    Some(resolve_type_from_string(resolver, &resolved_type))
}

fn resolve_global_function_return_types(
    resolver: Option<&TypeResolver>,
    metadata_lookup: &TypeMetadataLookup,
    function_name: &str,
) -> Vec<TypeResolution> {
    let signature = metadata_lookup.find_method_signature_for_call(None, function_name);
    let Some(signature) = signature else {
        return Vec::new();
    };

    let return_type = signature.return_type.as_deref().unwrap_or("Неопределено");
    vec![resolve_type_from_string(resolver, return_type)]
}

fn resolve_index_access_element_type(
    resolver: Option<&TypeResolver>,
    metadata_lookup: &TypeMetadataLookup,
    owner: &TypeResolution,
) -> Option<TypeResolution> {
    let resolver = resolver?;

    match &owner.result {
        ResolutionResult::Generic(generic) => {
            let base = generic.base_type.to_lowercase();
            let candidate = if base == "соответствие" || base == "map" {
                generic
                    .type_params
                    .get(1)
                    .or_else(|| generic.type_params.first())
            } else {
                generic.type_params.first()
            };

            if let Some(candidate) = candidate {
                if !matches!(candidate, ConcreteType::Special(SpecialType::Undefined)) {
                    return Some(resolve_concrete_type(resolver, candidate));
                }
            }

            let raw = metadata_lookup.get_raw_type(owner)?;
            let item_type = raw.collection_item_type.as_deref()?;
            if item_type.trim().is_empty() {
                return None;
            }
            let substituted = substitute_type_name_if_needed(item_type, &owner.type_name());
            Some(resolver.resolve_expression_sync(&substituted))
        }
        _ => {
            let raw = metadata_lookup.get_raw_type(owner)?;
            let item_type = raw.collection_item_type.as_deref()?;
            if item_type.trim().is_empty() {
                return None;
            }
            let substituted = substitute_type_name_if_needed(item_type, &owner.type_name());
            Some(resolver.resolve_expression_sync(&substituted))
        }
    }
}

fn resolve_concrete_type(resolver: &TypeResolver, concrete: &ConcreteType) -> TypeResolution {
    let type_name = match concrete {
        ConcreteType::Primitive(pt) => pt.display_name().to_string(),
        ConcreteType::Platform(pt) => pt.name.clone(),
        ConcreteType::Special(s) => s.display_name().to_string(),
        ConcreteType::GlobalFunction(func) => func.name.clone(),
        ConcreteType::TabularRow(row) => row.get_full_name(),
        ConcreteType::Configuration(cfg) => {
            if let Some(facet) = cfg.facet {
                format!("{}.{}", cfg.kind.faceted_type_prefix(&facet), cfg.name)
            } else {
                format!("{}.{}", cfg.kind.to_prefix(), cfg.name)
            }
        }
    };
    resolver.resolve_expression_sync(&type_name)
}

fn substitute_type_name_if_needed(type_name: &str, owner_type: &str) -> String {
    let Some(metadata_name) = SignatureIndex::extract_metadata_name(owner_type) else {
        return type_name.to_string();
    };
    SignatureIndex::substitute_type_name(type_name, metadata_name)
}

fn resolve_type_from_string(resolver: Option<&TypeResolver>, type_name: &str) -> TypeResolution {
    let type_name = type_name.trim();
    if type_name.is_empty() {
        return TypeResolution::unknown();
    }
    resolver
        .map(|resolver| resolver.resolve_expression_sync(type_name))
        .unwrap_or_else(|| TypeResolution::explicit(type_name))
}

fn dedup_resolutions(resolutions: Vec<TypeResolution>) -> Vec<TypeResolution> {
    let mut seen = std::collections::HashSet::<String>::new();
    let mut out = Vec::new();
    for resolution in resolutions {
        if resolution.is_unknown() {
            continue;
        }
        let key = resolution.type_name();
        if seen.insert(key) {
            out.push(resolution);
        }
    }
    out
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

    for item in items.iter() {
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
        for item in items.iter() {
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
    kind: Option<MetadataKind>,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    fn format_metadata_detail(kind: MetadataKind, facets: &[FacetKind]) -> String {
        if facets.is_empty() {
            return kind.to_russian_name().to_string();
        }
        let facets = facets
            .iter()
            .map(|facet| facet.display_name())
            .collect::<Vec<_>>()
            .join(", ");
        format!("{} ({})", kind.to_russian_name(), facets)
    }

    match kind {
        Some(kind) => {
            if let Some(items) = snapshot.metadata_index.get(&kind) {
                for item in items.iter() {
                    let item_kind = completion_kind_from_index_item(item);
                    let detail = match item.kind {
                        IndexItemKind::Metadata(kind) => {
                            Some(format_metadata_detail(kind, &item.facets))
                        }
                        _ => None,
                    };
                    target.push(Candidate::new(
                        CompletionItem::with_details(item.name.clone(), item_kind, detail, None),
                        priority,
                        None,
                        None,
                    ));
                }
            }
        }
        None => {
            for items in snapshot.metadata_index.values() {
                for item in items.iter() {
                    let item_kind = completion_kind_from_index_item(item);
                    let detail = match item.kind {
                        IndexItemKind::Metadata(kind) => {
                            Some(format_metadata_detail(kind, &item.facets))
                        }
                        _ => None,
                    };
                    target.push(Candidate::new(
                        CompletionItem::with_details(item.name.clone(), item_kind, detail, None),
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
        IndexItemKind::Metadata(kind) => CompletionKind::from_metadata_kind(*kind),
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

pub fn build_call_snippet(name: &str, params: &[(String, bool)]) -> Option<String> {
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
    use bsl_analysis_v2::{
        AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
    };
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::signature_index::{
        ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
    };
    use bsl_shared::domain::type_id::TypeId;
    use bsl_shared::domain::types::{RawDataSource, RawMethodData, RawPropertyData, RawTypeData};
    use bsl_shared::formatting::DetailLevel;
    use std::sync::Arc;

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
            IndexItem::new(
                "Процедура",
                IndexItemKind::Keyword,
                crate::system::IndexKind::Keyword,
            ),
            IndexItem::new(
                "Функция",
                IndexItemKind::Keyword,
                crate::system::IndexKind::Keyword,
            ),
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
        let params = vec![("Путь".to_string(), false), ("Режим".to_string(), true)];
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

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
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

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_test.bsl",
            parse_result: None,
            member_access_owner_type_hint: None,
        };

        let resolved = resolve_member_owner_type(Some(&ctx), content, line, column, "ТаблЗнач")
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

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_nested_chain_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_nested_chain_test.bsl",
            parse_result: None,
            member_access_owner_type_hint: None,
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
        assert!(
            labels.contains(&"Добавить".to_string()),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_supports_member_access_after_method_call() {
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
                RawTypeData {
                    name: "КолонкаТаблицыЗначений".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "Имя".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    ТаблЗнач = Новый ТаблицаЗначений;\n",
            "    ТаблЗнач.Колонки.Добавить().\n",
            "КонецПроцедуры\n"
        );
        let line = 2;
        let line_text = "    ТаблЗнач.Колонки.Добавить().";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_call_chain_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
        let parse_result = analysis
            .parse_result(V2FileId(1))
            .ok()
            .flatten()
            .expect("parse_result");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_call_chain_test.bsl",
            parse_result: Some(parse_result),
            member_access_owner_type_hint: None,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_call_chain_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(labels.contains(&"Имя".to_string()), "labels: {:?}", labels);
    }

    #[tokio::test]
    async fn completion_supports_member_access_after_index_access() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "Массив".to_string(),
                    source: RawDataSource::Platform,
                    collection_item_type: Some("КолонкаТаблицыЗначений".to_string()),
                    ..Default::default()
                },
                RawTypeData {
                    name: "КолонкаТаблицыЗначений".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "Имя".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    Перем arr;\n",
            "    arr = Новый Массив;\n",
            "    arr[0].\n",
            "КонецПроцедуры\n"
        );
        let line = 3;
        let line_text = "    arr[0].";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_index_access_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
        let parse_result = analysis
            .parse_result(V2FileId(1))
            .ok()
            .flatten()
            .expect("parse_result");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_index_access_test.bsl",
            parse_result: Some(parse_result),
            member_access_owner_type_hint: None,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_index_access_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(labels.contains(&"Имя".to_string()), "labels: {:?}", labels);
    }

    #[tokio::test]
    async fn completion_supports_member_access_after_map_index_access() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "Соответствие".to_string(),
                    source: RawDataSource::Platform,
                    collection_item_type: Some("КолонкаТаблицыЗначений".to_string()),
                    ..Default::default()
                },
                RawTypeData {
                    name: "КолонкаТаблицыЗначений".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "Имя".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    Перем map;\n",
            "    map = Новый Соответствие;\n",
            "    map[\"k\"].\n",
            "КонецПроцедуры\n"
        );
        let line = 3;
        let line_text = "    map[\"k\"].";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_map_index_access_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
        let parse_result = analysis
            .parse_result(V2FileId(1))
            .ok()
            .flatten()
            .expect("parse_result");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_map_index_access_test.bsl",
            parse_result: Some(parse_result),
            member_access_owner_type_hint: None,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_map_index_access_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(labels.contains(&"Имя".to_string()), "labels: {:?}", labels);
    }

    #[tokio::test]
    async fn completion_supports_member_access_after_ternary_expression() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "TypeA".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "PropA".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "TypeB".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "PropB".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    ?(Истина, Новый TypeA, Новый TypeB).\n",
            "КонецПроцедуры\n"
        );
        let line = 1;
        let line_text = "    ?(Истина, Новый TypeA, Новый TypeB).";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_ternary_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
        let parse_result = analysis
            .parse_result(V2FileId(1))
            .ok()
            .flatten()
            .expect("parse_result");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_ternary_test.bsl",
            parse_result: Some(parse_result),
            member_access_owner_type_hint: None,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_ternary_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(
            labels.contains(&"PropA".to_string()),
            "labels: {:?}",
            labels
        );
        assert!(
            labels.contains(&"PropB".to_string()),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_supports_member_access_after_choice_expression() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "TypeA".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "PropA".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "TypeB".to_string(),
                    source: RawDataSource::Platform,
                    properties: vec![RawPropertyData {
                        name: "PropB".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: true,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    Выбор\n",
            "        Когда Истина Тогда Новый TypeA\n",
            "        Иначе Новый TypeB\n",
            "    Конец.\n",
            "КонецПроцедуры\n"
        );
        let line = 4;
        let line_text = "    Конец.";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_choice_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
        let parse_result = analysis
            .parse_result(V2FileId(1))
            .ok()
            .flatten()
            .expect("parse_result");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_choice_test.bsl",
            parse_result: Some(parse_result),
            member_access_owner_type_hint: None,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_choice_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(
            labels.contains(&"PropA".to_string()),
            "labels: {:?}",
            labels
        );
        assert!(
            labels.contains(&"PropB".to_string()),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_substitutes_faceted_metadata_name_in_return_type() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![RawTypeData {
                name: "Справочники.Контрагенты".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "Наименование".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            }])
            .expect("load types");

        let mut signatures = SignatureIndex::new();
        signatures.add_platform_method(
            TypeId::new("СправочникМенеджер"),
            MethodSignature::new(
                "СоздатьЭлемент".to_string(),
                Some("СправочникМенеджер".to_string()),
                vec![],
                Some("СправочникОбъект".to_string()),
                None,
                None,
                SignatureSource::Platform,
                None,
                ContextRequirements::default(),
            ),
        );
        repository.set_signature_index(signatures);

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let index = IntellisenseIndexStore::new("cfg", "platform");
        let content = concat!(
            "Процедура Тест()\n",
            "    Manager = Справочники.Контрагенты;\n",
            "    Manager.СоздатьЭлемент().\n",
            "КонецПроцедуры\n"
        );
        let line = 2;
        let line_text = "    Manager.СоздатьЭлемент().";
        let column = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo.clone(),
            platform_signatures_loaded: false,
        });
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps,
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("completion_facet_substitution_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
        let parse_result = analysis
            .parse_result(V2FileId(1))
            .ok()
            .flatten()
            .expect("parse_result");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_facet_substitution_test.bsl",
            parse_result: Some(parse_result),
            member_access_owner_type_hint: None,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_facet_substitution_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(
            labels.contains(&"Наименование".to_string()),
            "labels: {:?}",
            labels
        );
    }
}
