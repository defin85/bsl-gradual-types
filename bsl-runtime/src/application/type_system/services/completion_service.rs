//! Completion Service - auto-completion operations
//!
//! Functions for LSP completion requests and contextual auto-completion.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, Span};

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::get_collection_kind;
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::{CompletionItem, CompletionKind, TypeMetadataLookup, TypeResolution};
use bsl_shared::ir::{ScopeId, ScopeKind, SemanticNodeKind, SemanticProgram};

use super::super::extractors::symbol_extractor::{
    extract_word_at_position, is_identifier_char, utf16_to_byte_offset,
};
use super::completion_target::extract_member_access_receiver_spans;
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
    #[allow(dead_code)]
    pub resolver: &'a TypeResolver,
    pub file_path: &'a str,
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

pub fn completion_member_access_owner_type_hints_from_analysis(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    column: u32,
) -> Vec<TypeResolution> {
    let Ok(exact_ready) = analysis.current_type_index_serve_only_ready(file_id) else {
        return Vec::new();
    };
    if !exact_ready {
        return Vec::new();
    }

    if let Ok(Some(ir_program)) = analysis.ir(file_id) {
        let resolutions = completion_member_access_owner_type_hints_from_semantic_program(
            file_content,
            line,
            column,
            ir_program.as_ref(),
        );
        if !resolutions.is_empty() {
            return resolutions;
        }
    }

    let Some(receiver_spans) = extract_member_access_receiver_spans(file_content, line, column) else {
        return Vec::new();
    };

    let mut resolutions = Vec::new();
    for span in receiver_spans {
        let mut probe_offsets = Vec::with_capacity(2);
        if span.end > span.start {
            probe_offsets.push(span.end.saturating_sub(1));
        }
        probe_offsets.push(span.start);

        for offset in probe_offsets {
            let Ok(profiled) = analysis.type_at_byte_offset_serve_only_profiled(file_id, offset) else {
                continue;
            };
            let Some(resolution) = profiled
                .resolution
                .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
            else {
                continue;
            };
            if !resolutions.contains(&resolution) {
                resolutions.push(resolution);
            }
            break;
        }
    }

    resolutions
}

pub fn completion_member_access_owner_type_hints_from_semantic_program(
    file_content: &str,
    line: u32,
    column: u32,
    ir_program: &SemanticProgram,
) -> Vec<TypeResolution> {
    let Some(receiver_spans) = extract_member_access_receiver_spans(file_content, line, column)
    else {
        return Vec::new();
    };

    let mut resolutions = Vec::new();
    for span in receiver_spans {
        let span = bsl_shared::ir::Span::new(span.start, span.end);
        let resolution = ir_program
            .semantic_facts
            .type_resolution_for_span(span)
            .or_else(|| {
                ir_program
                    .nodes
                    .iter()
                    .filter(|node| node.span.start <= span.start && node.span.end >= span.end)
                    .min_by_key(|node| node.span.len())
                    .and_then(|node| ir_program.semantic_facts.type_resolution_for_span(node.span))
            });
        let Some(resolution) = resolution.filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
        else {
            continue;
        };
        if !resolutions.contains(&resolution) {
            resolutions.push(resolution);
        }
    }

    resolutions
}

pub fn completion_member_access_owner_type_hint_from_analysis(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    column: u32,
) -> Option<TypeResolution> {
    let mut resolutions = completion_member_access_owner_type_hints_from_analysis(
        analysis,
        file_id,
        file_content,
        line,
        column,
    );
    if resolutions.len() == 1 {
        resolutions.pop()
    } else {
        None
    }
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
    member_access_owner_type_hint: Option<TypeResolution>,
    include_flow_sensitive: bool,
    trigger_char_hint: Option<char>,
) -> Result<CompletionResult> {
    let analysis = CompletionAnalysisContext {
        ir_program: Some(ir_program),
        resolver,
        file_path,
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
        // The shared member-access path may only use canonical owner hints or canonical
        // IR-derived owner facts for the current revision. Type-name and text/metadata
        // chain reconstruction must fail closed instead of synthesizing semantic truth.
        for owner_hint in resolve_member_owner_types_sync(analysis, file_content, line, column, "")
        {
            if let Some(kind) = get_collection_kind(&owner_hint.type_name()) {
                add_metadata_items_from_lookup(metadata_lookup, kind, &mut candidates, 0);
                continue;
            }
            add_methods_from_resolution(metadata_lookup, &owner_hint, &mut candidates, 0);
            add_properties_from_resolution(metadata_lookup, &owner_hint, &mut candidates, 1);
        }
    } else {
        collect_non_member_candidates(
            analysis,
            file_content,
            line,
            column,
            file_uri,
            &snapshot,
            metadata_lookup,
            &mut candidates,
        );
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

fn collect_non_member_candidates(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
    candidates: &mut Vec<Candidate>,
) {
    let can_collect_locals_from_ir = analysis.and_then(|ctx| ctx.ir_program.as_ref()).is_some();

    if can_collect_locals_from_ir {
        add_local_symbols_from_ir(analysis, file_content, line, column, candidates, 0);
        add_module_routines_from_ir(analysis, file_content, line, column, candidates, 1);
        add_global_functions_from_lookup(metadata_lookup, candidates, 1);
        add_all_metadata_items_from_lookup(metadata_lookup, candidates, 2);
        add_repository_types_from_lookup(metadata_lookup, candidates, 3);
        add_default_keywords(candidates, 4);
    } else {
        // In fallback mode keep local candidates from the file-bound index.
        add_symbols(snapshot, file_uri, candidates, 0, true);
        add_module_symbols(snapshot, candidates, 1);
        add_all_metadata_items_from_lookup(metadata_lookup, candidates, 2);
        add_types(snapshot, candidates, 3);
        add_keywords(snapshot, candidates, 4);
    }
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
