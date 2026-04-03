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

fn record_completion_timeline_trace_inner(
    traces: &StdMutex<VecDeque<crate::types::CompletionTimelineTrace>>,
    trace: crate::types::CompletionTimelineTrace,
) {
    let mut traces = traces
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    traces.push_back(trace);
    while traces.len() > super::COMPLETION_TIMELINE_MAX_ENTRIES {
        let _ = traces.pop_front();
    }
}

fn record_completion_response_flush_completed_at_ms_inner(
    traces: &StdMutex<VecDeque<crate::types::CompletionTimelineTrace>>,
    request_id: &str,
    response_flush_completed_at_ms: u64,
) {
    let mut traces = traces
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(trace) = traces
        .iter_mut()
        .rfind(|trace| trace.request_id.as_deref() == Some(request_id))
    else {
        return;
    };
    let Some(server_edge_details) = trace.server_edge_details.as_mut() else {
        return;
    };
    if server_edge_details.response_flush_completed_at_ms.is_none() {
        server_edge_details.response_flush_completed_at_ms = Some(response_flush_completed_at_ms);
        server_edge_details.response_ready_to_flush_wait_ms = Some(
            response_flush_completed_at_ms.saturating_sub(server_edge_details.response_sent_at_ms),
        );
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
            next_file_id_v2: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            diagnostics_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            type_index_precompute_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            current_revision_head_precompute_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            background_parse_snapshot_apply_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            document_symbol_bootstrap_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            diagnostics_generation_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_received_file_versions_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_document_shadow_state_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_apply_enqueued_at_v2: Arc::new(RwLock::new(HashMap::new())),
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
            completion_timeline_traces: completion_timeline_traces.clone(),
            next_completion_timeline_trace_id: next_completion_timeline_trace_id.clone(),
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

        let completion_timeline_traces_for_flush_hook = completion_timeline_traces.clone();
        super::request_context::set_completion_response_flush_hook(Some(Arc::new(
            move |request_id, response_flush_completed_at_ms| {
                record_completion_response_flush_completed_at_ms_inner(
                    completion_timeline_traces_for_flush_hook.as_ref(),
                    request_id.as_str(),
                    response_flush_completed_at_ms,
                );
            },
        )));

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
#[path = "core/tests.rs"]
mod tests;
