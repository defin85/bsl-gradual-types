//! Core functionality: constructor and helper methods

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::lsp_types::request::{
    CodeActionRequest, Formatting as DocumentFormattingRequest, InlayHintRequest, RangeFormatting,
    Request as LspRequest,
};
use tower_lsp::lsp_types::MessageType;
use tower_lsp::lsp_types::{Registration, Unregistration};
use tower_lsp::Client;
use tracing::{debug, info, warn};

use bsl_analysis_v2::{AnalysisHostV2, DepsSnapshotId, FileId as V2FileId, SettingsId};
use bsl_backend::system::fs_utils::read_bsl_file;
use bsl_backend::system::{
    build_deps_bundle_v2, DepsBundleV2, DepsBundleV2Meta, SystemCoordinator,
};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;

use crate::config::BslSettings;
use crate::converters::{semantic_error_to_diagnostic, syntax_errors_to_diagnostics};

use super::analysis_v2_runtime::AnalysisV2Runtime;
use super::{
    BslLanguageServer, CodeActionsCapabilityState, DocumentShadowStateV2,
    FormattingCapabilityState, InlayHintsCapabilityState, Url, V2FileKey,
};

#[path = "core/capability_registration.rs"]
mod capability_registration;
#[path = "core/deps_and_precompute.rs"]
mod deps_and_precompute;
pub(crate) use deps_and_precompute::CompletionArtifactPollTraceV2;
pub(crate) use deps_and_precompute::CompletionArtifactWaitOutcomeV2;
pub(crate) use deps_and_precompute::ExactTypeIndexWaitOutcomeV2;
#[path = "core/diagnostics_runtime.rs"]
mod diagnostics_runtime;
#[path = "core/execution_context.rs"]
mod execution_context;
#[path = "core/snapshot_status.rs"]
mod snapshot_status;

fn diagnostics_debounce_duration() -> Duration {
    // Diagnostics are triggered on every `textDocument/didChange`. Computing full diagnostics is
    // CPU-bound and not preemptible (abort only works at await points). Without debouncing, rapid
    // typing can build up a backlog and make completion/hover feel "frozen".
    //
    // Default: 250ms. Can be overridden via env for experiments.
    // Clamp to a small floor to avoid "0ms" misconfiguration that turns debounced profiles into
    // tight loops under rapid didChange traffic.
    let raw = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::LspDiagnosticsDebounceMs)
        .unwrap_or(250);
    Duration::from_millis(clamp_diagnostics_debounce_ms(raw))
}

fn clamp_diagnostics_debounce_ms(raw: u64) -> u64 {
    raw.max(25)
}

fn duration_from_millis_u128(value_ms: u128) -> Duration {
    Duration::from_millis(value_ms.min(u64::MAX as u128) as u64)
}

fn next_completion_timeline_trace_id_from(counter: &std::sync::atomic::AtomicU64) -> String {
    let id = counter.fetch_add(1, Ordering::Relaxed);
    format!("completion-trace-{id}")
}

fn next_diagnostics_save_timeline_trace_id_from(counter: &std::sync::atomic::AtomicU64) -> String {
    let id = counter.fetch_add(1, Ordering::Relaxed);
    format!("diagnostics-save-trace-{id}")
}

fn next_did_change_parse_snapshot_evidence_id_from(
    counter: &std::sync::atomic::AtomicU64,
) -> String {
    let id = counter.fetch_add(1, Ordering::Relaxed);
    format!("did-change-parse-snapshot-{id}")
}

fn diagnostics_save_timeline_cycle_terminal_outcome(
    trace: &crate::types::DiagnosticsSaveTimelineTrace,
) -> Option<String> {
    if trace.save_fastlane_outcome.is_none() || trace.idle_heavy_outcome.is_none() {
        return None;
    }

    trace
        .idle_heavy_outcome
        .clone()
        .or_else(|| {
            trace
                .followup_publish
                .as_ref()
                .map(|publish| publish.outcome.clone())
        })
        .or_else(|| trace.save_fastlane_outcome.clone())
        .or_else(|| {
            trace
                .first_publish
                .as_ref()
                .map(|publish| publish.outcome.clone())
        })
}

fn archive_diagnostics_save_timeline_trace_inner(
    store: &mut super::DiagnosticsSaveTimelineStore,
    trace: crate::types::DiagnosticsSaveTimelineTrace,
) {
    store.traces.push_back(trace);
    while store.traces.len() > super::DIAGNOSTICS_SAVE_TIMELINE_MAX_ENTRIES {
        let _ = store.traces.pop_front();
    }
}

fn diagnostics_save_timeline_terminal_key_is_recorded_inner(
    store: &super::DiagnosticsSaveTimelineStore,
    key: super::DiagnosticsSaveTimelineCycleKey,
) -> bool {
    store.terminal_keys.keys.contains(&key)
}

fn remember_diagnostics_save_timeline_terminal_key_inner(
    store: &mut super::DiagnosticsSaveTimelineStore,
    key: super::DiagnosticsSaveTimelineCycleKey,
) {
    if store.terminal_keys.keys.insert(key) {
        store.terminal_keys.order.push_back(key);
    }
    while store.terminal_keys.order.len() > super::DIAGNOSTICS_SAVE_TIMELINE_MAX_ENTRIES {
        let Some(oldest_key) = store.terminal_keys.order.pop_front() else {
            break;
        };
        store.terminal_keys.keys.remove(&oldest_key);
    }
}

fn diagnostics_save_timeline_trace_mut_inner<'a>(
    store: &'a mut super::DiagnosticsSaveTimelineStore,
    uri: &Url,
    key: super::DiagnosticsSaveTimelineCycleKey,
) -> Option<&'a mut crate::types::DiagnosticsSaveTimelineTrace> {
    if let Some(trace) = store.active_cycles.get_mut(&key) {
        return Some(trace);
    }
    store.traces.iter_mut().rev().find(|trace| {
        trace.uri == uri.as_str()
            && trace.requested_version == key.requested_version
            && trace.diagnostics_generation == key.diagnostics_generation
            && trace.save_cycle_sequence == key.save_cycle_sequence
    })
}

fn snapshot_diagnostics_save_timeline_traces_inner(
    store: &super::DiagnosticsSaveTimelineStore,
    limit: usize,
) -> Vec<crate::types::DiagnosticsSaveTimelineTrace> {
    let mut traces = store.traces.iter().cloned().collect::<Vec<_>>();
    traces.extend(store.active_cycles.values().cloned());
    traces.sort_by(|left, right| {
        left.started_at_ms
            .cmp(&right.started_at_ms)
            .then_with(|| left.save_cycle_sequence.cmp(&right.save_cycle_sequence))
            .then_with(|| left.trace_id.cmp(&right.trace_id))
    });
    if traces.len() > limit {
        traces = traces.split_off(traces.len().saturating_sub(limit));
    }
    traces
}

fn snapshot_did_change_parse_snapshot_evidence_inner(
    store: &super::DidChangeParseSnapshotEvidenceStore,
    limit: usize,
) -> Vec<crate::types::DidChangeParseSnapshotEvidenceTrace> {
    let len = store.order.len();
    let start = len.saturating_sub(limit);
    store
        .order
        .iter()
        .skip(start)
        .filter_map(|key| store.entries.get(key).cloned())
        .collect()
}

fn finalize_diagnostics_save_timeline_trace_for_terminal_outcome(
    trace: &mut crate::types::DiagnosticsSaveTimelineTrace,
    terminal_outcome: &str,
) {
    let terminal_outcome = terminal_outcome.to_string();
    if trace.save_fastlane_outcome.is_none() {
        trace.save_fastlane_outcome = Some(terminal_outcome.clone());
    }
    if trace.idle_heavy_outcome.is_none() {
        trace.idle_heavy_outcome = Some(terminal_outcome.clone());
    }
    trace.followup_wait_reason = None;
    trace.followup_blocker_reason = None;
    trace.followup_wait_for_file_version_ms = None;
    trace.followup_snapshot_with_deps_ms = None;
    trace.terminal_outcome = Some(terminal_outcome);
}

fn clear_diagnostics_save_timeline_followup_wait_inner(
    trace: &mut crate::types::DiagnosticsSaveTimelineTrace,
) {
    trace.followup_wait_reason = None;
    trace.followup_blocker_reason = None;
    trace.followup_wait_for_file_version_ms = None;
    trace.followup_snapshot_with_deps_ms = None;
}

fn duration_to_nonzero_ms(duration: Option<Duration>) -> Option<u64> {
    let millis = duration.map(|value| value.as_millis().min(u64::MAX as u128) as u64)?;
    (millis > 0).then_some(millis)
}

fn diagnostics_save_coherence_debug_enabled() -> bool {
    std::env::var_os("BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE").is_some()
}

fn emit_diagnostics_save_coherence_debug(message: String) {
    if diagnostics_save_coherence_debug_enabled() {
        eprintln!("{message}");
    }
}

fn update_followup_timing_max(slot: &mut Option<u64>, candidate: Option<u64>) {
    let Some(candidate) = candidate.filter(|value| *value > 0) else {
        return;
    };
    *slot = Some(slot.unwrap_or(0).max(candidate));
}

