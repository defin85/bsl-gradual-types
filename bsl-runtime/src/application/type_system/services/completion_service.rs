//! Completion Service - auto-completion operations
//!
//! Functions for LSP completion requests and contextual auto-completion.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, Span};

use bsl_shared::domain::code_location::{CodeLocation, ModuleType};
use bsl_shared::domain::metadata_constants::get_collection_kind;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{
    ConcreteType, ContextualTypeDescriptor, FacetKind, MetadataKind, ResolutionResult, SpecialType,
};
use bsl_shared::domain::{CompletionItem, CompletionKind, TypeMetadataLookup, TypeResolution};
use bsl_shared::ir::{ScopeId, ScopeKind, SemanticNodeKind, SemanticProgram};
use bsl_syntax::ast::Expression;

use super::super::extractors::symbol_extractor::{
    extract_word_at_position, is_identifier_char, utf16_to_byte_offset,
};
use super::completion_ranking::{rank_candidates_with_trace, RankingCandidate};
use super::completion_target::extract_completion_target_for_member_access;
use super::flow_sensitive::narrow_type_for_variable_at;
use crate::system::keyword_index::DEFAULT_KEYWORDS;
use crate::system::{
    IndexItemKind, IndexSnapshot, IntellisenseIndexStore, LineIndex, SymbolKind, SymbolScope,
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
static COMPLETION_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn completion_trace_enabled() -> bool {
    crate::system::runtime_config::global_runtime_config()
        .get_bool(crate::system::runtime_config::RuntimeKey::CompletionTrace)
        .unwrap_or(false)
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
    pub include_flow_sensitive: bool,
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
        include_flow_sensitive: false,
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
    include_flow_sensitive: bool,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver,
        file_path,
        parse_result: None,
        member_access_owner_type_hint,
        include_flow_sensitive,
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
    include_flow_sensitive: bool,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver,
        file_path,
        parse_result: Some(parse_result),
        member_access_owner_type_hint,
        include_flow_sensitive,
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
        let can_collect_locals_from_ir = analysis.and_then(|ctx| ctx.ir_program.as_ref()).is_some();

        if can_collect_locals_from_ir {
            add_local_symbols_from_ir(analysis, file_content, line, column, &mut candidates, 0);
            add_symbols(&snapshot, file_uri, &mut candidates, 0, false);
        } else {
            // In fallback mode keep local candidates from the file-bound index.
            add_symbols(&snapshot, file_uri, &mut candidates, 0, true);
        }
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
    let (_current_line, line_prefix_raw, cursor_char) = if line_index < lines.len() {
        let line_content = lines[line_index];
        // Convert UTF-16 offset -> UTF-8 byte offset
        let column_index = utf16_to_byte_offset(line_content, column);
        let line_prefix_raw = line_content.get(..column_index).unwrap_or(line_content);
        let cursor_char = line_content
            .get(column_index..)
            .and_then(|tail| tail.chars().next());
        (line_content, line_prefix_raw, cursor_char)
    } else {
        ("", "", None)
    };

    let in_string_or_comment = is_in_string_or_comment(line_prefix_raw);

    // Some clients request completion with cursor positioned on '.' itself.
    // Treat this as member-access context to avoid falling back to keyword completion.
    let effective_prefix_raw = if !in_string_or_comment {
        if let Some(cursor_char) = cursor_char.filter(|ch| *ch == '.' || *ch == '(') {
            format!("{line_prefix_raw}{cursor_char}")
        } else {
            line_prefix_raw.to_string()
        }
    } else {
        line_prefix_raw.to_string()
    };

    let line_prefix = trim_to_window(&effective_prefix_raw, CONTEXT_WINDOW_CHARS);
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
    let mut current_word = extract_word_at_position(content, line, column).unwrap_or_default();
    if member_access && line_trimmed.ends_with('.') {
        current_word.clear();
    }

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
    // Primary key is alphabetical label for predictable UX in editors.
    // Keep original case/source/score as stable tie-breakers for deterministic ordering.
    item.sort_text = Some(format!(
        "{}-{}-{:02}-{:04}",
        label_lower,
        item.label.as_str(),
        source_priority,
        score_rank
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
    let properties = metadata_lookup.get_properties_with_origin(resolution);
    let mut intrinsic_count = 0usize;
    let mut saw_intrinsic = false;
    for (property, origin) in properties {
        let property_priority = if TypeMetadataLookup::is_intrinsic_property_origin(origin) {
            intrinsic_count += 1;
            saw_intrinsic = true;
            // Для form-data pipeline сохраняем provider order в source_priority:
            // shape/repository-before-intrinsic -> intrinsic -> facet/fallback-repository.
            priority.saturating_add(1)
        } else if saw_intrinsic {
            priority.saturating_add(2)
        } else {
            priority
        };

        target.push(Candidate::new(
            CompletionItem::new(property.name, CompletionKind::Property),
            property_priority,
            Some(owner_type.clone()),
            None,
        ));
    }

    if intrinsic_count > 0 {
        tracing::debug!(
            metric = "completion_form_data_intrinsic_candidates_total",
            owner_type = owner_type,
            count = intrinsic_count,
            "Added intrinsic form-data property candidates"
        );
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
    let scope_position = resolve_completion_scope_position(ir_program, file_content, line, column)?;
    let byte_offset = scope_position.byte_offset;
    let scope_rank = &scope_position.scope_rank;

    if let Some(resolution) = resolve_implicit_member_owner_type_from_module_context(
        ctx,
        ir_program,
        &scope_position,
        base_name,
    ) {
        return Some(resolution);
    }

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
                ..
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
    let mut resolved: Option<TypeResolution> = None;

    if let Some(type_hint) = best.type_hint {
        let from_hint = resolve_type_from_string(Some(ctx.resolver), &type_hint);
        if !from_hint.is_unknown() {
            resolved = Some(from_hint);
        }
    } else if let Some(init_index) = best.initializer_node {
        let init_node = ir_program.nodes.get(init_index)?;

        fn build_ir_expr(
            program: &SemanticProgram,
            node_index: usize,
            depth: u8,
        ) -> Option<String> {
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

        let from_init = match &init_node.kind {
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
        };

        resolved = from_init;
    }

    let base = resolved.clone().unwrap_or_else(TypeResolution::unknown);
    if analysis.is_some_and(|ctx| ctx.include_flow_sensitive) {
        if let Some(narrowed) =
            narrow_type_for_variable_at(ir_program, byte_offset, base_name, base)
        {
            resolved = Some(narrowed);
        }
    }

    resolved.filter(|t| !t.is_unknown())
}

fn parse_owner_kind(owner_type: &str) -> Option<(MetadataKind, &str)> {
    let (xml_kind, object_name) = owner_type.split_once('.')?;
    let kind = MetadataKind::from_xml_tag(xml_kind)?;
    Some((kind, object_name))
}

fn resolve_type_from_contextual_descriptor(
    resolver: Option<&TypeResolver>,
    descriptor: &ContextualTypeDescriptor,
) -> TypeResolution {
    match descriptor {
        ContextualTypeDescriptor::PlatformType { type_name } => {
            resolve_type_from_string(resolver, type_name)
        }
        ContextualTypeDescriptor::ConfigurationFacet { kind, name, facet } => {
            TypeResolution::metadata_type(*kind, name, Some(*facet))
        }
        ContextualTypeDescriptor::FormType { .. }
        | ContextualTypeDescriptor::FormElementsType { .. } => {
            resolve_type_from_string(resolver, &descriptor.canonical_type_name())
        }
        ContextualTypeDescriptor::FormDataObject {
            kind, owner_name, ..
        } => {
            let mut resolution = TypeResolution::metadata_type(*kind, owner_name, None);
            for note in descriptor.resolution_metadata_notes() {
                if !resolution.metadata.notes.contains(&note) {
                    resolution.metadata.notes.push(note);
                }
            }
            resolution
        }
    }
}

fn resolve_implicit_member_owner_type_from_module_context(
    ctx: &CompletionAnalysisContext<'_>,
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
    base_name: &str,
) -> Option<TypeResolution> {
    if !is_implicit_context_symbol(base_name) {
        return None;
    }

    let base_lower = base_name.to_lowercase();
    let mut current_scope = Some(scope_position.scope_id);
    let mut visible = false;
    while let Some(scope_id) = current_scope {
        let Some(scope) = ir_program.get_scope(scope_id) else {
            break;
        };
        if scope
            .variables
            .keys()
            .any(|name| name.to_lowercase() == base_lower)
        {
            visible = true;
            break;
        }
        current_scope = scope.parent;
    }

    if !visible {
        return None;
    }

    let location = CodeLocation::determine_from_path(Path::new(ctx.file_path)).ok()?;

    let descriptor = match location.module_type {
        ModuleType::FormModule {
            form_name,
            owner_type,
        } => {
            let (kind, owner_name) = parse_owner_kind(&owner_type)?;
            let owner_name = owner_name.to_string();

            match base_lower.as_str() {
                "этотобъект" | "этаформа" | "форма" => {
                    ContextualTypeDescriptor::FormType {
                        kind,
                        owner_name,
                        form_name,
                    }
                }
                "объект" => ContextualTypeDescriptor::FormDataObject {
                    kind,
                    owner_name,
                    form_name,
                },
                "элементы" => ContextualTypeDescriptor::FormElementsType {
                    kind,
                    owner_name,
                    form_name,
                },
                "параметры" => ContextualTypeDescriptor::PlatformType {
                    type_name: "Структура".to_string(),
                },
                _ => return None,
            }
        }
        ModuleType::ManagerModule { owner_type } => {
            if !matches!(base_lower.as_str(), "этотобъект" | "объект") {
                return None;
            }
            let (kind, owner_name) = parse_owner_kind(&owner_type)?;
            ContextualTypeDescriptor::ConfigurationFacet {
                kind,
                name: owner_name.to_string(),
                facet: FacetKind::Manager,
            }
        }
        ModuleType::ObjectModule { owner_type } | ModuleType::RecordSetModule { owner_type } => {
            if !matches!(base_lower.as_str(), "этотобъект" | "объект") {
                return None;
            }
            let (kind, owner_name) = parse_owner_kind(&owner_type)?;
            ContextualTypeDescriptor::ConfigurationFacet {
                kind,
                name: owner_name.to_string(),
                facet: FacetKind::Object,
            }
        }
        _ => return None,
    };

    let resolution = resolve_type_from_contextual_descriptor(Some(ctx.resolver), &descriptor);
    if resolution.is_unknown() || resolution.is_dynamic() {
        None
    } else {
        Some(resolution)
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

#[derive(Debug, Clone)]
struct CompletionScopePosition {
    byte_offset: u32,
    scope_id: ScopeId,
    scope_rank: HashMap<ScopeId, usize>,
}

#[derive(Debug, Clone)]
struct LocalSymbolCandidate {
    name: String,
    scope_id: ScopeId,
    span_start: u32,
}

const IMPLICIT_CONTEXT_SYMBOL_KEYS: [&str; 6] = [
    "этотобъект",
    "этаформа",
    "форма",
    "объект",
    "элементы",
    "параметры",
];

fn is_implicit_context_symbol(name: &str) -> bool {
    let lowered = name.to_lowercase();
    IMPLICIT_CONTEXT_SYMBOL_KEYS.contains(&lowered.as_str())
}

fn resolve_loop_body_scope(
    ir_program: &SemanticProgram,
    parent_scope: ScopeId,
    body: &[usize],
    loop_variable: Option<&str>,
    loop_span_start: u32,
) -> Option<ScopeId> {
    if let Some(scope_id) = body
        .iter()
        .filter_map(|idx| ir_program.nodes.get(*idx).map(|node| node.scope_id))
        .next()
    {
        return Some(scope_id);
    }

    let parent = ir_program.get_scope(parent_scope)?;
    match loop_variable {
        Some(variable) => parent.children.iter().copied().find(|child| {
            ir_program
                .get_scope(*child)
                .and_then(|scope| scope.variables.get(variable))
                .map(|state| state.declaration_span.start == loop_span_start)
                .unwrap_or(false)
        }),
        None => parent.children.first().copied(),
    }
}

fn scope_from_body_nodes(ir_program: &SemanticProgram, body: &[usize]) -> Option<ScopeId> {
    body.iter()
        .filter_map(|idx| ir_program.nodes.get(*idx).map(|node| node.scope_id))
        .next()
}

fn body_bounds(ir_program: &SemanticProgram, body: &[usize]) -> Option<(u32, u32)> {
    let mut start: Option<u32> = None;
    let mut end: Option<u32> = None;

    for node_index in body {
        let Some(node) = ir_program.nodes.get(*node_index) else {
            continue;
        };
        start = Some(start.map_or(node.span.start, |value| value.min(node.span.start)));
        end = Some(end.map_or(node.span.end, |value| value.max(node.span.end)));
    }

    match (start, end) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    }
}

fn completion_scope_for_enclosing_node(
    ir_program: &SemanticProgram,
    node: &bsl_shared::ir::SemanticNode,
    byte_offset: u32,
) -> ScopeId {
    fn in_bounds(byte_offset: u32, start: u32, end: u32) -> bool {
        start <= byte_offset && byte_offset < end
    }

    match &node.kind {
        SemanticNodeKind::FunctionDeclaration { body_scope, .. }
        | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => *body_scope,
        SemanticNodeKind::BlockScope { scope_id, .. } => *scope_id,
        SemanticNodeKind::IfStatement {
            then_branch,
            else_branch,
        } => {
            let then_scope = scope_from_body_nodes(ir_program, then_branch);
            let then_bounds = body_bounds(ir_program, then_branch);
            let else_scope = else_branch
                .as_ref()
                .and_then(|body| scope_from_body_nodes(ir_program, body));
            let else_bounds = else_branch
                .as_ref()
                .and_then(|body| body_bounds(ir_program, body));

            if let (Some(scope), Some((else_start, _))) = (else_scope, else_bounds) {
                if node.span.contains(byte_offset) && byte_offset >= else_start {
                    return scope;
                }
            }

            if let (Some(scope), Some((then_start, then_end))) = (then_scope, then_bounds) {
                if in_bounds(byte_offset, then_start, then_end) {
                    return scope;
                }

                if let Some(else_scope) = else_scope {
                    if node.span.contains(byte_offset) && byte_offset > then_end {
                        return else_scope;
                    }
                    if node.span.contains(byte_offset) {
                        return node.scope_id;
                    }
                    return node.scope_id;
                }

                if node.span.contains(byte_offset) {
                    return scope;
                }
                return node.scope_id;
            }

            if node.span.contains(byte_offset) {
                return then_scope.or(else_scope).unwrap_or(node.scope_id);
            }

            node.scope_id
        }
        SemanticNodeKind::TryExcept {
            try_body,
            except_body,
        } => {
            let try_scope = scope_from_body_nodes(ir_program, try_body);
            let try_bounds = body_bounds(ir_program, try_body);
            let except_scope = scope_from_body_nodes(ir_program, except_body);
            let except_bounds = body_bounds(ir_program, except_body);

            if let (Some(scope), Some((except_start, _))) = (except_scope, except_bounds) {
                if node.span.contains(byte_offset) && byte_offset >= except_start {
                    return scope;
                }
            }

            if let (Some(scope), Some((try_start, try_end))) = (try_scope, try_bounds) {
                if in_bounds(byte_offset, try_start, try_end) {
                    return scope;
                }

                if let Some(except_scope) = except_scope {
                    if node.span.contains(byte_offset) && byte_offset > try_end {
                        return except_scope;
                    }
                    if node.span.contains(byte_offset) {
                        return node.scope_id;
                    }
                    return node.scope_id;
                }

                if node.span.contains(byte_offset) {
                    return scope;
                }
                return node.scope_id;
            }

            if node.span.contains(byte_offset) {
                return try_scope.or(except_scope).unwrap_or(node.scope_id);
            }

            node.scope_id
        }
        SemanticNodeKind::ForLoop { variable, body }
        | SemanticNodeKind::ForEachLoop { variable, body } => resolve_loop_body_scope(
            ir_program,
            node.scope_id,
            body,
            Some(variable.as_str()),
            node.span.start,
        )
        .unwrap_or(node.scope_id),
        SemanticNodeKind::WhileLoop { body } => {
            resolve_loop_body_scope(ir_program, node.scope_id, body, None, node.span.start)
                .unwrap_or(node.scope_id)
        }
        _ => node.scope_id,
    }
}

fn resolve_completion_scope_position(
    ir_program: &SemanticProgram,
    file_content: &str,
    line: u32,
    column: u32,
) -> Option<CompletionScopePosition> {
    let line_index = LineIndex::new(file_content);
    let byte_offset = line_index.utf16_position_to_byte_offset(file_content, line, column);
    let byte_offset: u32 = byte_offset.try_into().ok()?;

    let scope_id = {
        let from_node = (0u32..=32)
            .filter_map(|delta| byte_offset.checked_sub(delta))
            .find_map(|offset| ir_program.find_node_at_byte_offset(offset))
            .map(|node| completion_scope_for_enclosing_node(ir_program, node, byte_offset));

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

        from_node
            .or_else(from_enclosing_decl)
            .or_else(from_prev_node)?
    };

    let mut visible_scopes = Vec::new();
    let mut current_scope_id = Some(scope_id);
    while let Some(sid) = current_scope_id {
        visible_scopes.push(sid);
        current_scope_id = ir_program.get_scope(sid).and_then(|scope| scope.parent);
    }

    let scope_rank = visible_scopes
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, sid)| (sid, idx))
        .collect();

    Some(CompletionScopePosition {
        byte_offset,
        scope_id,
        scope_rank,
    })
}

fn collect_local_candidates_from_ir(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
) -> Vec<LocalSymbolCandidate> {
    let mut best_by_name: HashMap<String, LocalSymbolCandidate> = HashMap::new();

    let mut push_candidate = |name: &str, scope_id: ScopeId, span_start: u32| {
        push_local_candidate_if_visible(
            ir_program,
            scope_position,
            &mut best_by_name,
            name,
            scope_id,
            span_start,
            false,
        );
    };

    let enclosing_function_scope = scope_position
        .scope_rank
        .iter()
        .filter_map(|(scope_id, rank)| {
            let scope = ir_program.get_scope(*scope_id)?;
            matches!(scope.kind, ScopeKind::Function).then_some((*rank, *scope_id))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, scope_id)| scope_id);

    let mut collected_from_routine = false;
    if let Some(function_scope_id) = enclosing_function_scope {
        if let Some(decl_node) = ir_program.nodes.iter().find(|node| match &node.kind {
            SemanticNodeKind::FunctionDeclaration { body_scope, .. }
            | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => {
                *body_scope == function_scope_id
            }
            _ => false,
        }) {
            match &decl_node.kind {
                SemanticNodeKind::FunctionDeclaration { params, body, .. }
                | SemanticNodeKind::ProcedureDeclaration { params, body, .. } => {
                    for param in params {
                        push_candidate(&param.name, function_scope_id, decl_node.span.start);
                    }
                    collect_local_candidates_from_body(
                        ir_program,
                        scope_position,
                        body,
                        &mut push_candidate,
                    );
                    collected_from_routine = true;
                }
                _ => {}
            }
        }
    }

    if !collected_from_routine {
        for node in ir_program.nodes.iter() {
            match &node.kind {
                SemanticNodeKind::VariableDeclaration { name, .. } => {
                    push_candidate(name, node.scope_id, node.span.start);
                }
                SemanticNodeKind::Assignment { variable, .. } => {
                    push_candidate(variable, node.scope_id, node.span.start);
                }
                SemanticNodeKind::FunctionDeclaration {
                    params, body_scope, ..
                }
                | SemanticNodeKind::ProcedureDeclaration {
                    params, body_scope, ..
                } => {
                    for param in params {
                        push_candidate(&param.name, *body_scope, node.span.start);
                    }
                }
                SemanticNodeKind::ForLoop { variable, body }
                | SemanticNodeKind::ForEachLoop { variable, body } => {
                    if let Some(loop_scope) = resolve_loop_body_scope(
                        ir_program,
                        node.scope_id,
                        body,
                        Some(variable.as_str()),
                        node.span.start,
                    ) {
                        push_candidate(variable, loop_scope, node.span.start);
                    }
                }
                _ => {}
            }
        }
    }

    drop(push_candidate);

    // Дополняем кандидатов только implicit symbols из SymbolTable:
    // они могут не иметь отдельных AST/IR-узлов.
    for scope_id in scope_position.scope_rank.keys().copied() {
        let Some(scope) = ir_program.get_scope(scope_id) else {
            continue;
        };
        for (name, state) in scope.variables.iter() {
            if is_implicit_context_symbol(name) {
                push_local_candidate_if_visible(
                    ir_program,
                    scope_position,
                    &mut best_by_name,
                    name,
                    scope_id,
                    state.declaration_span.start,
                    true,
                );
            }
        }
    }

    let mut out: Vec<LocalSymbolCandidate> = best_by_name.into_values().collect();
    out.sort_by(|left, right| {
        let left_rank = scope_position
            .scope_rank
            .get(&left.scope_id)
            .copied()
            .unwrap_or(usize::MAX);
        let right_rank = scope_position
            .scope_rank
            .get(&right.scope_id)
            .copied()
            .unwrap_or(usize::MAX);
        left_rank
            .cmp(&right_rank)
            .then_with(|| right.span_start.cmp(&left.span_start))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    out
}

fn collect_local_candidates_from_body<F>(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
    body: &[usize],
    push_candidate: &mut F,
) where
    F: FnMut(&str, ScopeId, u32),
{
    for node_index in body {
        collect_local_candidates_from_node(ir_program, scope_position, *node_index, push_candidate);
    }
}

fn collect_local_candidates_from_node<F>(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
    node_index: usize,
    push_candidate: &mut F,
) where
    F: FnMut(&str, ScopeId, u32),
{
    let Some(node) = ir_program.nodes.get(node_index) else {
        return;
    };
    if node.span.start > scope_position.byte_offset {
        return;
    }

    match &node.kind {
        SemanticNodeKind::VariableDeclaration { name, .. } => {
            push_candidate(name, node.scope_id, node.span.start);
        }
        SemanticNodeKind::Assignment { variable, .. } => {
            push_candidate(variable, node.scope_id, node.span.start);
        }
        SemanticNodeKind::IfStatement {
            then_branch,
            else_branch,
        } => {
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                then_branch,
                push_candidate,
            );
            if let Some(else_branch) = else_branch.as_ref() {
                collect_local_candidates_from_body(
                    ir_program,
                    scope_position,
                    else_branch,
                    push_candidate,
                );
            }
        }
        SemanticNodeKind::TryExcept {
            try_body,
            except_body,
        } => {
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                try_body,
                push_candidate,
            );
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                except_body,
                push_candidate,
            );
        }
        SemanticNodeKind::WhileLoop { body } => {
            collect_local_candidates_from_body(ir_program, scope_position, body, push_candidate);
        }
        SemanticNodeKind::ForLoop { variable, body }
        | SemanticNodeKind::ForEachLoop { variable, body } => {
            if let Some(loop_scope) = resolve_loop_body_scope(
                ir_program,
                node.scope_id,
                body,
                Some(variable.as_str()),
                node.span.start,
            ) {
                push_candidate(variable, loop_scope, node.span.start);
            }
            collect_local_candidates_from_body(ir_program, scope_position, body, push_candidate);
        }
        SemanticNodeKind::BlockScope { statements, .. } => {
            collect_local_candidates_from_body(
                ir_program,
                scope_position,
                statements,
                push_candidate,
            );
        }
        _ => {}
    }
}

fn push_local_candidate_if_visible(
    ir_program: &SemanticProgram,
    scope_position: &CompletionScopePosition,
    best_by_name: &mut HashMap<String, LocalSymbolCandidate>,
    name: &str,
    scope_id: ScopeId,
    span_start: u32,
    allow_global: bool,
) {
    if span_start > scope_position.byte_offset {
        return;
    }

    let Some(candidate_rank) = scope_position.scope_rank.get(&scope_id).copied() else {
        return;
    };
    let Some(scope) = ir_program.get_scope(scope_id) else {
        return;
    };
    if !allow_global && matches!(scope.kind, ScopeKind::Global) {
        return;
    }

    let candidate = LocalSymbolCandidate {
        name: name.to_string(),
        scope_id,
        span_start,
    };
    let key = name.to_lowercase();

    let should_replace = match best_by_name.get(&key) {
        None => true,
        Some(existing) => {
            let existing_rank = scope_position
                .scope_rank
                .get(&existing.scope_id)
                .copied()
                .unwrap_or(usize::MAX);
            candidate_rank < existing_rank
                || (candidate_rank == existing_rank && span_start > existing.span_start)
        }
    };

    if should_replace {
        best_by_name.insert(key, candidate);
    }
}

fn add_local_symbols_from_ir(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    let Some(ctx) = analysis else {
        return;
    };
    let Some(ir_program) = ctx.ir_program.as_deref() else {
        return;
    };
    let Some(scope_position) =
        resolve_completion_scope_position(ir_program, file_content, line, column)
    else {
        return;
    };

    for local in collect_local_candidates_from_ir(ir_program, &scope_position) {
        target.push(Candidate::new(
            CompletionItem::new(local.name, CompletionKind::Variable),
            priority,
            None,
            Some(SymbolScope::Local),
        ));
    }
}

fn add_symbols(
    snapshot: &IndexSnapshot,
    file_uri: Option<&str>,
    target: &mut Vec<Candidate>,
    priority: u8,
    include_local: bool,
) {
    let Some(uri) = file_uri else {
        return;
    };
    let Some(items) = snapshot.symbol_index.get(uri) else {
        return;
    };

    for item in items.iter() {
        if matches!(item.scope, Some(SymbolScope::Local)) {
            if !include_local {
                continue;
            }
            let allow_unbound_local_routine = matches!(
                item.kind,
                IndexItemKind::Symbol(SymbolKind::Function | SymbolKind::Procedure)
            );
            if item.uri.as_deref() != Some(uri) && !allow_unbound_local_routine {
                continue;
            }
        }
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

    fn normalize_param_label(param_name: &str) -> String {
        let trimmed = param_name.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        // 1C Syntax Helper часто использует плейсхолдеры в угловых скобках:
        // "<Имя>", "<Тип>", "<Заголовок>" и т.п. (в т.ч. HTML-encoded).
        // Для сниппета нам нужен читабельный label без скобок.
        let unwrapped = trimmed
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .or_else(|| {
                trimmed
                    .strip_prefix("&lt;")
                    .and_then(|s| s.strip_suffix("&gt;"))
            })
            .unwrap_or(trimmed);

        unwrapped.trim().to_string()
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
        let normalized = normalize_param_label(&param_name);
        let label = if normalized.is_empty() {
            format!("param{}", index)
        } else {
            normalized
        };

        // Даже если параметр необязательный, показываем его имя в плейсхолдере:
        // это важно для UX (Tab по аргументам в сниппете).
        //
        // Клиент может удалять/пропускать необязательные параметры вручную,
        // но пустые плейсхолдеры ухудшают подсказки и навигацию.
        let placeholder = if is_optional && param_name.trim().is_empty() {
            // Если имя реально пустое - оставляем пустым плейсхолдером.
            format!("${{{}:}}", index)
        } else {
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
    use crate::system::{IndexItem, SymbolKind, SymbolScope};
    use bsl_analysis_v2::{
        AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
    };
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::signature_index::{
        ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
    };
    use bsl_shared::domain::type_id::TypeId;
    use bsl_shared::domain::types::{
        Certainty, ConcreteType, ConfigurationType, FacetKind, MetadataKind, RawDataSource,
        RawMethodData, RawPropertyData, RawTypeData, ResolutionMetadata, ResolutionResult,
        ResolutionSource, TypeResolution, FORM_DATA_FORM_TYPE_NOTE_PREFIX,
        FORM_DATA_SEMANTICS_NOTE,
    };
    use bsl_shared::formatting::DetailLevel;
    use std::sync::Arc;

    #[test]
    fn trim_to_window_keeps_tail() {
        let input = "0123456789";
        let trimmed = trim_to_window(input, 4);
        assert_eq!(trimmed, "6789");
    }

    #[test]
    fn with_sort_text_uses_original_label_as_case_tie_break() {
        let item = CompletionItem::new("Apple".to_string(), CompletionKind::Property);
        let with_sort = with_sort_text(item, 0.5, 1, "apple");
        let sort_text = with_sort.sort_text.expect("sort_text should be set");

        assert_eq!(sort_text, "apple-Apple-01-0500");
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
    fn completion_context_detects_member_access_when_cursor_is_on_dot() {
        let content = "Объект.";
        let (line, column) = utf16_column(content, ".");
        let ctx = analyze_completion_context(content, line, column);

        assert!(ctx.member_access);
        assert_eq!(ctx.member_base.as_deref(), Some("Объект"));
        assert_eq!(ctx.trigger_char, Some('.'));
        assert!(
            ctx.current_word.is_empty(),
            "cursor-on-dot should not keep previous identifier as prefix"
        );
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
    fn add_properties_from_resolution_preserves_form_data_provider_order_priorities() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "Документы.Док1".to_string(),
                    source: RawDataSource::Configuration,
                    facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                    kind: Some(MetadataKind::Document),
                    properties: vec![RawPropertyData {
                        name: "СвойствоМетаданных".to_string(),
                        prop_type: "Число".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "Формы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    properties: vec![RawPropertyData {
                        name: "РеквизитФормы".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "ДокументОбъект".to_string(),
                    source: RawDataSource::Platform,
                    facets: vec![FacetKind::Object],
                    properties: vec![RawPropertyData {
                        name: "ФацетСвойство".to_string(),
                        prop_type: "Число".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn TypeRepository> = repository.clone();
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Document,
                name: "Док1".to_string(),
                facet: Some(FacetKind::Object),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                notes: vec![
                    FORM_DATA_SEMANTICS_NOTE.to_string(),
                    format!(
                        "{}{}",
                        FORM_DATA_FORM_TYPE_NOTE_PREFIX, "Формы.Документы.Док1.Форма1"
                    ),
                ],
                ..Default::default()
            },
            active_facet: Some(FacetKind::Object),
            available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        };

        let mut target = Vec::new();
        add_properties_from_resolution(&metadata_lookup, &resolution, &mut target, 1);

        let link = target
            .iter()
            .find(|candidate| candidate.item.label == "Ссылка")
            .expect("missing intrinsic Ссылка");
        assert_eq!(link.source_priority, 2);

        let deletion_mark = target
            .iter()
            .find(|candidate| candidate.item.label == "ПометкаУдаления")
            .expect("missing intrinsic ПометкаУдаления");
        assert_eq!(deletion_mark.source_priority, 2);

        let metadata_prop = target
            .iter()
            .find(|candidate| candidate.item.label == "СвойствоМетаданных")
            .expect("missing metadata property");
        assert_eq!(metadata_prop.source_priority, 3);

        assert!(
            target
                .iter()
                .all(|candidate| candidate.item.label != "РеквизитФормы"),
            "form-data property completion must not include form-shape properties"
        );
        assert!(
            target
                .iter()
                .all(|candidate| candidate.item.label != "ФацетСвойство"),
            "form-data property completion must not include object-facet fallback properties"
        );
    }

    #[test]
    fn form_data_member_completion_includes_intrinsic_and_excludes_shape_and_facet_members() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "Документы.Док1".to_string(),
                    source: RawDataSource::Configuration,
                    facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                    kind: Some(MetadataKind::Document),
                    ..Default::default()
                },
                RawTypeData {
                    name: "Формы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    properties: vec![RawPropertyData {
                        name: "РеквизитФормы".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "ДокументОбъект".to_string(),
                    source: RawDataSource::Platform,
                    facets: vec![FacetKind::Object],
                    methods: vec![RawMethodData {
                        name: "Записать".to_string(),
                        return_type: "Булево".to_string(),
                        ..Default::default()
                    }],
                    properties: vec![RawPropertyData {
                        name: "ФацетСвойство".to_string(),
                        prop_type: "Число".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn TypeRepository> = repository.clone();
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Document,
                name: "Док1".to_string(),
                facet: Some(FacetKind::Object),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                notes: vec![
                    FORM_DATA_SEMANTICS_NOTE.to_string(),
                    format!(
                        "{}{}",
                        FORM_DATA_FORM_TYPE_NOTE_PREFIX, "Формы.Документы.Док1.Форма1"
                    ),
                ],
                ..Default::default()
            },
            active_facet: Some(FacetKind::Object),
            available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        };

        let mut target = Vec::new();
        add_methods_from_resolution(&metadata_lookup, &resolution, &mut target, 0);
        add_properties_from_resolution(&metadata_lookup, &resolution, &mut target, 1);

        assert!(target.iter().any(|candidate| {
            matches!(candidate.item.kind, CompletionKind::Property)
                && candidate.item.label == "Ссылка"
        }));
        assert!(target.iter().any(|candidate| {
            matches!(candidate.item.kind, CompletionKind::Property)
                && candidate.item.label == "ПометкаУдаления"
        }));
        assert!(target.iter().all(|candidate| {
            !(matches!(candidate.item.kind, CompletionKind::Property)
                && candidate.item.label == "РеквизитФормы")
        }));
        assert!(target.iter().all(|candidate| {
            !(matches!(candidate.item.kind, CompletionKind::Property)
                && candidate.item.label == "ФацетСвойство")
        }));
        assert!(target.iter().all(|candidate| {
            !(matches!(candidate.item.kind, CompletionKind::Method)
                && candidate.item.label == "Записать")
        }));
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
        assert_eq!(snippet, "Открыть(${1:Путь}, ${2:Режим})$0");
    }

    #[test]
    fn build_call_snippet_normalizes_angle_brackets_in_labels() {
        let params = vec![
            ("<Имя>".to_string(), false),
            ("&lt;Тип&gt;".to_string(), false),
            ("<Заголовок>".to_string(), false),
            ("<Ширина>".to_string(), false),
        ];
        let snippet = build_call_snippet("Добавить", &params).expect("snippet");
        assert_eq!(
            snippet,
            "Добавить(${1:Имя}, ${2:Тип}, ${3:Заголовок}, ${4:Ширина})$0"
        );
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
    async fn completion_non_member_without_ir_does_not_use_file_local_symbols() {
        let index = IntellisenseIndexStore::new("cfg", "platform");
        let uri = "file:///completion_non_member_no_ir_sources_test.bsl";

        let mut local_from_index = IndexItem::new(
            "ИндексЛокал",
            IndexItemKind::Symbol(SymbolKind::Variable),
            crate::system::IndexKind::Symbol,
        );
        local_from_index.scope = Some(SymbolScope::Local);

        let mut module_from_index = IndexItem::new(
            "ИндексМодуль",
            IndexItemKind::Symbol(SymbolKind::Function),
            crate::system::IndexKind::Symbol,
        );
        module_from_index.scope = Some(SymbolScope::Module);

        index.replace_symbols_for_uri(uri, vec![local_from_index, module_from_index]);

        let repository = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repository);
        let content = "    Инд";
        let column = content.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

        let result = get_completion(content, 0, column, Some(uri), &index, &metadata_lookup)
            .await
            .expect("completion ok");
        let labels: Vec<String> = result
            .items
            .into_iter()
            .map(|candidate| candidate.item.label)
            .collect();

        assert!(
            labels.iter().any(|label| label == "ИндексМодуль"),
            "labels: {:?}",
            labels
        );
        assert!(
            !labels.iter().any(|label| label == "ИндексЛокал"),
            "labels: {:?}",
            labels
        );
    }

    fn build_non_member_completion_fixture(
        content: &str,
        file_path: &str,
    ) -> (
        IntellisenseIndexStore,
        TypeMetadataLookup,
        Arc<TypeResolver>,
        Arc<bsl_shared::ir::SemanticProgram>,
    ) {
        let repository = Arc::new(InMemoryTypeRepository::new());
        let repo: Arc<dyn TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repo.get_signature_index_clone(),
            resolver: Some(resolver.clone()),
            repository: repo,
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
            path: Arc::from(file_path.to_string()),
        });

        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");
        let index = IntellisenseIndexStore::new("cfg", "platform");

        (index, metadata_lookup, resolver, ir_program)
    }

    async fn completion_labels_non_member(
        content: &str,
        line: u32,
        column: u32,
        file_uri: Option<&str>,
        file_path: &str,
        index: &IntellisenseIndexStore,
        metadata_lookup: &TypeMetadataLookup,
        resolver: &TypeResolver,
        ir_program: Arc<bsl_shared::ir::SemanticProgram>,
    ) -> Vec<String> {
        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver,
            file_path,
            parse_result: None,
            member_access_owner_type_hint: None,
            include_flow_sensitive: false,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            file_uri,
            index,
            metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        result
            .items
            .into_iter()
            .map(|candidate| candidate.item.label)
            .collect()
    }

    #[tokio::test]
    async fn completion_non_member_hides_block_locals_outside_if() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Если Истина Тогда\n",
            "        ВнутриБлока = 1;\n",
            "    КонецЕсли;\n",
            "    Вн\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_block_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let line = 4;
        let column = "    Вн".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            line,
            column,
            Some("file:///completion_lexical_block_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;

        assert!(
            !labels.iter().any(|label| label == "ВнутриБлока"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_handles_else_boundary_without_then_leak() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Если Истина Тогда\n",
            "        ТогдаЛокал = 1;\n",
            "    Иначе\n",
            "        Ло\n",
            "        ЛокалИначе = 2;\n",
            "    КонецЕсли;\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_else_boundary_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "        Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            4,
            query_column,
            Some("file:///completion_lexical_else_boundary_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;
        assert!(
            !labels.iter().any(|label| label == "ТогдаЛокал"),
            "labels: {:?}",
            labels
        );
        assert!(
            !labels.iter().any(|label| label == "ЛокалИначе"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_after_if_end_does_not_leak_branch_locals() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Если Истина Тогда\n",
            "        ТогдаЛокал = 1;\n",
            "    Иначе\n",
            "        ИначеЛокал = 2;\n",
            "    КонецЕсли;\n",
            "    Ло\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_after_if_end_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "    Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            6,
            query_column,
            Some("file:///completion_lexical_after_if_end_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;

        assert!(
            !labels.iter().any(|label| label == "ТогдаЛокал"),
            "labels: {:?}",
            labels
        );
        assert!(
            !labels.iter().any(|label| label == "ИначеЛокал"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_respects_position_before_and_after_declaration() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Пос\n",
            "    После = 1;\n",
            "    Пос\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_position_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "    Пос".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels_before = completion_labels_non_member(
            content,
            1,
            query_column,
            Some("file:///completion_lexical_position_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program.clone(),
        )
        .await;
        assert!(
            !labels_before.iter().any(|label| label == "После"),
            "labels_before: {:?}",
            labels_before
        );

        let labels_after = completion_labels_non_member(
            content,
            3,
            query_column,
            Some("file:///completion_lexical_position_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;
        assert!(
            labels_after.iter().any(|label| label == "После"),
            "labels_after: {:?}",
            labels_after
        );
    }

    #[tokio::test]
    async fn completion_non_member_prefers_nearest_scope_for_shadowed_names() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Имя = 1;\n",
            "    Если Истина Тогда\n",
            "        ИМЯ = 2;\n",
            "        им\n",
            "    КонецЕсли;\n",
            "    им\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_shadow_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let inner_column = "        им".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels_inner = completion_labels_non_member(
            content,
            4,
            inner_column,
            Some("file:///completion_lexical_shadow_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program.clone(),
        )
        .await;
        assert!(
            labels_inner.iter().any(|label| label == "ИМЯ"),
            "labels_inner: {:?}",
            labels_inner
        );
        assert!(
            !labels_inner.iter().any(|label| label == "Имя"),
            "labels_inner: {:?}",
            labels_inner
        );

        let outer_column = "    им".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels_outer = completion_labels_non_member(
            content,
            6,
            outer_column,
            Some("file:///completion_lexical_shadow_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;
        assert!(
            labels_outer.iter().any(|label| label == "Имя"),
            "labels_outer: {:?}",
            labels_outer
        );
        assert!(
            !labels_outer.iter().any(|label| label == "ИМЯ"),
            "labels_outer: {:?}",
            labels_outer
        );
    }

    #[tokio::test]
    async fn completion_non_member_implicit_local_visible_from_assignment() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Ло\n",
            "    Локал = Новый Массив;\n",
            "    Ло\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_implicit_local_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "    Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels_before = completion_labels_non_member(
            content,
            1,
            query_column,
            Some("file:///completion_lexical_implicit_local_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program.clone(),
        )
        .await;
        assert!(
            !labels_before.iter().any(|label| label == "Локал"),
            "labels_before: {:?}",
            labels_before
        );

        let labels_after = completion_labels_non_member(
            content,
            3,
            query_column,
            Some("file:///completion_lexical_implicit_local_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;
        assert!(
            labels_after.iter().any(|label| label == "Локал"),
            "labels_after: {:?}",
            labels_after
        );
    }

    #[tokio::test]
    async fn completion_non_member_hides_loop_locals_outside_loop() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Для Счетчик = 1 По 2 Цикл\n",
            "        ВЦикле = Счетчик;\n",
            "    КонецЦикла;\n",
            "    ВЦ\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_loop_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "    ВЦ".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            4,
            query_column,
            Some("file:///completion_lexical_loop_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;
        assert!(
            !labels.iter().any(|label| label == "ВЦикле"),
            "labels: {:?}",
            labels
        );
        assert!(
            !labels.iter().any(|label| label == "Счетчик"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_shows_loop_variable_inside_loop() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Для Счетчик = 1 По 2 Цикл\n",
            "        Сч\n",
            "    КонецЦикла;\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_loop_variable_inside_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "        Сч".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            2,
            query_column,
            Some("file:///completion_lexical_loop_variable_inside_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;
        assert!(
            labels.iter().any(|label| label == "Счетчик"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_ignores_non_identifier_assignment_targets() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Объект.Поле = 1;\n",
            "    По\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_assignment_target_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let column = "    По".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            2,
            column,
            Some("file:///completion_lexical_assignment_target_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;

        assert!(
            !labels.iter().any(|label| label == "Поле"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_handles_except_boundary_without_try_leak() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Попытка\n",
            "        ЛокалПопытка = 1;\n",
            "    Исключение\n",
            "        Ло\n",
            "        ЛокалИсключение = 2;\n",
            "    КонецПопытки;\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_except_boundary_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "        Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            4,
            query_column,
            Some("file:///completion_lexical_except_boundary_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;
        assert!(
            !labels.iter().any(|label| label == "ЛокалПопытка"),
            "labels: {:?}",
            labels
        );
        assert!(
            !labels.iter().any(|label| label == "ЛокалИсключение"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_after_try_end_does_not_leak_except_locals() {
        let content = concat!(
            "Процедура Тест()\n",
            "    Попытка\n",
            "        ЛокалПопытка = 1;\n",
            "    Исключение\n",
            "        ЛокалИсключение = 2;\n",
            "    КонецПопытки;\n",
            "    Ло\n",
            "КонецПроцедуры\n"
        );
        let file_path = "completion_lexical_after_try_end_scope_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let query_column = "    Ло".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            6,
            query_column,
            Some("file:///completion_lexical_after_try_end_scope_test.bsl"),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;

        assert!(
            !labels.iter().any(|label| label == "ЛокалПопытка"),
            "labels: {:?}",
            labels
        );
        assert!(
            !labels.iter().any(|label| label == "ЛокалИсключение"),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_non_member_uses_index_for_non_local_symbols_only() {
        let content = concat!("Процедура Тест()\n", "    Инд\n", "КонецПроцедуры\n");
        let file_path = "completion_lexical_sources_test.bsl";
        let (index, metadata_lookup, resolver, ir_program) =
            build_non_member_completion_fixture(content, file_path);

        let uri = "file:///completion_lexical_sources_test.bsl";
        let mut local_from_index = IndexItem::new(
            "ИндексЛокал",
            IndexItemKind::Symbol(SymbolKind::Variable),
            crate::system::IndexKind::Symbol,
        );
        local_from_index.scope = Some(SymbolScope::Local);

        let mut module_from_index = IndexItem::new(
            "ИндексМодуль",
            IndexItemKind::Symbol(SymbolKind::Function),
            crate::system::IndexKind::Symbol,
        );
        module_from_index.scope = Some(SymbolScope::Module);

        index.replace_symbols_for_uri(uri, vec![local_from_index, module_from_index]);

        let column = "    Инд".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        let labels = completion_labels_non_member(
            content,
            1,
            column,
            Some(uri),
            file_path,
            &index,
            &metadata_lookup,
            resolver.as_ref(),
            ir_program,
        )
        .await;

        assert!(
            labels.iter().any(|label| label == "ИндексМодуль"),
            "labels: {:?}",
            labels
        );
        assert!(
            !labels.iter().any(|label| label == "ИндексЛокал"),
            "labels: {:?}",
            labels
        );
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
            include_flow_sensitive: false,
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
    async fn completion_resolves_implicit_form_object_member_access_without_hint() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![
                RawTypeData {
                    name: "Документы.Док1".to_string(),
                    source: RawDataSource::Configuration,
                    facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                    kind: Some(MetadataKind::Document),
                    ..Default::default()
                },
                RawTypeData {
                    name: "Формы.Документы.Док1.Форма1".to_string(),
                    source: RawDataSource::Configuration,
                    properties: vec![RawPropertyData {
                        name: "РеквизитФормы".to_string(),
                        prop_type: "Строка".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
                RawTypeData {
                    name: "ДокументОбъект".to_string(),
                    source: RawDataSource::Platform,
                    facets: vec![FacetKind::Object],
                    methods: vec![RawMethodData {
                        name: "Записать".to_string(),
                        return_type: "Булево".to_string(),
                        ..Default::default()
                    }],
                    properties: vec![RawPropertyData {
                        name: "ФацетСвойство".to_string(),
                        prop_type: "Число".to_string(),
                        is_readonly: false,
                    }],
                    ..Default::default()
                },
            ])
            .expect("load types");

        let repo: Arc<dyn bsl_shared::domain::repository::TypeRepository> = repository.clone();
        let resolver = Arc::new(TypeResolver::new(repo.clone()));
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());
        let index = IntellisenseIndexStore::new("cfg", "platform");

        let content = concat!("Процедура Тест()\n", "    Объект.\n", "КонецПроцедуры\n");
        let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
        let line = 1;
        // Cursor is positioned on '.' (not after it) to emulate editor behavior.
        let column = "    Объект".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

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
            path: Arc::from(file_path),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path,
            parse_result: None,
            member_access_owner_type_hint: None,
            include_flow_sensitive: false,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("file:///completion_form_module_implicit_owner_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(!labels.contains(&"Записать".to_string()), "labels: {:?}", labels);
        assert!(
            labels.contains(&"Ссылка".to_string()),
            "labels: {:?}",
            labels
        );
        assert!(
            labels.contains(&"ПометкаУдаления".to_string()),
            "labels: {:?}",
            labels
        );
    }

    #[tokio::test]
    async fn completion_uses_flow_sensitive_narrowing_for_member_access() {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![RawTypeData {
                name: "Строка".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Длина".to_string(),
                    return_type: "Число".to_string(),
                    ..Default::default()
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
            "    Перем x;\n",
            "    Если ТипЗнч(x) = Тип(\"Строка\") Тогда\n",
            "        x.\n",
            "    КонецЕсли;\n",
            "КонецПроцедуры\n"
        );

        let line = 3;
        let line_text = "        x.";
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
            path: Arc::from("completion_narrowing_test.bsl"),
        });
        let analysis = host.analysis();
        let ir_program = analysis.ir(V2FileId(1)).ok().flatten().expect("ir");

        let ctx = CompletionAnalysisContext {
            ir_program: Some(ir_program),
            resolver: resolver.as_ref(),
            file_path: "completion_narrowing_test.bsl",
            parse_result: None,
            member_access_owner_type_hint: None,
            include_flow_sensitive: true,
        };

        let result = get_completion_with_analysis(
            content,
            line,
            column,
            Some("completion_narrowing_test.bsl"),
            &index,
            &metadata_lookup,
            Some(&ctx),
        )
        .await
        .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(
            labels.contains(&"Длина".to_string()),
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
            include_flow_sensitive: false,
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
            include_flow_sensitive: false,
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
            include_flow_sensitive: false,
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
            include_flow_sensitive: false,
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
            include_flow_sensitive: false,
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
            include_flow_sensitive: false,
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
            include_flow_sensitive: false,
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
