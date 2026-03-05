//! Core functionality: constructor and helper methods

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
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

        let cancellation_registry_weak = Arc::downgrade(&completion_cancellation_registry_v2);
        let dispatcher_weak = Arc::downgrade(&completion_dispatcher_v2);
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

        Self {
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
            diagnostics_generation_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_received_file_versions_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_document_shadow_state_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_apply_enqueued_at_v2: Arc::new(RwLock::new(HashMap::new())),
            scale_aware_churn_state_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_seen_files_v2: Arc::new(RwLock::new(std::collections::HashSet::new())),
            completion_stale_fallback_cache_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_parity_state_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_dispatcher_v2,
            completion_cancellation_registry_v2,
            last_deps_id_v2: Arc::new(RwLock::new(Some(initial_deps_id))),
            last_settings_id_v2: Arc::new(RwLock::new(Some(initial_settings_id))),
            full_index_state: Arc::new(Mutex::new(super::FullIndexRuntimeState::default())),
            next_full_index_operation_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            full_index_watchdog_timeout: Duration::from_millis(1_200_000),
            completion_timeline_traces: Arc::new(Mutex::new(VecDeque::new())),
            next_completion_timeline_trace_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    pub(crate) fn next_completion_timeline_trace_id(&self) -> String {
        let id = self
            .next_completion_timeline_trace_id
            .fetch_add(1, Ordering::Relaxed);
        format!("completion-trace-{id}")
    }

    pub(crate) async fn record_completion_timeline_trace(
        &self,
        trace: crate::types::CompletionTimelineTrace,
    ) {
        let mut traces = self.completion_timeline_traces.lock().await;
        traces.push_back(trace);
        while traces.len() > super::COMPLETION_TIMELINE_MAX_ENTRIES {
            let _ = traces.pop_front();
        }
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