fn overwrite_diagnostics_save_timeline_ready_snapshot_phase_attribution_view_inner(
    trace: &mut crate::types::DiagnosticsSaveTimelineTrace,
    attribution: diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2,
) {
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_ms,
        attribution.parse_exec_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_parse_build_ms,
        attribution.parse_exec_core_parse_build_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms,
        attribution.parse_exec_core_build_pre_parse_setup_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms,
        attribution.parse_exec_core_build_parser_base_recovery_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms,
        attribution.parse_exec_core_build_parser_tree_build_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms,
        attribution.parse_exec_core_build_exact_ready_snapshot_assembly_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
        attribution.parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
        attribution.parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
    );
    if let Some(value) = attribution.program_lowering_reuse_outcome {
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome =
            Some(value.to_string());
    }
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units =
        attribution.program_lowering_reused_lowering_units;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units =
        attribution.program_lowering_rebuilt_lowering_units;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count =
        attribution.program_lowering_reused_window_count;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count =
        attribution.program_lowering_rebuilt_window_count;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units =
        attribution.program_lowering_largest_rebuilt_window_lowering_units;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count =
        attribution.program_lowering_fully_reused_top_level_node_count;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count =
        attribution.program_lowering_fully_rebuilt_top_level_node_count;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count =
        attribution.program_lowering_routine_body_reuse_node_count;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units =
        attribution.program_lowering_fully_reused_top_level_lowering_units;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units =
        attribution.program_lowering_fully_rebuilt_top_level_lowering_units;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units =
        attribution.program_lowering_routine_body_reused_prefix_lowering_units;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units =
        attribution.program_lowering_routine_body_reused_suffix_lowering_units;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units =
        attribution.program_lowering_routine_body_rebuilt_lowering_units;
    if let Some(value) = attribution.program_lowering_reuse_plan_build_source {
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source =
            Some(value.to_string());
    }
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit =
        attribution.program_lowering_reuse_plan_take_if_unique_hit;
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit =
        attribution.program_lowering_reuse_plan_borrowed_cache_hit;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms,
        attribution.program_lowering_reuse_plan_build_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms,
        attribution.program_lowering_reuse_plan_owned_build_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms,
        attribution.program_lowering_reuse_plan_borrowed_build_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms,
        attribution.program_lowering_reuse_plan_rebase_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count =
        attribution.program_lowering_reuse_plan_rebase_statement_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms,
        attribution.program_lowering_reused_progress_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count =
        attribution.program_lowering_reused_progress_call_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms,
        attribution.program_lowering_rebuild_dispatch_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count =
        attribution.program_lowering_rebuild_dispatch_call_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms,
        attribution.program_lowering_rebuild_dispatch_callable_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count =
        attribution.program_lowering_rebuild_dispatch_callable_call_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms,
        attribution.program_lowering_rebuild_dispatch_callable_body_dispatch_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count =
        attribution.program_lowering_rebuild_dispatch_callable_body_dispatch_call_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms,
        attribution.program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms,
        attribution.program_lowering_rebuild_dispatch_control_flow_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count =
        attribution.program_lowering_rebuild_dispatch_control_flow_call_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms,
        attribution.program_lowering_rebuild_dispatch_simple_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count =
        attribution.program_lowering_rebuild_dispatch_simple_call_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms,
        attribution.program_lowering_rebuild_dispatch_other_ms,
    );
    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count =
        attribution.program_lowering_rebuild_dispatch_other_call_count;
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms,
        attribution
            .parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms,
        attribution
            .parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms,
    );
    if let Some(value) =
        attribution.parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint
    {
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint =
            Some(value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms,
        attribution.parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms,
        attribution.parse_exec_core_build_tree_cache_install_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms,
        attribution.parse_exec_optional_cache_enrichment_ms,
    );
    if let Some(value) = attribution.parse_exec_core_build_dominant_checkpoint {
        trace.followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint =
            Some(value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms,
        attribution.parse_exec_core_build_dominant_checkpoint_ms,
    );
    if let Some(value) = attribution.parse_exec_dominant_subphase {
        trace.followup_ready_snapshot_parse_exec_dominant_subphase = Some(value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_dominant_subphase_ms,
        attribution.parse_exec_dominant_subphase_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_post_parse_pre_materialization_ms,
        attribution.post_parse_pre_materialization_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_ready_install_ms,
        attribution.ready_install_ms,
    );
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_document_symbol_side_work_ms,
        attribution.document_symbol_side_work_ms,
    );
    if let Some(value) = attribution.dominant_phase {
        trace.followup_ready_snapshot_dominant_phase = Some(value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_dominant_phase_ms,
        attribution.dominant_phase_ms,
    );
}

fn merge_diagnostics_save_timeline_ready_snapshot_phase_attribution_inner(
    trace: &mut crate::types::DiagnosticsSaveTimelineTrace,
    attribution: diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2,
) {
    if trace.followup_ready_snapshot_timeout_phase.is_none() {
        trace.followup_ready_snapshot_timeout_phase =
            attribution.timeout_phase.map(|value| value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_timeout_phase_elapsed_ms,
        attribution.timeout_phase_elapsed_ms,
    );
    if trace.followup_ready_snapshot_timeout_leaf.is_none() {
        trace.followup_ready_snapshot_timeout_leaf =
            attribution.timeout_leaf.map(|value| value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_timeout_leaf_elapsed_ms,
        attribution.timeout_leaf_elapsed_ms,
    );
    if trace
        .followup_ready_snapshot_parse_exec_timeout_subphase
        .is_none()
    {
        trace.followup_ready_snapshot_parse_exec_timeout_subphase = attribution
            .parse_exec_timeout_subphase
            .map(|value| value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms,
        attribution.parse_exec_timeout_subphase_elapsed_ms,
    );
    if trace
        .followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint
        .is_none()
    {
        trace.followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint = attribution
            .parse_exec_core_build_timeout_checkpoint
            .map(|value| value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms,
        attribution.parse_exec_core_build_timeout_checkpoint_elapsed_ms,
    );
    if trace
        .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint
        .is_none()
    {
        trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint =
            attribution
                .parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint
                .map(|value| value.to_string());
    }
    update_followup_timing_max(
        &mut trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms,
        attribution
            .parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms,
    );
    overwrite_diagnostics_save_timeline_ready_snapshot_phase_attribution_view_inner(
        trace,
        attribution,
    );
}

fn set_diagnostics_save_timeline_followup_relief_valve_inner(
    trace: &mut crate::types::DiagnosticsSaveTimelineTrace,
    outcome: &'static str,
    budget: Duration,
    elapsed: Option<Duration>,
) {
    trace.followup_ready_snapshot_relief_valve_outcome = Some(outcome.to_string());
    trace.followup_ready_snapshot_relief_valve_budget_ms =
        Some(budget.as_millis().min(u64::MAX as u128) as u64);
    trace.followup_ready_snapshot_relief_valve_elapsed_ms =
        elapsed.map(|value| value.as_millis().min(u64::MAX as u128) as u64);
}

fn set_diagnostics_save_timeline_followup_ready_snapshot_continuation_inner(
    trace: &mut crate::types::DiagnosticsSaveTimelineTrace,
    reason: &'static str,
) {
    trace.followup_ready_snapshot_continuation_reason = Some(reason.to_string());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticsSaveTimelineFastlaneProgress {
    Pending,
    SuccessfulFirstPublish,
    TerminalWithoutPublish,
}

fn diagnostics_save_timeline_fastlane_progress_inner(
    store: &super::DiagnosticsSaveTimelineStore,
    key: super::DiagnosticsSaveTimelineCycleKey,
) -> DiagnosticsSaveTimelineFastlaneProgress {
    let Some(trace) = store.active_cycles.get(&key) else {
        return DiagnosticsSaveTimelineFastlaneProgress::Pending;
    };

    if trace.requested_version != key.requested_version {
        return DiagnosticsSaveTimelineFastlaneProgress::Pending;
    }

    if trace.save_fastlane_outcome.as_deref() == Some("published")
        && trace.first_publish.as_ref().is_some_and(|publish| {
            publish.profile == "save_fastlane" && publish.outcome == "published"
        })
    {
        return DiagnosticsSaveTimelineFastlaneProgress::SuccessfulFirstPublish;
    }

    if trace.save_fastlane_outcome.is_some() {
        return DiagnosticsSaveTimelineFastlaneProgress::TerminalWithoutPublish;
    }

    DiagnosticsSaveTimelineFastlaneProgress::Pending
}

fn append_completion_timeline_completed_stage(
    trace: &mut crate::types::CompletionTimelineTrace,
    name: &str,
    duration: Duration,
) {
    let duration_ms = duration.as_millis().min(u64::MAX as u128) as u64;
    if duration_ms == 0 {
        return;
    }

    let started_offset_ms = trace
        .stages
        .iter()
        .map(|stage| stage.started_offset_ms.saturating_add(stage.duration_ms))
        .max()
        .unwrap_or(0);
    trace
        .stages
        .push(crate::types::CompletionTimelineStageTrace {
            name: name.to_string(),
            status: "completed".to_string(),
            started_offset_ms,
            duration_ms,
        });
    trace.total_duration_ms = trace
        .total_duration_ms
        .max(started_offset_ms.saturating_add(duration_ms));
    trace.dominant_stage = trace
        .stages
        .iter()
        .filter(|stage| stage.status != "skipped")
        .max_by_key(|stage| stage.duration_ms)
        .map(|stage| stage.name.clone());
}

fn record_completion_timeline_trace_inner(
    traces: &StdMutex<VecDeque<crate::types::CompletionTimelineTrace>>,
    mut trace: crate::types::CompletionTimelineTrace,
) {
    let record_started = Instant::now();
    let mut traces = traces
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    append_completion_timeline_completed_stage(
        &mut trace,
        "handler_epilogue_trace_record",
        record_started.elapsed(),
    );
    traces.push_back(trace);
    while traces.len() > super::COMPLETION_TIMELINE_MAX_ENTRIES {
        let _ = traces.pop_front();
    }
}

fn record_completion_response_egress_patch_inner(
    traces: &StdMutex<VecDeque<crate::types::CompletionTimelineTrace>>,
    patch: &super::request_context::CompletionResponseEgressTracePatch,
) -> Option<super::CompletionResponseEgressDerivedTrace> {
    let mut traces = traces
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let trace = traces
        .iter_mut()
        .rfind(|trace| trace.request_id.as_deref() == Some(patch.request_id.as_str()))?;
    let server_edge_details = trace.server_edge_details.as_mut()?;
    if server_edge_details.response_flush_completed_at_ms.is_none() {
        let derived = super::derive_completion_response_egress_trace(
            super::CompletionResponseEgressTraceInputs {
                response_sent_at_ms: server_edge_details.response_sent_at_ms,
                response_output_handoff_started_at_ms: Some(
                    patch.response_output_handoff_started_at_ms,
                ),
                response_output_handoff_enqueued_at_ms: Some(
                    patch.response_output_handoff_enqueued_at_ms,
                ),
                response_output_enqueue_completed_at_ms: Some(
                    patch.response_output_enqueue_completed_at_ms,
                ),
                response_output_encode_started_at_ms: Some(
                    patch.response_output_encode_started_at_ms,
                ),
                response_output_write_started_at_ms: Some(
                    patch.response_output_write_started_at_ms,
                ),
                response_output_encode_completed_at_ms: Some(
                    patch.response_output_encode_completed_at_ms,
                ),
                response_flush_completed_at_ms: Some(patch.response_flush_completed_at_ms),
            },
        );
        server_edge_details.response_output_handoff_started_at_ms =
            Some(patch.response_output_handoff_started_at_ms);
        server_edge_details.response_output_handoff_enqueued_at_ms =
            Some(patch.response_output_handoff_enqueued_at_ms);
        server_edge_details.response_output_enqueue_completed_at_ms =
            Some(patch.response_output_enqueue_completed_at_ms);
        server_edge_details.response_output_encode_started_at_ms =
            Some(patch.response_output_encode_started_at_ms);
        server_edge_details.response_output_write_started_at_ms =
            Some(patch.response_output_write_started_at_ms);
        server_edge_details.response_output_encode_completed_at_ms =
            Some(patch.response_output_encode_completed_at_ms);
        server_edge_details.response_flush_completed_at_ms =
            Some(patch.response_flush_completed_at_ms);
        server_edge_details.response_ready_to_output_handoff_wait_ms =
            derived.response_ready_to_output_handoff_wait_ms;
        server_edge_details.response_output_handoff_send_wait_ms =
            derived.response_output_handoff_send_wait_ms;
        server_edge_details.response_output_handoff_to_writer_wait_ms =
            derived.response_output_handoff_to_writer_wait_ms;
        server_edge_details.response_ready_to_output_enqueue_wait_ms =
            derived.response_ready_to_output_enqueue_wait_ms;
        server_edge_details.response_output_queue_wait_ms = derived.response_output_queue_wait_ms;
        server_edge_details.response_output_encode_exec_ms = derived.response_output_encode_exec_ms;
        server_edge_details.response_output_write_and_flush_exec_ms =
            derived.response_output_write_and_flush_exec_ms;
        server_edge_details.response_ready_to_flush_wait_ms =
            derived.response_ready_to_flush_wait_ms;
        return Some(derived);
    }

    None
}

fn record_completion_response_egress_metrics(
    coordinator: &SystemCoordinator,
    derived: &super::CompletionResponseEgressDerivedTrace,
) {
    for (stage, value_ms) in [
        (
            "response_ready_to_output_handoff_wait",
            derived.response_ready_to_output_handoff_wait_ms,
        ),
        (
            "response_output_handoff_send_wait",
            derived.response_output_handoff_send_wait_ms,
        ),
        (
            "response_output_handoff_to_writer_wait",
            derived.response_output_handoff_to_writer_wait_ms,
        ),
        (
            "response_ready_to_output_enqueue_wait",
            derived.response_ready_to_output_enqueue_wait_ms,
        ),
        (
            "response_output_queue_wait",
            derived.response_output_queue_wait_ms,
        ),
        (
            "response_output_encode_exec",
            derived.response_output_encode_exec_ms,
        ),
        (
            "response_output_write_and_flush_exec",
            derived.response_output_write_and_flush_exec_ms,
        ),
        (
            "response_ready_to_flush_wait",
            derived.response_ready_to_flush_wait_ms,
        ),
    ] {
        if let Some(value_ms) = value_ms {
            coordinator.record_completion_stage_latency(stage, Duration::from_millis(value_ms));
        }
    }
}

fn build_pre_dispatch_terminal_completion_trace(
    input: super::request_context::PreDispatchCompletionTerminalTraceInput,
    trace_id: String,
) -> crate::types::CompletionTimelineTrace {
    let started_at_ms = input.adapter_read_at_ms.unwrap_or(input.resolved_at_ms);
    let queued_before_dispatch_ms = input.resolved_at_ms.saturating_sub(started_at_ms);
    let terminal_status = if input.outcome == "cancelled" {
        "cancelled"
    } else {
        "failed"
    };

    crate::types::CompletionTimelineTrace {
        trace_id,
        request_id: Some(input.request_id),
        client_probe_id: input.client_probe_id,
        uri: input.uri,
        trigger_mode: input.trigger_mode,
        outcome: input.outcome,
        started_at_ms,
        total_duration_ms: queued_before_dispatch_ms,
        dominant_stage: Some("queued_before_dispatch".to_string()),
        prepare_details: None,
        collect_breakdown: None,
        server_edge_details: None,
        turn_attribution: None,
        stages: vec![
            crate::types::CompletionTimelineStageTrace {
                name: "queued_before_dispatch".to_string(),
                status: terminal_status.to_string(),
                started_offset_ms: 0,
                duration_ms: queued_before_dispatch_ms,
            },
            crate::types::CompletionTimelineStageTrace {
                name: "terminal".to_string(),
                status: terminal_status.to_string(),
                started_offset_ms: queued_before_dispatch_ms,
                duration_ms: 0,
            },
        ],
    }
}

#[cfg(test)]
pub(crate) fn validate_scale_aware_baseline_schema_for_acceptance(
    baseline_report: &serde_json::Value,
) -> Result<(), String> {
    bsl_backend::perf_gate_evaluator::validate_scale_aware_baseline_schema(baseline_report)
}

#[cfg(test)]
pub(crate) fn evaluate_scale_aware_gate_for_acceptance(
    current_report: &serde_json::Value,
    baseline_report: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    bsl_backend::perf_gate_evaluator::evaluate_scale_aware_gate(current_report, baseline_report)
}

impl BslLanguageServer {
    pub fn new(client: Client, coordinator: Arc<SystemCoordinator>) -> Self {
        let default_settings = BslSettings::default();
        let default_diagnostics_detail_level =
            bsl_shared::formatting::DetailLevel::parse(&default_settings.diagnostics.detail_level);

        let mut analysis_host_v2 = AnalysisHostV2::default();
        let initial_deps_bundle =
            build_deps_bundle_v2(&coordinator, None, None).unwrap_or_else(|err| {
                warn!("Failed to build initial deps bundle v2: {}", err);

                let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
                let signature_index = repository.get_signature_index_clone();
                let resolver = Some(Arc::new(TypeResolver::new(repository.clone())));

                let semantic_deps = Arc::new(bsl_analysis_v2::SemanticDeps {
                    repository,
                    signature_index,
                    resolver,
                    platform_signatures_loaded: false,
                });

                let index_snapshot = Arc::new(coordinator.intellisense_index().snapshot());
                let index_snapshot_id = index_snapshot.id.as_str().to_string();

                DepsBundleV2 {
                    deps_id: DepsSnapshotId::from_hash(""),
                    semantic_deps,
                    index_snapshot,
                    meta: DepsBundleV2Meta {
                        platform_version: env!("CARGO_PKG_VERSION").to_string(),
                        platform_fingerprint: None,
                        config_fingerprint: None,
                        index_snapshot_id,
                        strict_fingerprint: false,
                    },
                }
            });
        let initial_deps_id = initial_deps_bundle.deps_id.clone();
        analysis_host_v2.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
            deps_id: initial_deps_id.clone(),
            deps: initial_deps_bundle.semantic_deps.clone(),
        });
        let initial_settings_id = compute_settings_id_v2(&default_settings);
        analysis_host_v2.apply_change(bsl_analysis_v2::Change::SetSettingsSnapshot {
            settings_id: initial_settings_id.clone(),
            diagnostics_detail_level: default_diagnostics_detail_level,
        });
        let analysis_v2 = AnalysisV2Runtime::new(
            analysis_host_v2,
            initial_deps_bundle.index_snapshot.clone(),
            Some(coordinator.clone()),
        );
        let completion_pipeline_knobs =
            bsl_runtime::application::CompletionPipelineKnobs::from_runtime_config();
        let completion_dispatcher_v2 = Arc::new(
            super::completion_dispatcher::CompletionDispatcherRegistry::new(
                completion_pipeline_knobs.queue_capacity,
            ),
        );
        let completion_cancellation_registry_v2 =
            Arc::new(super::completion_cancellation::CompletionCancellationRegistry::default());
        let completion_timeline_traces = Arc::new(StdMutex::new(VecDeque::new()));
        let next_completion_timeline_trace_id = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let diagnostics_save_timeline_store =
            Arc::new(StdMutex::new(super::DiagnosticsSaveTimelineStore::default()));
        let next_diagnostics_save_timeline_trace_id =
            Arc::new(std::sync::atomic::AtomicU64::new(1));
        let did_change_parse_snapshot_evidence_store = Arc::new(StdMutex::new(
            super::DidChangeParseSnapshotEvidenceStore::default(),
        ));
        let next_did_change_parse_snapshot_evidence_id =
            Arc::new(std::sync::atomic::AtomicU64::new(1));

        let server = Self {
            client,
            diagnostics_counts: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(default_settings)),
            completion_snippet_support: Arc::new(RwLock::new(false)),
            auto_reindex_paused: Arc::new(RwLock::new(false)),
            coordinator,
            formatting_capability: Arc::new(RwLock::new(FormattingCapabilityState::default())),
            inlay_hints_capability: Arc::new(RwLock::new(InlayHintsCapabilityState::default())),
            code_actions_capability: Arc::new(RwLock::new(CodeActionsCapabilityState::default())),

            analysis_v2,
            text_sync_v2: Arc::new(Mutex::new(())),
            file_key_to_file_id_v2: Arc::new(RwLock::new(HashMap::new())),
            file_id_to_uri_v2: Arc::new(RwLock::new(HashMap::new())),
            next_file_id_v2: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            diagnostics_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            type_index_precompute_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            current_revision_head_precompute_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            background_parse_snapshot_apply_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            document_symbol_bootstrap_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            diagnostics_generation_v2: Arc::new(RwLock::new(HashMap::new())),
            diagnostics_save_cycle_sequence_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_received_file_versions_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_current_revision_handoff_versions_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_document_shadow_state_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_ready_parse_snapshots_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_detached_diagnostics_ready_artifacts_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_snapshot_failures_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_snapshot_status_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_save_fastlane_syntax_artifacts_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_apply_enqueued_at_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_diagnostics_publish_state_v2: Arc::new(RwLock::new(HashMap::new())),
            scale_aware_churn_state_v2: Arc::new(RwLock::new(HashMap::new())),
            document_symbol_ready_cache_v2: Arc::new(RwLock::new(HashMap::new())),
            document_symbol_request_epochs_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_seen_files_v2: Arc::new(RwLock::new(std::collections::HashSet::new())),
            completion_parity_state_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_head_serve_observations_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_dispatcher_v2,
            completion_cancellation_registry_v2,
            last_deps_id_v2: Arc::new(RwLock::new(Some(initial_deps_id))),
            last_settings_id_v2: Arc::new(RwLock::new(Some(initial_settings_id))),
            full_index_state: Arc::new(Mutex::new(super::FullIndexRuntimeState::default())),
            next_full_index_operation_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            full_index_watchdog_timeout: Duration::from_millis(1_200_000),
            current_context_latest_generations: Arc::new(StdMutex::new(HashMap::new())),
            current_context_generation_notify: Arc::new(tokio::sync::Notify::new()),
            current_context_parse_broker: Arc::new(StdMutex::new(HashMap::new())),
            completion_timeline_traces: completion_timeline_traces.clone(),
            next_completion_timeline_trace_id: next_completion_timeline_trace_id.clone(),
            diagnostics_save_timeline_store: diagnostics_save_timeline_store.clone(),
            did_change_parse_snapshot_evidence_store: did_change_parse_snapshot_evidence_store
                .clone(),
            diagnostics_did_save_followup_lane_v2: Arc::new(StdMutex::new(
                super::DiagnosticsDidSaveFollowupLaneStateV2::default(),
            )),
            diagnostics_did_save_followup_lane_notify_v2: Arc::new(tokio::sync::Notify::new()),
            next_diagnostics_save_timeline_trace_id: next_diagnostics_save_timeline_trace_id
                .clone(),
            next_did_change_parse_snapshot_evidence_id: next_did_change_parse_snapshot_evidence_id
                .clone(),
            next_document_symbol_request_epoch_v2: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            next_type_index_precompute_task_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        };

        let cancellation_registry_weak =
            Arc::downgrade(&server.completion_cancellation_registry_v2);
        let dispatcher_weak = Arc::downgrade(&server.completion_dispatcher_v2);
        super::request_context::set_cancel_request_hook(Some(Arc::new(move |request_id| {
            let Some(registry) = cancellation_registry_weak.upgrade() else {
                return;
            };
            let Some(dispatcher) = dispatcher_weak.upgrade() else {
                return;
            };
            let Some(entry) = registry.cancel_request(&request_id) else {
                return;
            };
            tokio::spawn(async move {
                let file_id = entry.file_id;
                let cancelled_request_epoch = entry.request_epoch;
                let _ = dispatcher
                    .cancel_pre_active_completion(file_id, cancelled_request_epoch)
                    .await;
                let ticket = dispatcher.emit_cancel(file_id, request_id.clone()).await;
                if matches!(
                    ticket.queue_outcome,
                    super::completion_dispatcher::QueueEnqueueOutcome::Full
                        | super::completion_dispatcher::QueueEnqueueOutcome::Closed
                ) {
                    debug!(
                        file_id = file_id.0,
                        file_seq = ticket.file_seq,
                        request_epoch = ticket.request_epoch,
                        cancelled_request_epoch,
                        request_id = %request_id,
                        queue_outcome = ?ticket.queue_outcome,
                        "completion dispatcher dropped cancel event"
                    );
                }
            });
        })));

        let completion_timeline_traces_for_hook = completion_timeline_traces.clone();
        let next_completion_timeline_trace_id_for_hook = next_completion_timeline_trace_id.clone();
        let coordinator_for_hook = server.coordinator.clone();
        super::request_context::set_pre_dispatch_completion_terminal_hook(Some(Arc::new(
            move |input| {
                let completion_timeline_traces = completion_timeline_traces_for_hook.clone();
                let next_completion_timeline_trace_id =
                    next_completion_timeline_trace_id_for_hook.clone();
                let coordinator = coordinator_for_hook.clone();
                tokio::spawn(async move {
                    let trace = build_pre_dispatch_terminal_completion_trace(
                        input,
                        next_completion_timeline_trace_id_from(
                            next_completion_timeline_trace_id.as_ref(),
                        ),
                    );
                    let public_outcome = match trace.outcome.as_str() {
                        "queue_rejected" => "fail_closed".to_string(),
                        other => other.to_string(),
                    };
                    record_completion_timeline_trace_inner(
                        completion_timeline_traces.as_ref(),
                        trace,
                    );
                    coordinator.record_intellisense_v2_completion_outcome(&public_outcome);
                });
            },
        )));

        let completion_timeline_traces_for_egress_hook = completion_timeline_traces.clone();
        let coordinator_for_egress_hook = server.coordinator.clone();
        super::request_context::set_completion_response_egress_hook(Some(Arc::new(move |patch| {
            if let Some(derived) = record_completion_response_egress_patch_inner(
                completion_timeline_traces_for_egress_hook.as_ref(),
                &patch,
            ) {
                record_completion_response_egress_metrics(
                    coordinator_for_egress_hook.as_ref(),
                    &derived,
                );
            }
        })));

        server
    }

    pub(crate) fn next_completion_timeline_trace_id(&self) -> String {
        next_completion_timeline_trace_id_from(self.next_completion_timeline_trace_id.as_ref())
    }

    pub(crate) fn record_completion_timeline_trace(
        &self,
        trace: crate::types::CompletionTimelineTrace,
    ) {
        record_completion_timeline_trace_inner(self.completion_timeline_traces.as_ref(), trace);
    }

    pub(crate) fn begin_diagnostics_save_timeline_cycle(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
    ) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if diagnostics_save_timeline_terminal_key_is_recorded_inner(&store, key) {
            return;
        }
        store.active_cycles.entry(key).or_insert_with(|| {
            crate::types::DiagnosticsSaveTimelineTrace {
                trace_id: next_diagnostics_save_timeline_trace_id_from(
                    self.next_diagnostics_save_timeline_trace_id.as_ref(),
                ),
                uri: uri.to_string(),
                requested_version: key.requested_version,
                diagnostics_generation: key.diagnostics_generation,
                save_cycle_sequence: key.save_cycle_sequence,
                trigger: bsl_runtime::application::DiagnosticsTrigger::DidSave
                    .as_str()
                    .to_string(),
                started_at_ms: super::unix_timestamp_ms(),
                first_publish: None,
                followup_publish: None,
                save_fastlane_outcome: None,
                idle_heavy_outcome: None,
                followup_syntax_work_mode: None,
                followup_semantic_path: None,
                followup_semantic_parse_source: None,
                followup_semantic_ir_source: None,
                followup_semantic_materialization_path: None,
                followup_ready_snapshot_zero_probe: None,
                followup_ready_snapshot_wait_probe: None,
                followup_ready_snapshot_task_state: None,
                followup_ready_snapshot_timeout_phase: None,
                followup_ready_snapshot_timeout_phase_elapsed_ms: None,
                followup_ready_snapshot_timeout_leaf: None,
                followup_ready_snapshot_timeout_leaf_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_ms: None,
                followup_ready_snapshot_parse_exec_timeout_subphase: None,
                followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_parse_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
                followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: None,
                followup_ready_snapshot_parse_exec_dominant_subphase: None,
                followup_ready_snapshot_parse_exec_dominant_subphase_ms: None,
                followup_ready_snapshot_post_parse_pre_materialization_ms: None,
                followup_ready_snapshot_ready_install_ms: None,
                followup_ready_snapshot_document_symbol_side_work_ms: None,
                followup_ready_snapshot_dominant_phase: None,
                followup_ready_snapshot_dominant_phase_ms: None,
                followup_ready_snapshot_relief_valve_outcome: None,
                followup_ready_snapshot_relief_valve_budget_ms: None,
                followup_ready_snapshot_relief_valve_elapsed_ms: None,
                followup_ready_snapshot_continuation_reason: None,
                followup_shadow_state_available: None,
                followup_wait_reason: None,
                followup_blocker_reason: None,
                followup_runtime_queue_wait_ms: None,
                followup_apply_lag_ms: None,
                followup_wait_for_file_version_ms: None,
                followup_snapshot_with_deps_ms: None,
                terminal_outcome: None,
            }
        });
    }

    pub(crate) fn record_diagnostics_save_timeline_profile_result(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
        result: super::DiagnosticsSaveTimelineProfileResult,
    ) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if diagnostics_save_timeline_terminal_key_is_recorded_inner(&store, key) {
            return;
        }
        let trace_completed = {
            let trace = store.active_cycles.entry(key).or_insert_with(|| {
                crate::types::DiagnosticsSaveTimelineTrace {
                    trace_id: next_diagnostics_save_timeline_trace_id_from(
                        self.next_diagnostics_save_timeline_trace_id.as_ref(),
                    ),
                    uri: uri.to_string(),
                    requested_version: key.requested_version,
                    diagnostics_generation: key.diagnostics_generation,
                    save_cycle_sequence: key.save_cycle_sequence,
                    trigger: bsl_runtime::application::DiagnosticsTrigger::DidSave
                        .as_str()
                        .to_string(),
                    started_at_ms: super::unix_timestamp_ms(),
                    first_publish: None,
                    followup_publish: None,
                    save_fastlane_outcome: None,
                    idle_heavy_outcome: None,
                    followup_syntax_work_mode: None,
                    followup_semantic_path: None,
                    followup_semantic_parse_source: None,
                    followup_semantic_ir_source: None,
                    followup_semantic_materialization_path: None,
                    followup_ready_snapshot_zero_probe: None,
                    followup_ready_snapshot_wait_probe: None,
                    followup_ready_snapshot_task_state: None,
                    followup_ready_snapshot_timeout_phase: None,
                    followup_ready_snapshot_timeout_phase_elapsed_ms: None,
                    followup_ready_snapshot_timeout_leaf: None,
                    followup_ready_snapshot_timeout_leaf_elapsed_ms: None,
                    followup_ready_snapshot_parse_exec_ms: None,
                    followup_ready_snapshot_parse_exec_timeout_subphase: None,
                    followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: None,
                    followup_ready_snapshot_parse_exec_core_parse_build_ms: None,
                    followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: None,
                    followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: None,
                    followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: None,
                    followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                        None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                        None,
                    followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
                    followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
                    followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: None,
                    followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: None,
                    followup_ready_snapshot_parse_exec_dominant_subphase: None,
                    followup_ready_snapshot_parse_exec_dominant_subphase_ms: None,
                    followup_ready_snapshot_post_parse_pre_materialization_ms: None,
                    followup_ready_snapshot_ready_install_ms: None,
                    followup_ready_snapshot_document_symbol_side_work_ms: None,
                    followup_ready_snapshot_dominant_phase: None,
                    followup_ready_snapshot_dominant_phase_ms: None,
                    followup_ready_snapshot_relief_valve_outcome: None,
                    followup_ready_snapshot_relief_valve_budget_ms: None,
                    followup_ready_snapshot_relief_valve_elapsed_ms: None,
                    followup_ready_snapshot_continuation_reason: None,
                    followup_shadow_state_available: None,
                    followup_wait_reason: None,
                    followup_blocker_reason: None,
                    followup_runtime_queue_wait_ms: None,
                    followup_apply_lag_ms: None,
                    followup_wait_for_file_version_ms: None,
                    followup_snapshot_with_deps_ms: None,
                    terminal_outcome: None,
                }
            });

            match result.profile {
                bsl_runtime::application::DiagnosticsProfile::SaveFastlane => {
                    trace.save_fastlane_outcome = Some(result.disposition.as_str().to_string());
                }
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy => {
                    trace.idle_heavy_outcome = Some(result.disposition.as_str().to_string());
                }
                _ => {}
            }

            if let Some(publish) = result.publish {
                if matches!(
                    result.profile,
                    bsl_runtime::application::DiagnosticsProfile::IdleHeavy
                ) {
                    trace.followup_syntax_work_mode = publish.syntax_work_mode.clone();
                    if publish.semantic_path.is_some() {
                        trace.followup_semantic_path = publish.semantic_path.clone();
                    }
                    if publish.semantic_parse_source.is_some() {
                        trace.followup_semantic_parse_source =
                            publish.semantic_parse_source.clone();
                    }
                    if publish.semantic_ir_source.is_some() {
                        trace.followup_semantic_ir_source = publish.semantic_ir_source.clone();
                    }
                    if publish.semantic_materialization_path.is_some() {
                        trace.followup_semantic_materialization_path =
                            publish.semantic_materialization_path.clone();
                    }
                    update_followup_timing_max(
                        &mut trace.followup_runtime_queue_wait_ms,
                        publish.runtime_queue_wait_ms,
                    );
                    update_followup_timing_max(
                        &mut trace.followup_apply_lag_ms,
                        publish.apply_lag_ms,
                    );
                }
                if trace.first_publish.is_none() {
                    trace.first_publish = Some(publish);
                } else if trace.followup_publish.is_none() {
                    trace.followup_publish = Some(publish);
                }
            }

            if matches!(
                result.profile,
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy
            ) {
                clear_diagnostics_save_timeline_followup_wait_inner(trace);
            }

            trace.terminal_outcome = diagnostics_save_timeline_cycle_terminal_outcome(trace);
            trace.terminal_outcome.is_some()
        };
        if trace_completed {
            let Some(trace) = store.active_cycles.remove(&key) else {
                return;
            };
            remember_diagnostics_save_timeline_terminal_key_inner(&mut store, key);
            archive_diagnostics_save_timeline_trace_inner(&mut store, trace);
        }
    }

    pub(crate) fn record_diagnostics_save_timeline_profile_disposition(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
        profile: bsl_runtime::application::DiagnosticsProfile,
        disposition: bsl_runtime::application::DiagnosticsDisposition,
    ) {
        self.record_diagnostics_save_timeline_profile_result(
            uri,
            key,
            super::DiagnosticsSaveTimelineProfileResult {
                profile,
                disposition,
                publish: None,
            },
        );
    }

    pub(crate) fn record_diagnostics_save_timeline_followup_probe_state(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
        ready_snapshot_zero_probe: Option<&'static str>,
        ready_snapshot_wait_probe: Option<&'static str>,
        ready_snapshot_task_state: Option<&'static str>,
        shadow_state_available: Option<bool>,
        ready_snapshot_phase_attribution: Option<
            diagnostics_runtime::DiagnosticsReadySnapshotPhaseAttributionV2,
        >,
    ) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if diagnostics_save_timeline_terminal_key_is_recorded_inner(&store, key) {
            return;
        }
        let trace = store.active_cycles.entry(key).or_insert_with(|| {
            crate::types::DiagnosticsSaveTimelineTrace {
                trace_id: next_diagnostics_save_timeline_trace_id_from(
                    self.next_diagnostics_save_timeline_trace_id.as_ref(),
                ),
                uri: uri.to_string(),
                requested_version: key.requested_version,
                diagnostics_generation: key.diagnostics_generation,
                save_cycle_sequence: key.save_cycle_sequence,
                trigger: bsl_runtime::application::DiagnosticsTrigger::DidSave
                    .as_str()
                    .to_string(),
                started_at_ms: super::unix_timestamp_ms(),
                first_publish: None,
                followup_publish: None,
                save_fastlane_outcome: None,
                    idle_heavy_outcome: None,
                    followup_syntax_work_mode: None,
                    followup_semantic_path: None,
                    followup_semantic_parse_source: None,
                    followup_semantic_ir_source: None,
                    followup_semantic_materialization_path: None,
                    followup_ready_snapshot_zero_probe: None,
                followup_ready_snapshot_wait_probe: None,
                followup_ready_snapshot_task_state: None,
                followup_ready_snapshot_timeout_phase: None,
                followup_ready_snapshot_timeout_phase_elapsed_ms: None,
                followup_ready_snapshot_timeout_leaf: None,
                followup_ready_snapshot_timeout_leaf_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_ms: None,
                followup_ready_snapshot_parse_exec_timeout_subphase: None,
                followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_parse_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
                followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: None,
                followup_ready_snapshot_parse_exec_dominant_subphase: None,
                followup_ready_snapshot_parse_exec_dominant_subphase_ms: None,
                followup_ready_snapshot_post_parse_pre_materialization_ms: None,
                followup_ready_snapshot_ready_install_ms: None,
                followup_ready_snapshot_document_symbol_side_work_ms: None,
                followup_ready_snapshot_dominant_phase: None,
                followup_ready_snapshot_dominant_phase_ms: None,
                followup_ready_snapshot_relief_valve_outcome: None,
                followup_ready_snapshot_relief_valve_budget_ms: None,
                followup_ready_snapshot_relief_valve_elapsed_ms: None,
                followup_ready_snapshot_continuation_reason: None,
                followup_shadow_state_available: None,
                followup_wait_reason: None,
                followup_blocker_reason: None,
                followup_runtime_queue_wait_ms: None,
                followup_apply_lag_ms: None,
                followup_wait_for_file_version_ms: None,
                followup_snapshot_with_deps_ms: None,
                terminal_outcome: None,
            }
        });
        if let Some(outcome) = ready_snapshot_zero_probe {
            trace.followup_ready_snapshot_zero_probe = Some(outcome.to_string());
        }
        if let Some(outcome) = ready_snapshot_wait_probe {
            trace.followup_ready_snapshot_wait_probe = Some(outcome.to_string());
        }
        if trace.followup_ready_snapshot_task_state.is_none() {
            if let Some(task_state) = ready_snapshot_task_state {
                trace.followup_ready_snapshot_task_state = Some(task_state.to_string());
            }
        }
        if trace.followup_shadow_state_available.is_none() {
            trace.followup_shadow_state_available = shadow_state_available;
        }
        if let Some(attribution) = ready_snapshot_phase_attribution {
            merge_diagnostics_save_timeline_ready_snapshot_phase_attribution_inner(
                trace,
                attribution,
            );
            if diagnostics_save_coherence_debug_enabled() {
                let stored_program_conversion = trace
                    .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms;
                let stored_program_lowering = trace
                    .followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms;
                emit_diagnostics_save_coherence_debug(format!(
                    "[diag-save-coherence][timeline] trace_id={} requested_version={} save_cycle_sequence={} incoming_program_conversion_ms={:?} incoming_program_lowering_ms={:?} incoming_packaging_ms={:?} incoming_timeout_checkpoint={:?} stored_program_conversion_ms={:?} stored_program_lowering_ms={:?} stored_packaging_ms={:?} stored_timeout_checkpoint={:?} incoherent={}",
                    trace.trace_id,
                    trace.requested_version,
                    trace.save_cycle_sequence,
                    attribution.parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms,
                    attribution.parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms,
                    attribution
                        .parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms,
                    attribution
                        .parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint,
                    stored_program_conversion,
                    stored_program_lowering,
                    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms,
                    trace.followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint,
                    stored_program_conversion
                        .zip(stored_program_lowering)
                        .is_some_and(|(program_conversion, program_lowering)| program_conversion < program_lowering),
                ));
            }
        }
    }

    pub(crate) fn record_diagnostics_save_timeline_followup_relief_valve(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
        outcome: &'static str,
        budget: Duration,
        elapsed: Option<Duration>,
    ) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(trace) = diagnostics_save_timeline_trace_mut_inner(&mut store, uri, key) {
            set_diagnostics_save_timeline_followup_relief_valve_inner(
                trace, outcome, budget, elapsed,
            );
            return;
        }
        if diagnostics_save_timeline_terminal_key_is_recorded_inner(&store, key) {
            return;
        }
        let trace = store.active_cycles.entry(key).or_insert_with(|| {
            crate::types::DiagnosticsSaveTimelineTrace {
                trace_id: next_diagnostics_save_timeline_trace_id_from(
                    self.next_diagnostics_save_timeline_trace_id.as_ref(),
                ),
                uri: uri.to_string(),
                requested_version: key.requested_version,
                diagnostics_generation: key.diagnostics_generation,
                save_cycle_sequence: key.save_cycle_sequence,
                trigger: bsl_runtime::application::DiagnosticsTrigger::DidSave
                    .as_str()
                    .to_string(),
                started_at_ms: super::unix_timestamp_ms(),
                first_publish: None,
                followup_publish: None,
                save_fastlane_outcome: None,
                idle_heavy_outcome: None,
                followup_syntax_work_mode: None,
                followup_semantic_path: None,
                followup_semantic_parse_source: None,
                followup_semantic_ir_source: None,
                followup_semantic_materialization_path: None,
                followup_ready_snapshot_zero_probe: None,
                followup_ready_snapshot_wait_probe: None,
                followup_ready_snapshot_task_state: None,
                followup_ready_snapshot_timeout_phase: None,
                followup_ready_snapshot_timeout_phase_elapsed_ms: None,
                followup_ready_snapshot_timeout_leaf: None,
                followup_ready_snapshot_timeout_leaf_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_ms: None,
                followup_ready_snapshot_parse_exec_timeout_subphase: None,
                followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_parse_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
                followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: None,
                followup_ready_snapshot_parse_exec_dominant_subphase: None,
                followup_ready_snapshot_parse_exec_dominant_subphase_ms: None,
                followup_ready_snapshot_post_parse_pre_materialization_ms: None,
                followup_ready_snapshot_ready_install_ms: None,
                followup_ready_snapshot_document_symbol_side_work_ms: None,
                followup_ready_snapshot_dominant_phase: None,
                followup_ready_snapshot_dominant_phase_ms: None,
                followup_ready_snapshot_relief_valve_outcome: None,
                followup_ready_snapshot_relief_valve_budget_ms: None,
                followup_ready_snapshot_relief_valve_elapsed_ms: None,
                followup_ready_snapshot_continuation_reason: None,
                followup_shadow_state_available: None,
                followup_wait_reason: None,
                followup_blocker_reason: None,
                followup_runtime_queue_wait_ms: None,
                followup_apply_lag_ms: None,
                followup_wait_for_file_version_ms: None,
                followup_snapshot_with_deps_ms: None,
                terminal_outcome: None,
            }
        });
        set_diagnostics_save_timeline_followup_relief_valve_inner(trace, outcome, budget, elapsed);
    }

    pub(crate) fn record_diagnostics_save_timeline_followup_ready_snapshot_continuation(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
        reason: &'static str,
    ) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(trace) = diagnostics_save_timeline_trace_mut_inner(&mut store, uri, key) {
            set_diagnostics_save_timeline_followup_ready_snapshot_continuation_inner(trace, reason);
            return;
        }
        if diagnostics_save_timeline_terminal_key_is_recorded_inner(&store, key) {
            return;
        }
        let trace = store.active_cycles.entry(key).or_insert_with(|| {
            crate::types::DiagnosticsSaveTimelineTrace {
                trace_id: next_diagnostics_save_timeline_trace_id_from(
                    self.next_diagnostics_save_timeline_trace_id.as_ref(),
                ),
                uri: uri.to_string(),
                requested_version: key.requested_version,
                diagnostics_generation: key.diagnostics_generation,
                save_cycle_sequence: key.save_cycle_sequence,
                trigger: bsl_runtime::application::DiagnosticsTrigger::DidSave
                    .as_str()
                    .to_string(),
                started_at_ms: super::unix_timestamp_ms(),
                first_publish: None,
                followup_publish: None,
                save_fastlane_outcome: None,
                idle_heavy_outcome: None,
                followup_syntax_work_mode: None,
                followup_semantic_path: None,
                followup_semantic_parse_source: None,
                followup_semantic_ir_source: None,
                followup_semantic_materialization_path: None,
                followup_ready_snapshot_zero_probe: None,
                followup_ready_snapshot_wait_probe: None,
                followup_ready_snapshot_task_state: None,
                followup_ready_snapshot_timeout_phase: None,
                followup_ready_snapshot_timeout_phase_elapsed_ms: None,
                followup_ready_snapshot_timeout_leaf: None,
                followup_ready_snapshot_timeout_leaf_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_ms: None,
                followup_ready_snapshot_parse_exec_timeout_subphase: None,
                followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_parse_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
                followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: None,
                followup_ready_snapshot_parse_exec_dominant_subphase: None,
                followup_ready_snapshot_parse_exec_dominant_subphase_ms: None,
                followup_ready_snapshot_post_parse_pre_materialization_ms: None,
                followup_ready_snapshot_ready_install_ms: None,
                followup_ready_snapshot_document_symbol_side_work_ms: None,
                followup_ready_snapshot_dominant_phase: None,
                followup_ready_snapshot_dominant_phase_ms: None,
                followup_ready_snapshot_relief_valve_outcome: None,
                followup_ready_snapshot_relief_valve_budget_ms: None,
                followup_ready_snapshot_relief_valve_elapsed_ms: None,
                followup_ready_snapshot_continuation_reason: None,
                followup_shadow_state_available: None,
                followup_wait_reason: None,
                followup_blocker_reason: None,
                followup_runtime_queue_wait_ms: None,
                followup_apply_lag_ms: None,
                followup_wait_for_file_version_ms: None,
                followup_snapshot_with_deps_ms: None,
                terminal_outcome: None,
            }
        });
        set_diagnostics_save_timeline_followup_ready_snapshot_continuation_inner(trace, reason);
    }

    pub(crate) fn record_diagnostics_save_timeline_followup_wait_state(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
        reason: &'static str,
        runtime_queue_wait_ms: Option<Duration>,
        apply_lag_ms: Option<Duration>,
        wait_for_file_version_ms: Option<Duration>,
        snapshot_with_deps_ms: Option<Duration>,
        syntax_work_mode: Option<&'static str>,
        semantic_path: Option<&'static str>,
        semantic_parse_source: Option<&'static str>,
        semantic_ir_source: Option<&'static str>,
        semantic_materialization_path: Option<&'static str>,
    ) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if diagnostics_save_timeline_terminal_key_is_recorded_inner(&store, key) {
            return;
        }
        let trace = store.active_cycles.entry(key).or_insert_with(|| {
            crate::types::DiagnosticsSaveTimelineTrace {
                trace_id: next_diagnostics_save_timeline_trace_id_from(
                    self.next_diagnostics_save_timeline_trace_id.as_ref(),
                ),
                uri: uri.to_string(),
                requested_version: key.requested_version,
                diagnostics_generation: key.diagnostics_generation,
                save_cycle_sequence: key.save_cycle_sequence,
                trigger: bsl_runtime::application::DiagnosticsTrigger::DidSave
                    .as_str()
                    .to_string(),
                started_at_ms: super::unix_timestamp_ms(),
                first_publish: None,
                followup_publish: None,
                save_fastlane_outcome: None,
                idle_heavy_outcome: None,
                followup_syntax_work_mode: None,
                followup_semantic_path: None,
                followup_semantic_parse_source: None,
                followup_semantic_ir_source: None,
                followup_semantic_materialization_path: None,
                followup_ready_snapshot_zero_probe: None,
                followup_ready_snapshot_wait_probe: None,
                followup_ready_snapshot_task_state: None,
                followup_ready_snapshot_timeout_phase: None,
                followup_ready_snapshot_timeout_phase_elapsed_ms: None,
                followup_ready_snapshot_timeout_leaf: None,
                followup_ready_snapshot_timeout_leaf_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_ms: None,
                followup_ready_snapshot_parse_exec_timeout_subphase: None,
                followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_parse_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
                followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: None,
                followup_ready_snapshot_parse_exec_dominant_subphase: None,
                followup_ready_snapshot_parse_exec_dominant_subphase_ms: None,
                followup_ready_snapshot_post_parse_pre_materialization_ms: None,
                followup_ready_snapshot_ready_install_ms: None,
                followup_ready_snapshot_document_symbol_side_work_ms: None,
                followup_ready_snapshot_dominant_phase: None,
                followup_ready_snapshot_dominant_phase_ms: None,
                followup_ready_snapshot_relief_valve_outcome: None,
                followup_ready_snapshot_relief_valve_budget_ms: None,
                followup_ready_snapshot_relief_valve_elapsed_ms: None,
                followup_ready_snapshot_continuation_reason: None,
                followup_shadow_state_available: None,
                followup_wait_reason: None,
                followup_blocker_reason: None,
                followup_runtime_queue_wait_ms: None,
                followup_apply_lag_ms: None,
                followup_wait_for_file_version_ms: None,
                followup_snapshot_with_deps_ms: None,
                terminal_outcome: None,
            }
        });
        if let Some(syntax_work_mode) = syntax_work_mode {
            trace.followup_syntax_work_mode = Some(syntax_work_mode.to_string());
        }
        if let Some(semantic_path) = semantic_path {
            trace.followup_semantic_path = Some(semantic_path.to_string());
        }
        if let Some(semantic_parse_source) = semantic_parse_source {
            trace.followup_semantic_parse_source = Some(semantic_parse_source.to_string());
        }
        if let Some(semantic_ir_source) = semantic_ir_source {
            trace.followup_semantic_ir_source = Some(semantic_ir_source.to_string());
        }
        if let Some(semantic_materialization_path) = semantic_materialization_path {
            trace.followup_semantic_materialization_path =
                Some(semantic_materialization_path.to_string());
        }
        trace.followup_wait_reason = Some(reason.to_string());
        update_followup_timing_max(
            &mut trace.followup_runtime_queue_wait_ms,
            duration_to_nonzero_ms(runtime_queue_wait_ms),
        );
        update_followup_timing_max(
            &mut trace.followup_apply_lag_ms,
            duration_to_nonzero_ms(apply_lag_ms),
        );
        trace.followup_wait_for_file_version_ms =
            wait_for_file_version_ms.map(|value| value.as_millis().min(u64::MAX as u128) as u64);
        trace.followup_snapshot_with_deps_ms =
            snapshot_with_deps_ms.map(|value| value.as_millis().min(u64::MAX as u128) as u64);
    }

    pub(crate) fn record_diagnostics_save_timeline_followup_blocker_reason(
        &self,
        uri: &Url,
        key: super::DiagnosticsSaveTimelineCycleKey,
        reason: &'static str,
    ) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if diagnostics_save_timeline_terminal_key_is_recorded_inner(&store, key) {
            return;
        }
        let trace = store.active_cycles.entry(key).or_insert_with(|| {
            crate::types::DiagnosticsSaveTimelineTrace {
                trace_id: next_diagnostics_save_timeline_trace_id_from(
                    self.next_diagnostics_save_timeline_trace_id.as_ref(),
                ),
                uri: uri.to_string(),
                requested_version: key.requested_version,
                diagnostics_generation: key.diagnostics_generation,
                save_cycle_sequence: key.save_cycle_sequence,
                trigger: bsl_runtime::application::DiagnosticsTrigger::DidSave
                    .as_str()
                    .to_string(),
                started_at_ms: super::unix_timestamp_ms(),
                first_publish: None,
                followup_publish: None,
                save_fastlane_outcome: None,
                idle_heavy_outcome: None,
                followup_syntax_work_mode: None,
                followup_semantic_path: None,
                followup_semantic_parse_source: None,
                followup_semantic_ir_source: None,
                followup_semantic_materialization_path: None,
                followup_ready_snapshot_zero_probe: None,
                followup_ready_snapshot_wait_probe: None,
                followup_ready_snapshot_task_state: None,
                followup_ready_snapshot_timeout_phase: None,
                followup_ready_snapshot_timeout_phase_elapsed_ms: None,
                followup_ready_snapshot_timeout_leaf: None,
                followup_ready_snapshot_timeout_leaf_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_ms: None,
                followup_ready_snapshot_parse_exec_timeout_subphase: None,
                followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_parse_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: None,
                followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: None,
                followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
                    None,
                followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
                    None,
                followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: None,
                followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: None,
                followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: None,
                followup_ready_snapshot_parse_exec_dominant_subphase: None,
                followup_ready_snapshot_parse_exec_dominant_subphase_ms: None,
                followup_ready_snapshot_post_parse_pre_materialization_ms: None,
                followup_ready_snapshot_ready_install_ms: None,
                followup_ready_snapshot_document_symbol_side_work_ms: None,
                followup_ready_snapshot_dominant_phase: None,
                followup_ready_snapshot_dominant_phase_ms: None,
                followup_ready_snapshot_relief_valve_outcome: None,
                followup_ready_snapshot_relief_valve_budget_ms: None,
                followup_ready_snapshot_relief_valve_elapsed_ms: None,
                followup_ready_snapshot_continuation_reason: None,
                followup_shadow_state_available: None,
                followup_wait_reason: None,
                followup_blocker_reason: None,
                followup_runtime_queue_wait_ms: None,
                followup_apply_lag_ms: None,
                followup_wait_for_file_version_ms: None,
                followup_snapshot_with_deps_ms: None,
                terminal_outcome: None,
            }
        });
        trace.followup_blocker_reason = Some(reason.to_string());
    }

    pub(crate) fn diagnostics_save_timeline_fastlane_progress(
        &self,
        key: super::DiagnosticsSaveTimelineCycleKey,
    ) -> DiagnosticsSaveTimelineFastlaneProgress {
        let store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        diagnostics_save_timeline_fastlane_progress_inner(&store, key)
    }

    pub(crate) fn clear_active_diagnostics_save_timeline_cycles_for_file(&self, file_id: V2FileId) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let keys = store
            .active_cycles
            .keys()
            .copied()
            .filter(|key| key.file_id == file_id)
            .collect::<Vec<_>>();
        for key in keys {
            let Some(mut trace) = store.active_cycles.remove(&key) else {
                continue;
            };
            finalize_diagnostics_save_timeline_trace_for_terminal_outcome(
                &mut trace,
                bsl_runtime::application::DiagnosticsDisposition::ClientCancel.as_str(),
            );
            remember_diagnostics_save_timeline_terminal_key_inner(&mut store, key);
            archive_diagnostics_save_timeline_trace_inner(&mut store, trace);
        }
    }

    pub(crate) fn clear_diagnostics_save_timeline_terminal_keys_for_file(&self, file_id: V2FileId) {
        let mut store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        store
            .terminal_keys
            .order
            .retain(|key| key.file_id != file_id);
        store
            .terminal_keys
            .keys
            .retain(|key| key.file_id != file_id);
    }

    pub(crate) fn record_did_change_parse_snapshot_evidence(
        &self,
        uri: &Url,
        key: super::DidChangeParseSnapshotEvidenceKey,
        parse_mode: &'static str,
        base_text_source: &'static str,
        change_shape: &'static str,
        content_changes_count: usize,
        replay_order: &'static str,
        base_document_version: Option<i32>,
        changed_ranges_count: usize,
        fallback_reason: Option<&str>,
        attribution: &super::DidChangeParseSnapshotAttributionV2,
    ) {
        let mut store = self
            .did_change_parse_snapshot_evidence_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !store.entries.contains_key(&key) {
            store.order.push_back(key);
        }
        store.entries.insert(
            key,
            crate::types::DidChangeParseSnapshotEvidenceTrace {
                evidence_id: next_did_change_parse_snapshot_evidence_id_from(
                    self.next_did_change_parse_snapshot_evidence_id.as_ref(),
                ),
                uri: uri.to_string(),
                requested_version: key.requested_version,
                started_at_ms: super::unix_timestamp_ms(),
                parse_mode: parse_mode.to_string(),
                base_text_source: base_text_source.to_string(),
                change_shape: change_shape.to_string(),
                content_changes_count,
                replay_order: replay_order.to_string(),
                base_document_version,
                changed_ranges_count,
                fallback_reason: fallback_reason.map(str::to_string),
                parser_base_root_cause: attribution
                    .stale_parser_base
                    .as_ref()
                    .map(|state| state.root_cause.to_string()),
                shadow_document_version: attribution
                    .stale_parser_base
                    .as_ref()
                    .map(|state| state.shadow_document_version),
                latest_ready_document_version: attribution
                    .stale_parser_base
                    .as_ref()
                    .and_then(|state| state.latest_ready_document_version),
                matching_ready_snapshot_for_shadow_state: attribution
                    .stale_parser_base
                    .as_ref()
                    .map(|state| state.matching_ready_snapshot_for_shadow_state),
                ready_snapshot_prime_attempted: attribution
                    .stale_parser_base
                    .as_ref()
                    .map(|state| state.ready_snapshot_prime_attempted),
                tree_cache_matches_shadow_text_after_prime: attribution
                    .stale_parser_base
                    .as_ref()
                    .and_then(|state| state.tree_cache_matches_shadow_text_after_prime),
            },
        );
        while store.order.len() > super::DID_CHANGE_PARSE_SNAPSHOT_EVIDENCE_MAX_ENTRIES {
            let Some(oldest_key) = store.order.pop_front() else {
                break;
            };
            store.entries.remove(&oldest_key);
        }
    }

    pub(crate) fn snapshot_did_change_parse_snapshot_evidence(
        &self,
        limit: usize,
    ) -> Vec<crate::types::DidChangeParseSnapshotEvidenceTrace> {
        let store = self
            .did_change_parse_snapshot_evidence_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        snapshot_did_change_parse_snapshot_evidence_inner(&store, limit)
    }

    pub(crate) fn snapshot_diagnostics_save_timeline_traces(
        &self,
        limit: usize,
    ) -> Vec<crate::types::DiagnosticsSaveTimelineTrace> {
        let store = self
            .diagnostics_save_timeline_store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        snapshot_diagnostics_save_timeline_traces_inner(&store, limit)
    }

    pub(crate) async fn record_completion_head_hit_v2(
        &self,
        file_id: V2FileId,
        file_version: i32,
        deps_id: DepsSnapshotId,
        settings_id: Option<SettingsId>,
        exact_ready: bool,
    ) {
        self.coordinator
            .record_intellisense_v2_completion_route("head_hit");
        let mut observations = self.completion_head_serve_observations_v2.write().await;
        if exact_ready {
            observations.remove(&file_id);
            return;
        }
        match observations.get(&file_id) {
            Some(existing)
                if existing.file_version == file_version
                    && existing.deps_id == deps_id
                    && existing.settings_id == settings_id => {}
            _ => {
                observations.insert(
                    file_id,
                    super::CompletionHeadServeObservationV2 {
                        file_version,
                        deps_id,
                        settings_id,
                        served_at: Instant::now(),
                    },
                );
            }
        }
    }

    pub(crate) async fn record_completion_exact_hit_v2(
        &self,
        file_id: V2FileId,
        file_version: i32,
        deps_id: DepsSnapshotId,
        settings_id: Option<SettingsId>,
    ) {
        self.coordinator
            .record_intellisense_v2_completion_route("exact_hit");
        let _ = self
            .record_completion_head_to_exact_upgrade_if_pending_v2(
                file_id,
                file_version,
                &deps_id,
                settings_id.as_ref(),
            )
            .await;
    }

    pub(crate) async fn record_completion_head_to_exact_upgrade_if_pending_v2(
        &self,
        file_id: V2FileId,
        file_version: i32,
        deps_id: &DepsSnapshotId,
        settings_id: Option<&SettingsId>,
    ) -> bool {
        let pending_duration = {
            let mut observations = self.completion_head_serve_observations_v2.write().await;
            let Some(existing) = observations.get(&file_id) else {
                return false;
            };
            if existing.file_version != file_version
                || &existing.deps_id != deps_id
                || existing.settings_id.as_ref() != settings_id
            {
                return false;
            }
            let duration = existing.served_at.elapsed();
            observations.remove(&file_id);
            duration
        };

        self.coordinator
            .record_intellisense_v2_completion_head_to_exact_upgrade(pending_duration);
        true
    }

    pub(crate) async fn begin_document_symbol_request_v2(&self, file_id: V2FileId) -> u64 {
        let epoch = self
            .next_document_symbol_request_epoch_v2
            .fetch_add(1, Ordering::Relaxed);
        self.document_symbol_request_epochs_v2
            .write()
            .await
            .insert(file_id, epoch);
        epoch
    }

    pub(crate) async fn document_symbol_request_superseded_v2(
        &self,
        file_id: V2FileId,
        request_epoch: u64,
    ) -> bool {
        self.document_symbol_request_epochs_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            != Some(request_epoch)
    }

    pub(crate) async fn record_document_symbol_ready_v2(
        &self,
        file_id: V2FileId,
        file_version: i32,
        response: tower_lsp::lsp_types::DocumentSymbolResponse,
    ) {
        let Some(latest_received_version) = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
        else {
            return;
        };
        if latest_received_version < file_version {
            return;
        }

        let mut cache = self.document_symbol_ready_cache_v2.write().await;
        if cache
            .get(&file_id)
            .is_some_and(|existing| existing.file_version > file_version)
        {
            return;
        }
        cache.insert(
            file_id,
            super::DocumentSymbolReadyStateV2 {
                file_version,
                response,
            },
        );
    }

    pub(crate) async fn latest_document_symbol_ready_v2(
        &self,
        file_id: V2FileId,
    ) -> Option<super::DocumentSymbolReadyStateV2> {
        self.document_symbol_ready_cache_v2
            .read()
            .await
            .get(&file_id)
            .cloned()
    }
}

fn compute_settings_id_v2(settings: &BslSettings) -> SettingsId {
    let payload = format!(
        "schema={};hover.detail_level={};hover.max_methods={};hover.max_properties={};hover.show_certainty={};diagnostics.detail_level={};diagnostics.show_hints={};formatting.enabled={};formatting.indent_size={}",
        bsl_analysis_v2::SETTINGS_SCHEMA_VERSION,
        settings.hover.detail_level,
        settings.hover.max_methods,
        settings.hover.max_properties,
        settings.hover.show_certainty,
        settings.diagnostics.detail_level,
        settings.diagnostics.show_hints,
        settings.formatting.enabled,
        settings.formatting.indent_size
    );
    SettingsId::from_hash(blake3::hash(payload.as_bytes()).to_hex().to_string())
}

#[cfg(test)]
#[path = "core/tests/mod.rs"]
mod tests;
