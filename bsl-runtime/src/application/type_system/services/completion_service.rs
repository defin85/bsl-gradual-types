//! Completion Service - auto-completion operations
//!
//! Functions for LSP completion requests and contextual auto-completion.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, Span};

use bsl_shared::domain::metadata_constants::get_collection_kind;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{
    ConcreteType, FacetKind, MetadataKind, ResolutionResult, SpecialType,
};
use bsl_shared::domain::{CompletionItem, CompletionKind, TypeMetadataLookup, TypeResolution};
use bsl_shared::ir::{ScopeId, ScopeKind, SemanticNodeKind, SemanticProgram};

use super::super::extractors::symbol_extractor::{
    extract_word_at_position, is_identifier_char, utf16_to_byte_offset,
};
use super::completion_ranking::{rank_candidates_with_trace, RankingCandidate};
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
    pub member_identity: Option<String>,
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
    #[allow(dead_code)]
    pub parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    pub member_access_owner_type_hint: Option<TypeResolution>,
    #[allow(dead_code)]
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
        None,
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
    get_completion_with_semantic_program_snapshot_with_trigger_hint(
        file_content,
        line,
        column,
        file_uri,
        index_snapshot,
        metadata_lookup,
        file_path,
        resolver,
        ir_program,
        member_access_owner_type_hint,
        include_flow_sensitive,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_completion_with_semantic_program_snapshot_with_trigger_hint(
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
    trigger_char_hint: Option<char>,
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
        trigger_char_hint,
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
    get_completion_with_semantic_program_snapshot_v2_with_trigger_hint(
        file_content,
        line,
        column,
        file_uri,
        index_snapshot,
        metadata_lookup,
        file_path,
        resolver,
        ir_program,
        parse_result,
        member_access_owner_type_hint,
        include_flow_sensitive,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_completion_with_semantic_program_snapshot_v2_with_trigger_hint(
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
    trigger_char_hint: Option<char>,
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
        trigger_char_hint,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_completion_with_semantic_hint_snapshot_with_trigger_hint(
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index_snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
    file_path: &str,
    resolver: &TypeResolver,
    parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    member_access_owner_type_hint: Option<TypeResolution>,
    include_flow_sensitive: bool,
    trigger_char_hint: Option<char>,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        ir_program: None,
        resolver,
        file_path,
        parse_result,
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
        trigger_char_hint,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn get_completion_with_analysis(
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index: &dyn IndexSnapshotSource,
    metadata_lookup: &TypeMetadataLookup,
    analysis: Option<&CompletionAnalysisContext<'_>>,
    trigger_char_hint: Option<char>,
) -> Result<CompletionResult> {
    let trace_request_id = if completion_trace_enabled() {
        Some(next_completion_request_id())
    } else {
        None
    };
    let analysis_file_path = analysis.map(|analysis| analysis.file_path);
    let context =
        analyze_completion_context_with_trigger_hint(file_content, line, column, trigger_char_hint);
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
        if let Some(owner_hint) = analysis
            .and_then(|ctx| ctx.member_access_owner_type_hint.as_ref())
            .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
            .cloned()
        {
            add_methods_from_resolution(metadata_lookup, &owner_hint, &mut candidates, 0);
            add_properties_from_resolution(metadata_lookup, &owner_hint, &mut candidates, 1);
        } else if let Some(receiver_chain) =
            extract_member_receiver_chain(file_content, line, column)
        {
            // Shared owner hints are now the only source of truth for per-instance
            // schema/effects. Local fallback below is limited to metadata/context chains.
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
            member_identity: candidate.member_identity,
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
            member_identity: candidate.member_identity,
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

#[path = "completion_service/context.rs"]
mod context;
#[path = "completion_service/member_resolution.rs"]
mod member_resolution;
#[path = "completion_service/scope_candidates.rs"]
mod scope_candidates;

pub use self::context::CompletionContext;
pub use self::member_resolution::{
    build_call_snippet, resolve_method_completion, resolve_type_details, CompletionResolveDetails,
};

use self::context::*;
use self::member_resolution::*;
use self::scope_candidates::*;

#[derive(Debug, Clone)]
struct Candidate {
    item: CompletionItem,
    source_priority: u8,
    label_lower: String,
    owner_type: Option<String>,
    member_identity: Option<String>,
    scope: Option<SymbolScope>,
}

impl Candidate {
    fn new(
        item: CompletionItem,
        source_priority: u8,
        owner_type: Option<String>,
        member_identity: Option<String>,
        scope: Option<SymbolScope>,
    ) -> Self {
        let label_lower = item.label.to_lowercase();
        Self {
            item,
            source_priority,
            label_lower,
            owner_type,
            member_identity,
            scope,
        }
    }
}

#[cfg(test)]
#[path = "completion_service/tests.rs"]
mod tests;
