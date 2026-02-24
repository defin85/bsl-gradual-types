//! Core functionality: constructor and helper methods

use std::collections::HashMap;
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
            diagnostics_generation_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_received_file_versions_v2: Arc::new(RwLock::new(HashMap::new())),
            latest_document_shadow_state_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_seen_files_v2: Arc::new(RwLock::new(std::collections::HashSet::new())),
            completion_stale_fallback_cache_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_parity_state_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_dispatcher_v2,
            completion_cancellation_registry_v2,
            last_deps_id_v2: Arc::new(RwLock::new(Some(initial_deps_id))),
            last_settings_id_v2: Arc::new(RwLock::new(Some(initial_settings_id))),
        }
    }

    pub(crate) async fn sync_formatting_capability_registration(&self) {
        const DOC_FORMATTING_ID: &str = "bsl.formatting";
        const RANGE_FORMATTING_ID: &str = "bsl.rangeFormatting";

        let enabled = self.settings.read().await.formatting.enabled;

        let (spawn_worker, dynamic_doc, dynamic_range) = {
            let mut state = self.formatting_capability.write().await;
            state.desired_enabled = enabled;

            if !(state.dynamic_document_formatting || state.dynamic_range_formatting) {
                return;
            }

            if state.in_flight {
                return;
            }

            if state.registered == state.desired_enabled {
                return;
            }

            state.in_flight = true;
            (
                true,
                state.dynamic_document_formatting,
                state.dynamic_range_formatting,
            )
        };

        if !spawn_worker {
            return;
        }

        let client = self.client.clone();
        let state = self.formatting_capability.clone();

        tokio::spawn(async move {
            loop {
                let (desired_enabled, currently_registered) = {
                    let guard = state.read().await;
                    (guard.desired_enabled, guard.registered)
                };

                if desired_enabled == currently_registered {
                    let mut guard = state.write().await;
                    guard.in_flight = false;
                    return;
                }

                let result = if desired_enabled {
                    let mut registrations = Vec::new();
                    if dynamic_doc {
                        registrations.push(Registration {
                            id: DOC_FORMATTING_ID.to_string(),
                            method: DocumentFormattingRequest::METHOD.to_string(),
                            register_options: Some(serde_json::json!({ "documentSelector": null })),
                        });
                    }
                    if dynamic_range {
                        registrations.push(Registration {
                            id: RANGE_FORMATTING_ID.to_string(),
                            method: RangeFormatting::METHOD.to_string(),
                            register_options: Some(serde_json::json!({ "documentSelector": null })),
                        });
                    }

                    client.register_capability(registrations).await
                } else {
                    let mut unregisterations = Vec::new();
                    if dynamic_doc {
                        unregisterations.push(Unregistration {
                            id: DOC_FORMATTING_ID.to_string(),
                            method: DocumentFormattingRequest::METHOD.to_string(),
                        });
                    }
                    if dynamic_range {
                        unregisterations.push(Unregistration {
                            id: RANGE_FORMATTING_ID.to_string(),
                            method: RangeFormatting::METHOD.to_string(),
                        });
                    }

                    client.unregister_capability(unregisterations).await
                };

                match result {
                    Ok(()) => {
                        let mut guard = state.write().await;
                        guard.registered = desired_enabled;
                    }
                    Err(err) => {
                        warn!(
                            "Failed to {} formatting capability: {}",
                            if desired_enabled {
                                "register"
                            } else {
                                "unregister"
                            },
                            err
                        );
                        let mut guard = state.write().await;
                        guard.in_flight = false;
                        return;
                    }
                }
            }
        });
    }

    pub(crate) async fn sync_inlay_hints_capability_registration(&self) {
        const INLAY_HINTS_ID: &str = "bsl.inlayHints";

        let enabled = {
            let settings_enabled = self.settings.read().await.type_hints.enabled;
            let gate_enabled = self
                .config
                .read()
                .await
                .as_ref()
                .and_then(|cfg| cfg.enable_type_hints)
                .unwrap_or(false);
            settings_enabled && gate_enabled
        };

        let spawn_worker = {
            let mut state = self.inlay_hints_capability.write().await;
            state.desired_enabled = enabled;

            if !state.dynamic_registration {
                return;
            }

            if state.in_flight {
                return;
            }

            if state.registered == state.desired_enabled {
                return;
            }

            state.in_flight = true;
            true
        };

        if !spawn_worker {
            return;
        }

        let client = self.client.clone();
        let state = self.inlay_hints_capability.clone();

        tokio::spawn(async move {
            loop {
                let (desired_enabled, currently_registered) = {
                    let guard = state.read().await;
                    (guard.desired_enabled, guard.registered)
                };

                if desired_enabled == currently_registered {
                    let mut guard = state.write().await;
                    guard.in_flight = false;
                    return;
                }

                let result = if desired_enabled {
                    client
                        .register_capability(vec![Registration {
                            id: INLAY_HINTS_ID.to_string(),
                            method: InlayHintRequest::METHOD.to_string(),
                            register_options: Some(serde_json::json!({ "documentSelector": null })),
                        }])
                        .await
                } else {
                    client
                        .unregister_capability(vec![Unregistration {
                            id: INLAY_HINTS_ID.to_string(),
                            method: InlayHintRequest::METHOD.to_string(),
                        }])
                        .await
                };

                match result {
                    Ok(()) => {
                        let mut guard = state.write().await;
                        guard.registered = desired_enabled;
                    }
                    Err(err) => {
                        warn!(
                            "Failed to {} inlay hints capability: {}",
                            if desired_enabled {
                                "register"
                            } else {
                                "unregister"
                            },
                            err
                        );
                        let mut guard = state.write().await;
                        guard.in_flight = false;
                        return;
                    }
                }
            }
        });
    }

    pub(crate) async fn sync_code_actions_capability_registration(&self) {
        const CODE_ACTIONS_ID: &str = "bsl.codeActions";

        let enabled = {
            let settings_enabled = self.settings.read().await.code_actions.enabled;
            let gate_enabled = self
                .config
                .read()
                .await
                .as_ref()
                .and_then(|cfg| cfg.enable_code_actions)
                .unwrap_or(false);
            settings_enabled && gate_enabled
        };

        let spawn_worker = {
            let mut state = self.code_actions_capability.write().await;
            state.desired_enabled = enabled;

            if !state.dynamic_registration {
                return;
            }

            if state.in_flight {
                return;
            }

            if state.registered == state.desired_enabled {
                return;
            }

            state.in_flight = true;
            true
        };

        if !spawn_worker {
            return;
        }

        let client = self.client.clone();
        let state = self.code_actions_capability.clone();

        tokio::spawn(async move {
            loop {
                let (desired_enabled, currently_registered) = {
                    let guard = state.read().await;
                    (guard.desired_enabled, guard.registered)
                };

                if desired_enabled == currently_registered {
                    let mut guard = state.write().await;
                    guard.in_flight = false;
                    return;
                }

                let result = if desired_enabled {
                    client
                        .register_capability(vec![Registration {
                            id: CODE_ACTIONS_ID.to_string(),
                            method: CodeActionRequest::METHOD.to_string(),
                            register_options: Some(serde_json::json!({
                                "documentSelector": null,
                                "codeActionKinds": ["quickfix", "refactor.extract"]
                            })),
                        }])
                        .await
                } else {
                    client
                        .unregister_capability(vec![Unregistration {
                            id: CODE_ACTIONS_ID.to_string(),
                            method: CodeActionRequest::METHOD.to_string(),
                        }])
                        .await
                };

                match result {
                    Ok(()) => {
                        let mut guard = state.write().await;
                        guard.registered = desired_enabled;
                    }
                    Err(err) => {
                        warn!(
                            "Failed to {} code actions capability: {}",
                            if desired_enabled {
                                "register"
                            } else {
                                "unregister"
                            },
                            err
                        );
                        let mut guard = state.write().await;
                        guard.in_flight = false;
                        return;
                    }
                }
            }
        });
    }

    pub async fn update_diagnostics_count(&self, uri: &Url, count: usize) {
        let mut counts = self.diagnostics_counts.write().await;
        if count == 0 {
            counts.remove(uri);
        } else {
            counts.insert(uri.clone(), count);
        }
    }

    pub(crate) async fn get_or_create_file_id_v2(&self, uri: &Url) -> V2FileId {
        let key = match uri.to_file_path() {
            Ok(path) => V2FileKey::Path(path),
            Err(_) => V2FileKey::Url(uri.to_string()),
        };

        if let Some(&file_id) = self.file_key_to_file_id_v2.read().await.get(&key) {
            return file_id;
        }

        let mut map = self.file_key_to_file_id_v2.write().await;
        if let Some(&file_id) = map.get(&key) {
            return file_id;
        }

        let raw = self.next_file_id_v2.fetch_add(1, Ordering::Relaxed);
        let file_id = V2FileId(raw);
        map.insert(key, file_id);
        file_id
    }

    pub(crate) async fn get_file_id_v2(&self, uri: &Url) -> Option<V2FileId> {
        let key = match uri.to_file_path() {
            Ok(path) => V2FileKey::Path(path),
            Err(_) => V2FileKey::Url(uri.to_string()),
        };
        self.file_key_to_file_id_v2.read().await.get(&key).copied()
    }

    pub(crate) async fn build_execution_context_v2(
        &self,
        operation: bsl_runtime::application::SemanticOperation,
        file_id: V2FileId,
        min_file_version: Option<i32>,
        flow_sensitive: bool,
    ) -> bsl_runtime::application::ExecutionContext {
        self.build_execution_context_v2_with_completion_mode(
            operation,
            file_id,
            min_file_version,
            flow_sensitive,
            None,
        )
        .await
    }

    pub(crate) async fn build_execution_context_v2_with_completion_mode(
        &self,
        operation: bsl_runtime::application::SemanticOperation,
        file_id: V2FileId,
        min_file_version: Option<i32>,
        flow_sensitive: bool,
        completion_mode: Option<&'static str>,
    ) -> bsl_runtime::application::ExecutionContext {
        let settings = self.settings.read().await.clone();
        let settings_id = self
            .last_settings_id_v2
            .read()
            .await
            .clone()
            .unwrap_or_else(|| compute_settings_id_v2(&settings));
        let cancellation = match operation {
            bsl_runtime::application::SemanticOperation::Diagnostics => {
                bsl_runtime::application::CancellationPolicy::BestEffort
            }
            _ => bsl_runtime::application::CancellationPolicy::RespectClientAbort,
        };
        let expected_deps_id = self.last_deps_id_v2.read().await.clone();

        bsl_runtime::application::ExecutionContext {
            origin: bsl_runtime::application::ObservabilityOrigin::Lsp,
            operation,
            completion_mode,
            file_id,
            min_file_version,
            expected_deps_id,
            flow_sensitive,
            settings: bsl_runtime::application::ExecutionSettings {
                settings_id,
                diagnostics_detail_level: bsl_shared::formatting::DetailLevel::parse(
                    &settings.diagnostics.detail_level,
                ),
            },
            cancellation,
        }
    }

    pub(crate) async fn prepare_lsp_stateful_operation_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        operation: bsl_runtime::application::SemanticOperation,
        flow_sensitive: bool,
    ) -> Result<
        (
            bsl_runtime::application::ExecutionContext,
            bsl_runtime::application::PreparedOperationSnapshot,
            i32,
        ),
        bsl_runtime::application::SemanticOutcome,
    > {
        self.prepare_lsp_stateful_operation_v2_with_completion_mode(
            uri,
            file_id,
            operation,
            flow_sensitive,
            None,
        )
        .await
    }

    pub(crate) async fn prepare_lsp_stateful_operation_v2_with_completion_mode(
        &self,
        uri: &Url,
        file_id: V2FileId,
        operation: bsl_runtime::application::SemanticOperation,
        flow_sensitive: bool,
        completion_mode: Option<&'static str>,
    ) -> Result<
        (
            bsl_runtime::application::ExecutionContext,
            bsl_runtime::application::PreparedOperationSnapshot,
            i32,
        ),
        bsl_runtime::application::SemanticOutcome,
    > {
        let min_file_version = match self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
        {
            Some(version) => version,
            None => {
                let path = match uri.to_file_path() {
                    Ok(path) => path,
                    Err(_) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            operation = operation.as_str(),
                            "IntelliSense v2: missing local file path for fallback load"
                        );
                        return Err(bsl_runtime::application::SemanticOutcome::StaleVersion);
                    }
                };

                let file_content = match read_bsl_file(&path) {
                    Ok(content) => content,
                    Err(err) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            operation = operation.as_str(),
                            error = %err,
                            "IntelliSense v2: failed to read file for fallback load"
                        );
                        return Err(bsl_runtime::application::SemanticOutcome::StaleVersion);
                    }
                };

                let path_string = path.to_string_lossy().to_string();
                self.analysis_v2
                    .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                        file_id,
                        text: Arc::from(file_content.clone()),
                        version: 0,
                        path: Arc::from(path_string.clone()),
                    }]);
                self.latest_received_file_versions_v2
                    .write()
                    .await
                    .insert(file_id, 0);
                self.latest_document_shadow_state_v2.write().await.insert(
                    file_id,
                    DocumentShadowStateV2 {
                        version: 0,
                        text: Arc::from(file_content),
                    },
                );
                0
            }
        };

        let context = self
            .build_execution_context_v2_with_completion_mode(
                operation,
                file_id,
                Some(min_file_version),
                flow_sensitive,
                completion_mode,
            )
            .await;
        let prepared = self
            .analysis_v2
            .prepare_stateful_operation(&context, Some(self.coordinator.as_ref()))
            .await?;

        Ok((context, prepared, min_file_version))
    }

    pub(crate) async fn sync_v2_globals(&self) {
        let settings = self.settings.read().await.clone();
        let settings_id = compute_settings_id_v2(&settings);
        let diagnostics_detail_level =
            bsl_shared::formatting::DetailLevel::parse(&settings.diagnostics.detail_level);

        let mut changes = Vec::new();

        {
            let mut last_settings_id = self.last_settings_id_v2.write().await;
            if last_settings_id.as_ref() != Some(&settings_id) {
                *last_settings_id = Some(settings_id.clone());
                changes.push(bsl_analysis_v2::Change::SetSettingsSnapshot {
                    settings_id,
                    diagnostics_detail_level,
                });
            }
        }

        self.analysis_v2.apply_changes(changes);
    }

    pub(crate) async fn deps_update_v2(
        &self,
        reason: &str,
        platform_docs_root: Option<PathBuf>,
        config_root: Option<PathBuf>,
    ) {
        let build_started = Instant::now();
        let coordinator = self.coordinator.clone();
        let bundle_result = tokio::task::spawn_blocking(move || {
            build_deps_bundle_v2(
                coordinator.as_ref(),
                platform_docs_root.as_deref(),
                config_root.as_deref(),
            )
        })
        .await;

        let bundle = match bundle_result {
            Ok(Ok(bundle)) => bundle,
            Ok(Err(err)) => {
                let elapsed = build_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_deps_update_build_latency(elapsed);
                self.coordinator.record_intellisense_v2_deps_update_error();
                warn!(
                    "deps_update_v2 build failed: reason={}, error={}",
                    reason, err
                );
                return;
            }
            Err(err) => {
                let elapsed = build_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_deps_update_build_latency(elapsed);
                self.coordinator.record_intellisense_v2_deps_update_error();
                warn!(
                    "deps_update_v2 build join failed: reason={}, error={}",
                    reason, err
                );
                return;
            }
        };

        let build_elapsed = build_started.elapsed();
        self.coordinator
            .record_intellisense_v2_deps_update_build_latency(build_elapsed);

        self.apply_deps_bundle_v2(reason, bundle).await;
    }

    pub(crate) async fn apply_deps_bundle_v2(&self, reason: &str, bundle: DepsBundleV2) {
        let apply_started = Instant::now();
        let ok = self
            .analysis_v2
            .apply_deps_bundle(
                bundle.deps_id.clone(),
                bundle.semantic_deps.clone(),
                bundle.index_snapshot.clone(),
            )
            .await;
        let apply_elapsed = apply_started.elapsed();
        self.coordinator
            .record_intellisense_v2_deps_update_apply_latency(apply_elapsed);

        if !ok {
            self.coordinator.record_intellisense_v2_deps_update_error();
            warn!(
                "deps_update_v2 apply failed: reason={}, deps_id={}, index_snapshot_id={}",
                reason,
                bundle.deps_id.as_str(),
                bundle.meta.index_snapshot_id
            );
            return;
        }

        {
            let mut last_deps_id = self.last_deps_id_v2.write().await;
            *last_deps_id = Some(bundle.deps_id.clone());
        }

        self.coordinator
            .record_intellisense_v2_deps_update_success();
        info!(
            "deps_update_v2 applied: reason={}, deps_id={}, index_snapshot_id={}, platform_version={}, platform_fp={}, config_fp={}, strict_fingerprint={}",
            reason,
            bundle.deps_id.as_str(),
            bundle.meta.index_snapshot_id,
            bundle.meta.platform_version,
            bundle.meta.platform_fingerprint.as_deref().unwrap_or("none"),
            bundle.meta.config_fingerprint.as_deref().unwrap_or("none"),
            bundle.meta.strict_fingerprint
        );
    }

    pub(crate) async fn cancel_diagnostics_v2(&self, file_id: V2FileId) {
        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        let keys: Vec<super::DiagnosticsTaskKeyV2> = tasks
            .keys()
            .copied()
            .filter(|key| key.file_id == file_id)
            .collect();
        for key in keys {
            if let Some(task) = tasks.remove(&key) {
                task.handle.abort();
            }
        }
    }

    pub(crate) async fn bump_diagnostics_generation_v2(&self, file_id: V2FileId) -> u64 {
        let mut generations = self.diagnostics_generation_v2.write().await;
        let next = generations
            .get(&file_id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        generations.insert(file_id, next);
        next
    }

    async fn current_diagnostics_generation_v2(&self, file_id: V2FileId) -> Option<u64> {
        self.diagnostics_generation_v2
            .read()
            .await
            .get(&file_id)
            .copied()
    }

    fn record_diagnostics_pipeline_event_v2(
        &self,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        reason: bsl_runtime::application::DiagnosticsDisposition,
    ) {
        self.coordinator
            .record_intellisense_v2_diagnostics_pipeline_event(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                trigger.as_str(),
                profile.as_str(),
                reason.as_str(),
            );
    }

    pub(crate) async fn run_diagnostics_profile_immediate_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
        diagnostics_generation: u64,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
    ) {
        let _ = self
            .execute_diagnostics_profile_once_v2(
                &uri,
                file_id,
                expected_version,
                diagnostics_generation,
                trigger,
                profile,
            )
            .await;
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn schedule_diagnostics_profile_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
        diagnostics_generation: u64,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        debounce: bool,
    ) {
        let key = super::DiagnosticsTaskKeyV2 { file_id, profile };
        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        if let Some(task) = tasks.get_mut(&key) {
            task.requested_version = expected_version;
            task.diagnostics_generation = diagnostics_generation;
            task.trigger = trigger;
            task.debounce = debounce;
            return;
        }

        let server = self.clone();
        let uri_for_task = uri.clone();
        let handle = tokio::spawn(async move {
            loop {
                // If the document is already closed, stop.
                let file_still_open = server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .contains_key(&file_id);
                if !file_still_open {
                    break;
                }

                let (requested_version, requested_generation, trigger, debounce) = {
                    let tasks = server.diagnostics_tasks_v2.lock().await;
                    let Some(task) = tasks.get(&key) else {
                        break;
                    };
                    (
                        task.requested_version,
                        task.diagnostics_generation,
                        task.trigger,
                        task.debounce,
                    )
                };

                // Coalesce rapid edits: while user is typing, keep moving the target forward.
                if debounce {
                    let delay = diagnostics_debounce_duration();
                    if delay != Duration::from_millis(0) {
                        tokio::time::sleep(delay).await;
                    }

                    let current_requested = {
                        let tasks = server.diagnostics_tasks_v2.lock().await;
                        let Some(task) = tasks.get(&key) else {
                            break;
                        };
                        (task.requested_version, task.diagnostics_generation)
                    };
                    if current_requested != (requested_version, requested_generation) {
                        continue;
                    }
                }

                let _ = server
                    .execute_diagnostics_profile_once_v2(
                        &uri_for_task,
                        file_id,
                        requested_version,
                        requested_generation,
                        trigger,
                        profile,
                    )
                    .await;

                let mut tasks = server.diagnostics_tasks_v2.lock().await;
                let Some(task) = tasks.get(&key) else {
                    break;
                };
                if task.requested_version == requested_version
                    && task.diagnostics_generation == requested_generation
                {
                    tasks.remove(&key);
                    break;
                }
            }
        });

        tasks.insert(
            key,
            super::DiagnosticsTaskV2 {
                requested_version: expected_version,
                diagnostics_generation,
                trigger,
                debounce,
                handle,
            },
        );
    }

    async fn execute_diagnostics_profile_once_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        requested_version: i32,
        requested_generation: u64,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        let current_generation = self.current_diagnostics_generation_v2(file_id).await;
        if current_generation != Some(requested_generation) {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
            );
            return bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
        }

        let current_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if current_version != Some(requested_version) {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::SupersededVersion,
            );
            return bsl_runtime::application::DiagnosticsDisposition::SupersededVersion;
        }

        let (show_hints, flow_sensitive_enabled) = {
            let settings = self.settings.read().await;
            (
                settings.diagnostics.show_hints,
                settings.enable_flow_sensitive,
            )
        };
        let plan =
            bsl_runtime::application::diagnostics_execution_plan(profile, flow_sensitive_enabled);
        if !plan.run_syntax && !plan.run_semantic {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::Published,
            );
            return bsl_runtime::application::DiagnosticsDisposition::Published;
        }

        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                file_id,
                Some(requested_version),
                plan.flow_sensitive_semantic,
            )
            .await;
        let prepared = match self
            .analysis_v2
            .prepare_stateful_operation(&context, Some(self.coordinator.as_ref()))
            .await
        {
            Ok(prepared) => prepared,
            Err(outcome) => {
                let disposition = if matches!(
                    outcome,
                    bsl_runtime::application::SemanticOutcome::StaleVersion
                ) {
                    bsl_runtime::application::DiagnosticsDisposition::SupersededVersion
                } else {
                    bsl_runtime::application::DiagnosticsDisposition::Cancelled
                };
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    expected_version = requested_version,
                    expected_generation = requested_generation,
                    profile = profile.as_str(),
                    trigger = trigger.as_str(),
                    outcome = outcome.as_str(),
                    "diagnostics_v2: skip publish (stateful operation not ready)"
                );
                self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                return disposition;
            }
        };

        let wait_elapsed = prepared.wait_elapsed.unwrap_or(Duration::ZERO);
        if wait_elapsed > Duration::ZERO {
            if let Some(threshold) = super::intellisense_v2_slow_client_log_threshold() {
                if wait_elapsed >= threshold {
                    self.client
                        .log_message(
                            MessageType::INFO,
                            format!(
                                "[perf] diagnostics_v2 wait_for_file_version: wait_ms={} uri={} file_id={} expected_version={} profile={}",
                                wait_elapsed.as_millis(),
                                uri,
                                file_id.0,
                                requested_version,
                                profile.as_str(),
                            ),
                        )
                        .await;
                }
            }
            if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "diagnostics_v2: wait_for_file_version is slow"
                    );
                }
            }
        }

        let current_generation = self.current_diagnostics_generation_v2(file_id).await;
        if current_generation != Some(requested_generation) {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
            );
            return bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
        }
        let current_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        if current_version != Some(requested_version) {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::SupersededVersion,
            );
            return bsl_runtime::application::DiagnosticsDisposition::SupersededVersion;
        }

        let mut analysis = Some(prepared.snapshot.analysis);
        let index_snapshot_id = prepared.snapshot.index_snapshot.id.as_str().to_string();
        let observed_deps_id = Some(prepared.snapshot.deps_id.as_str().to_string());
        let observed_settings_id = analysis
            .as_ref()
            .and_then(|value| value.settings_id().ok().map(|id| id.as_str().to_string()));
        let file_text = analysis
            .as_ref()
            .and_then(|value| value.file_text(file_id).ok().flatten());
        let line_index = analysis
            .as_ref()
            .and_then(|value| value.line_index(file_id).ok().flatten());
        let (file_bytes, file_lines) = file_text
            .as_deref()
            .map(|text| (text.len(), text.lines().count()))
            .unwrap_or((0, 0));

        let snapshot_elapsed = prepared.snapshot_elapsed;
        if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
            if snapshot_elapsed >= threshold {
                warn!(
                    uri = %uri,
                    file_id = file_id.0,
                    expected_version = requested_version,
                    expected_generation = requested_generation,
                    profile = profile.as_str(),
                    snapshot_ms = snapshot_elapsed.as_millis(),
                    threshold_ms = threshold.as_millis(),
                    "diagnostics_v2: snapshot acquisition is slow"
                );
            }
        }

        let mut diagnostics = Vec::new();
        let mut was_cancelled = false;

        if plan.run_syntax {
            self.coordinator
                .record_intellisense_v2_payload_shape_with_origin(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    bsl_runtime::application::ObservabilityStage::SyntaxDiagnosticsQuery.as_str(),
                    file_bytes,
                    file_lines,
                );
            let analysis_for_blocking = analysis
                .take()
                .expect("analysis snapshot must be available for syntax stage");
            let context_for_blocking = context.clone();
            let coordinator_for_blocking = self.coordinator.clone();
            let uri_for_blocking = uri.clone();
            let file_text_for_blocking = file_text.clone();
            let line_index_for_blocking = line_index.clone();
            let syntax_result = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                plan.cpu_class,
                context_for_blocking.origin.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    let started = Instant::now();
                    let syntax_query =
                        bsl_runtime::application::IntellisenseV2Facade::run_syntax_diagnostics_query_singleflight(
                            &context_for_blocking,
                            &analysis_for_blocking,
                            Some(coordinator_for_blocking.as_ref()),
                            file_id,
                        );
                    let elapsed = started.elapsed();
                    match syntax_query {
                        Ok(Some(syntax_errors)) => {
                            let mut diagnostics = Vec::new();
                            if let (Some(text), Some(index)) =
                                (file_text_for_blocking.as_deref(), line_index_for_blocking.as_deref())
                            {
                                diagnostics.extend(syntax_errors_to_diagnostics(
                                    &syntax_errors,
                                    &uri_for_blocking,
                                    text,
                                    index,
                                ));
                            }
                            (diagnostics, false, elapsed, analysis_for_blocking)
                        }
                        Ok(None) => (Vec::new(), false, elapsed, analysis_for_blocking),
                        Err(_) => (Vec::new(), true, elapsed, analysis_for_blocking),
                    }
                },
            )
            .await;
            match syntax_result {
                Ok((syntax_diagnostics, syntax_cancelled, syntax_elapsed, next_analysis)) => {
                    analysis = Some(next_analysis);
                    diagnostics.extend(syntax_diagnostics);
                    was_cancelled |= syntax_cancelled;
                    if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                        if syntax_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                expected_generation = requested_generation,
                                profile = profile.as_str(),
                                syntax_diagnostics_ms = syntax_elapsed.as_millis(),
                                file_bytes,
                                file_lines,
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: syntax_diagnostics query is slow"
                            );
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: syntax spawn_blocking failed"
                    );
                    was_cancelled = true;
                }
            }
        }

        if was_cancelled {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::Cancelled,
            );
            return bsl_runtime::application::DiagnosticsDisposition::Cancelled;
        }

        if plan.run_semantic {
            self.coordinator
                .record_intellisense_v2_payload_shape_with_origin(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery.as_str(),
                    file_bytes,
                    file_lines,
                );
            let current_generation = self.current_diagnostics_generation_v2(file_id).await;
            if current_generation != Some(requested_generation) {
                self.record_diagnostics_pipeline_event_v2(
                    trigger,
                    profile,
                    bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration,
                );
                return bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
            }
            let current_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();
            if current_version != Some(requested_version) {
                self.record_diagnostics_pipeline_event_v2(
                    trigger,
                    profile,
                    bsl_runtime::application::DiagnosticsDisposition::SupersededVersion,
                );
                return bsl_runtime::application::DiagnosticsDisposition::SupersededVersion;
            }

            let analysis_for_blocking = analysis
                .take()
                .expect("analysis snapshot must be available for semantic stage");
            let context_for_blocking = context.clone();
            let coordinator_for_blocking = self.coordinator.clone();
            let file_text_for_blocking = file_text.clone();
            let line_index_for_blocking = line_index.clone();
            let semantic_flow_sensitive = plan.flow_sensitive_semantic;
            let semantic_show_hints = show_hints;
            let semantic_result = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                plan.cpu_class,
                context_for_blocking.origin.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    let started = Instant::now();
                    let query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
                        &context_for_blocking,
                        bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
                        &analysis_for_blocking,
                        Some(coordinator_for_blocking.as_ref()),
                        |analysis| {
                            if semantic_flow_sensitive {
                                analysis.semantic_diagnostics_flow_sensitive(file_id)
                            } else {
                                analysis.semantic_diagnostics(file_id)
                            }
                        },
                    );
                    let elapsed = started.elapsed();
                    match query {
                        Ok(Some(semantic_errors)) => {
                            let mut diagnostics = Vec::new();
                            for error in semantic_errors.iter() {
                                if !semantic_show_hints
                                    && matches!(
                                        error.severity,
                                        bsl_shared::domain::types::DiagnosticSeverity::Hint
                                    )
                                {
                                    continue;
                                }
                                if let (Some(text), Some(index)) =
                                    (file_text_for_blocking.as_deref(), line_index_for_blocking.as_deref())
                                {
                                    diagnostics.push(semantic_error_to_diagnostic(error, text, index));
                                }
                            }
                            (diagnostics, false, elapsed)
                        }
                        Ok(None) => (Vec::new(), false, elapsed),
                        Err(_) => (Vec::new(), true, elapsed),
                    }
                },
            )
            .await;
            match semantic_result {
                Ok((semantic_diagnostics, semantic_cancelled, semantic_elapsed)) => {
                    diagnostics.extend(semantic_diagnostics);
                    was_cancelled |= semantic_cancelled;
                    if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                        if semantic_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                expected_generation = requested_generation,
                                profile = profile.as_str(),
                                semantic_diagnostics_ms = semantic_elapsed.as_millis(),
                                file_bytes,
                                file_lines,
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: semantic_diagnostics query is slow"
                            );
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: semantic spawn_blocking failed"
                    );
                    was_cancelled = true;
                }
            }
        }

        let diagnostics_len = diagnostics.len();
        let (is_current, current_version, current_deps_id, current_settings_id, current_generation) = {
            let current_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();
            let current_deps_id = self
                .last_deps_id_v2
                .read()
                .await
                .as_ref()
                .map(|id| id.as_str().to_string());
            let current_settings_id = self
                .last_settings_id_v2
                .read()
                .await
                .as_ref()
                .map(|id| id.as_str().to_string());
            let current_generation = self.current_diagnostics_generation_v2(file_id).await;
            let is_current = !was_cancelled
                && current_version == Some(requested_version)
                && current_deps_id == observed_deps_id
                && current_settings_id == observed_settings_id
                && current_generation == Some(requested_generation);
            (
                is_current,
                current_version,
                current_deps_id,
                current_settings_id,
                current_generation,
            )
        };

        if !is_current {
            let disposition = if was_cancelled {
                bsl_runtime::application::DiagnosticsDisposition::Cancelled
            } else if current_generation != Some(requested_generation) {
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration
            } else if current_version != Some(requested_version) {
                bsl_runtime::application::DiagnosticsDisposition::SupersededVersion
            } else {
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration
            };
            debug!(
                uri = %uri,
                file_id = file_id.0,
                expected_version = requested_version,
                expected_generation = requested_generation,
                current_version = ?current_version,
                current_generation = ?current_generation,
                observed_deps_id = ?observed_deps_id,
                current_deps_id = ?current_deps_id,
                observed_settings_id = ?observed_settings_id,
                current_settings_id = ?current_settings_id,
                profile = profile.as_str(),
                trigger = trigger.as_str(),
                disposition = disposition.as_str(),
                "diagnostics_v2: skip publish (stale)"
            );
            self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
            return disposition;
        }

        debug!(
            uri = %uri,
            file_id = file_id.0,
            expected_version = requested_version,
            expected_generation = requested_generation,
            deps_id = observed_deps_id.as_deref().unwrap_or_default(),
            settings_id = observed_settings_id.as_deref().unwrap_or_default(),
            index_snapshot_id = index_snapshot_id,
            profile = profile.as_str(),
            trigger = trigger.as_str(),
            diagnostics_len,
            "diagnostics_v2: publish diagnostics"
        );

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(requested_version))
            .await;
        self.update_diagnostics_count(uri, diagnostics_len).await;
        self.record_diagnostics_pipeline_event_v2(
            trigger,
            profile,
            bsl_runtime::application::DiagnosticsDisposition::Published,
        );
        bsl_runtime::application::DiagnosticsDisposition::Published
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
mod tests {
    use super::*;
    use axum::http::{header, Request as AxumRequest};
    use bsl_agent::jobs::JobManager;
    use bsl_agent::server::types::{
        BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
        BslSymbolSearchParams, BslTypeAtPositionParams, DocumentRef as McpDocumentRef,
        FileRef as McpFileRef, Position as McpPosition, WorkspaceOpenParams, WorkspaceScope,
        WorkspaceScopeTagged,
    };
    use bsl_agent::session::SessionManager;
    use bsl_agent::types::JobStateDto;
    use bsl_backend::presentation::web::{create_router, AppState};
    use bsl_backend::system::{
        build_deps_bundle_v2, EffectiveStartupInputs, IndexItem, IndexItemKind, IndexKind,
        IndexSnapshot, IndexSnapshotId, TypeKind,
    };
    use futures::StreamExt;
    use std::collections::BTreeSet;
    use tokio::sync::mpsc::UnboundedReceiver;
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::jsonrpc::Request;
    use tower_lsp::lsp_types::{
        ClientCapabilities, CodeActionContext, CodeActionOrCommand, CodeActionParams,
        CompletionContext, CompletionItemKind, CompletionParams, CompletionResponse,
        CompletionTriggerKind, DidChangeConfigurationParams, DidChangeTextDocumentParams,
        DidOpenTextDocumentParams, DocumentFormattingParams, DocumentRangeFormattingParams,
        DocumentSymbolParams, DocumentSymbolResponse, FormattingOptions, GotoDefinitionParams,
        GotoDefinitionResponse, Hover, HoverContents, HoverParams, InitializeParams,
        InitializedParams, InlayHint, InlayHintLabel, InlayHintParams, Location, MarkedString,
        PartialResultParams, Position, PrepareRenameResponse, PublishDiagnosticsParams, Range,
        ReferenceContext, ReferenceParams, RenameParams, SymbolInformation, SymbolKind,
        TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
        TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier, WorkDoneProgressParams,
        WorkspaceEdit, WorkspaceSymbolParams,
    };
    use tower_lsp::LanguageServer;
    use tower_lsp::LspService;

    const UNIFIED_STAGE_COUNTER_KEYS: &[&str] = &[
        "intellisense_v2_runtime_wait_for_file_version_queue_wait_total",
        "intellisense_v2_runtime_wait_for_file_version_exec_total",
        "intellisense_v2_runtime_snapshot_with_deps_queue_wait_total",
        "intellisense_v2_runtime_snapshot_with_deps_exec_total",
        "intellisense_v2_wait_for_file_version_diagnostics_total",
        "intellisense_v2_snapshot_diagnostics_total",
        "intellisense_v2_ir_query_other_total",
        "intellisense_v2_syntax_diagnostics_query_total",
        "intellisense_v2_semantic_diagnostics_query_total",
        "intellisense_v2_parse_result_query_total",
        "intellisense_v2_ir_query_cancelled_total_other",
        "intellisense_v2_query_cancelled_total_syntax",
        "intellisense_v2_query_cancelled_total_semantic",
        "intellisense_v2_interactive_wait_budget_exhausted_total",
        "intellisense_v2_interactive_stale_served_total",
        "intellisense_v2_interactive_knob_clamped_total",
        "intellisense_v2_singleflight_leader_total",
        "intellisense_v2_singleflight_shared_total",
        "intellisense_v2_singleflight_key_unavailable_total",
        "intellisense_v2_runtime_queue_wait_interactive_total",
        "intellisense_v2_runtime_queue_wait_background_total",
        "intellisense_v2_runtime_exec_interactive_total",
        "intellisense_v2_runtime_exec_background_total",
        "intellisense_v2_completion_stale_fallback_total",
        "intellisense_v2_completion_fallback_unavailable_total",
        "intellisense_v2_revision_lag_sample_total",
        "intellisense_v2_observability_contract_violation_total",
        "intellisense_v2_projection_missing_total",
        "intellisense_v2_runtime_saturation_sample_total",
    ];

    const UNIFIED_STAGE_HISTOGRAM_KEYS: &[&str] = &[
        "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
        "intellisense_v2_runtime_wait_for_file_version_exec_ms",
        "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
        "intellisense_v2_runtime_snapshot_with_deps_exec_ms",
        "intellisense_v2_wait_for_file_version_diagnostics_ms",
        "intellisense_v2_snapshot_diagnostics_ms",
        "intellisense_v2_ir_query_other_ms",
        "intellisense_v2_syntax_diagnostics_query_ms",
        "intellisense_v2_semantic_diagnostics_query_ms",
        "intellisense_v2_parse_result_query_ms",
        "intellisense_v2_singleflight_wait_ms",
        "intellisense_v2_runtime_queue_wait_interactive_ms",
        "intellisense_v2_runtime_queue_wait_background_ms",
        "intellisense_v2_runtime_exec_interactive_ms",
        "intellisense_v2_runtime_exec_background_ms",
        "intellisense_v2_revision_lag_versions",
    ];

    #[test]
    fn diagnostics_debounce_floor_prevents_zero_ms_tight_loops() {
        assert_eq!(clamp_diagnostics_debounce_ms(0), 25);
        assert_eq!(clamp_diagnostics_debounce_ms(1), 25);
        assert_eq!(clamp_diagnostics_debounce_ms(25), 25);
        assert_eq!(clamp_diagnostics_debounce_ms(250), 250);
    }

    const UNIFIED_STAGE_GAUGE_KEYS: &[&str] = &[
        "intellisense_v2_runtime_saturation_waiters_interactive",
        "intellisense_v2_runtime_saturation_waiters_background",
        "intellisense_v2_runtime_saturation_permits_interactive",
        "intellisense_v2_runtime_saturation_permits_background",
        "intellisense_v2_runtime_saturation_permits_shared",
        "intellisense_v2_runtime_saturation_queue_depth_total",
    ];

    fn assert_unified_intellisense_v2_stage_contract(payload: &serde_json::Value) {
        let metrics = payload.get("metrics").expect("metrics field");
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let gauges = metrics
            .get("gauges")
            .and_then(|value| value.as_object())
            .expect("metrics.gauges object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");

        for key in UNIFIED_STAGE_COUNTER_KEYS {
            assert!(
                counters.contains_key(*key),
                "missing counter key {key}, got keys={:?}",
                counters.keys().collect::<Vec<_>>()
            );
        }

        for key in UNIFIED_STAGE_HISTOGRAM_KEYS {
            assert!(
                histograms.contains_key(*key),
                "missing histogram key {key}, got keys={:?}",
                histograms.keys().collect::<Vec<_>>()
            );
        }

        for key in UNIFIED_STAGE_GAUGE_KEYS {
            assert!(
                gauges.contains_key(*key),
                "missing gauge key {key}, got keys={:?}",
                gauges.keys().collect::<Vec<_>>()
            );
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct NormalizedSemanticDiagnostic {
        message: String,
        severity: String,
        start_line: u32,
        start_character: u32,
        end_line: u32,
        end_character: u32,
    }

    async fn initialize_lsp_service(service: &mut LspService<BslLanguageServer>) {
        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let initialize_response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(
            initialize_response.is_some(),
            "initialize should return a response"
        );

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );
    }

    async fn wait_lsp_publish_diagnostics(
        receiver: &mut UnboundedReceiver<PublishDiagnosticsParams>,
        uri: &Url,
    ) -> Vec<tower_lsp::lsp_types::Diagnostic> {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
        let mut last_for_uri: Option<Vec<tower_lsp::lsp_types::Diagnostic>> = None;
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            match tokio::time::timeout_at(deadline, receiver.recv()).await {
                Ok(Some(params)) if params.uri == *uri => {
                    let diagnostics = params.diagnostics;
                    if !diagnostics.is_empty() {
                        return diagnostics;
                    }
                    last_for_uri = Some(diagnostics);
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => break,
            }
        }
        last_for_uri.unwrap_or_default()
    }

    fn build_web_test_state() -> AppState {
        let coordinator = Arc::new(SystemCoordinator::new());
        coordinator
            .start_with_paths_blocking(None, None, None, None)
            .expect("startup");
        let deps_bundle_v2 =
            build_deps_bundle_v2(coordinator.as_ref(), None, None).expect("deps bundle v2");

        AppState {
            deps_bundle_v2: Arc::new(tokio::sync::RwLock::new(Arc::new(deps_bundle_v2))),
            system_coordinator: coordinator,
            syntax_helper_path: None,
            startup_inputs: Arc::new(tokio::sync::RwLock::new(EffectiveStartupInputs {
                syntax_helper_path: None,
                configuration_path: None,
                platform_version: None,
                cache_enabled: true,
                strict_fingerprint: false,
            })),
        }
    }

    async fn wait_mcp_startup(job_manager: &JobManager, startup_job_id: Option<&str>) {
        let job_id = startup_job_id.expect("startup_job_id missing");
        loop {
            let status = job_manager.wait(job_id, 60_000).await.expect("job_wait");
            match status.state {
                JobStateDto::Succeeded => break,
                JobStateDto::Queued | JobStateDto::Running => continue,
                other => panic!("startup job ended unexpectedly: {}", other.as_str()),
            }
        }
    }

    fn normalize_lsp_semantic_diagnostics(
        diagnostics: &[tower_lsp::lsp_types::Diagnostic],
    ) -> Vec<NormalizedSemanticDiagnostic> {
        let mut normalized: Vec<NormalizedSemanticDiagnostic> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2"))
            .map(|diagnostic| {
                let severity = match diagnostic.severity {
                    Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR) => "error",
                    Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING) => "warning",
                    Some(tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION) => "info",
                    Some(tower_lsp::lsp_types::DiagnosticSeverity::HINT) => "hint",
                    Some(_) | None => "info",
                };
                NormalizedSemanticDiagnostic {
                    message: diagnostic.message.clone(),
                    severity: severity.to_string(),
                    start_line: diagnostic.range.start.line,
                    start_character: diagnostic.range.start.character,
                    end_line: diagnostic.range.end.line,
                    end_character: diagnostic.range.end.character,
                }
            })
            .collect();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn normalize_web_semantic_diagnostics(
        payload: &serde_json::Value,
    ) -> Vec<NormalizedSemanticDiagnostic> {
        fn read_u32(diagnostic: &serde_json::Value, key: &str, fallback: Option<&str>) -> u32 {
            diagnostic
                .get(key)
                .or_else(|| fallback.and_then(|alt| diagnostic.get(alt)))
                .and_then(|value| value.as_u64())
                .unwrap_or_default() as u32
        }

        let mut normalized: Vec<NormalizedSemanticDiagnostic> = payload
            .get("semanticErrors")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .map(|diagnostic| NormalizedSemanticDiagnostic {
                message: diagnostic
                    .get("message")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                severity: diagnostic
                    .get("severity")
                    .and_then(|value| value.as_str())
                    .unwrap_or("info")
                    .to_lowercase(),
                start_line: read_u32(diagnostic, "line", None),
                start_character: read_u32(diagnostic, "column", None),
                end_line: read_u32(diagnostic, "endLine", Some("end_line")),
                end_character: read_u32(diagnostic, "endColumn", Some("end_column")),
            })
            .collect();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn normalize_mcp_semantic_diagnostics(
        diagnostics: &[bsl_agent::semantic::dto::DiagnosticDto],
    ) -> Vec<NormalizedSemanticDiagnostic> {
        let mut normalized: Vec<NormalizedSemanticDiagnostic> = diagnostics
            .iter()
            .map(|diagnostic| {
                let severity = match diagnostic.severity {
                    bsl_agent::semantic::dto::DiagnosticSeverityDto::Error => "error",
                    bsl_agent::semantic::dto::DiagnosticSeverityDto::Warning => "warning",
                    bsl_agent::semantic::dto::DiagnosticSeverityDto::Info => "info",
                };
                NormalizedSemanticDiagnostic {
                    message: diagnostic.message.clone(),
                    severity: severity.to_string(),
                    start_line: diagnostic.range.start.line,
                    start_character: diagnostic.range.start.character,
                    end_line: diagnostic.range.end.line,
                    end_character: diagnostic.range.end.character,
                }
            })
            .collect();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct NormalizedSymbol {
        name: String,
        start_line: u32,
        start_character: u32,
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct NormalizedPoint {
        start_line: u32,
        start_character: u32,
    }

    fn normalize_lsp_member_labels(response: &CompletionResponse) -> Vec<String> {
        let items = match response {
            CompletionResponse::Array(items) => items.as_slice(),
            CompletionResponse::List(list) => list.items.as_slice(),
        };
        let mut out: Vec<String> = items
            .iter()
            .filter(|item| {
                matches!(
                    item.kind,
                    Some(CompletionItemKind::METHOD)
                        | Some(CompletionItemKind::PROPERTY)
                        | Some(CompletionItemKind::FIELD)
                        | Some(CompletionItemKind::FUNCTION)
                        | Some(CompletionItemKind::CONSTRUCTOR)
                )
            })
            .map(|item| item.label.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn normalize_mcp_member_labels(members: &[bsl_agent::types::MemberDto]) -> Vec<String> {
        let mut out: Vec<String> = members.iter().map(|member| member.name.clone()).collect();
        out.sort();
        out.dedup();
        out
    }

    fn normalize_lsp_workspace_symbols(symbols: &[SymbolInformation]) -> Vec<NormalizedSymbol> {
        let mut out: Vec<NormalizedSymbol> = symbols
            .iter()
            .map(|symbol| NormalizedSymbol {
                name: symbol.name.clone(),
                start_line: symbol.location.range.start.line,
                start_character: symbol.location.range.start.character,
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn normalize_mcp_workspace_symbols(
        symbols: &[bsl_agent::types::SymbolDto],
    ) -> Vec<NormalizedSymbol> {
        let mut out: Vec<NormalizedSymbol> = symbols
            .iter()
            .map(|symbol| NormalizedSymbol {
                name: symbol.name.clone(),
                start_line: symbol.range.start.line,
                start_character: symbol.range.start.character,
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn normalize_lsp_locations(locations: &[Location]) -> Vec<NormalizedPoint> {
        let mut out: Vec<NormalizedPoint> = locations
            .iter()
            .map(|location| NormalizedPoint {
                start_line: location.range.start.line,
                start_character: location.range.start.character,
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn normalize_mcp_references(
        references: &[bsl_agent::types::ReferenceDto],
    ) -> Vec<NormalizedPoint> {
        let mut out: Vec<NormalizedPoint> = references
            .iter()
            .map(|reference| NormalizedPoint {
                start_line: reference.range.start.line,
                start_character: reference.range.start.character,
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn normalize_lsp_definition(response: Option<GotoDefinitionResponse>) -> Vec<NormalizedPoint> {
        let mut out: Vec<NormalizedPoint> = match response {
            Some(GotoDefinitionResponse::Scalar(location)) => vec![NormalizedPoint {
                start_line: location.range.start.line,
                start_character: location.range.start.character,
            }],
            Some(GotoDefinitionResponse::Array(locations)) => locations
                .into_iter()
                .map(|location| NormalizedPoint {
                    start_line: location.range.start.line,
                    start_character: location.range.start.character,
                })
                .collect(),
            Some(GotoDefinitionResponse::Link(links)) => links
                .into_iter()
                .map(|link| NormalizedPoint {
                    start_line: link.target_range.start.line,
                    start_character: link.target_range.start.character,
                })
                .collect(),
            None => Vec::new(),
        };
        out.sort();
        out.dedup();
        out
    }

    fn normalize_mcp_definition(
        location: Option<&bsl_agent::types::LocationDto>,
    ) -> Vec<NormalizedPoint> {
        let mut out = location
            .map(|location| {
                vec![NormalizedPoint {
                    start_line: location.range.start.line,
                    start_character: location.range.start.character,
                }]
            })
            .unwrap_or_default();
        out.sort();
        out.dedup();
        out
    }

    fn extract_hover_text(hover: Hover) -> Option<String> {
        match hover.contents {
            HoverContents::Scalar(marked) => match marked {
                MarkedString::String(value) => Some(value),
                MarkedString::LanguageString(value) => Some(value.value),
            },
            HoverContents::Array(values) => values
                .into_iter()
                .map(|value| match value {
                    MarkedString::String(value) => Some(value),
                    MarkedString::LanguageString(value) => Some(value.value),
                })
                .next()
                .flatten(),
            HoverContents::Markup(value) => Some(value.value),
        }
    }

    fn metrics_root(payload: &serde_json::Value) -> &serde_json::Value {
        payload.get("metrics").unwrap_or(payload)
    }

    fn stage_from_metric_key(key: &str) -> Option<&'static str> {
        if !key.starts_with("intellisense_v2_") {
            return None;
        }
        if key.contains("runtime_wait_for_file_version") || key.contains("wait_for_file_version_") {
            return Some("runtime_wait_for_file_version");
        }
        if key.contains("runtime_snapshot_with_deps") || key.contains("snapshot_") {
            return Some("runtime_snapshot_with_deps");
        }
        if key.contains("semantic_diagnostics_query") {
            return Some("semantic_diagnostics_query");
        }
        if key.contains("syntax_diagnostics_query") {
            return Some("syntax_diagnostics_query");
        }
        if key.contains("parse_result_query") {
            return Some("parse_result_query");
        }
        if key.contains("ir_query_") {
            return Some("ir_query");
        }
        None
    }

    fn collect_observed_stages(payload: &serde_json::Value) -> BTreeSet<&'static str> {
        let metrics = metrics_root(payload);
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");

        let mut stages = BTreeSet::new();
        for key in counters.keys().chain(histograms.keys()) {
            if let Some(stage) = stage_from_metric_key(key.as_str()) {
                stages.insert(stage);
            }
        }
        stages
    }

    fn metric_number(value: &serde_json::Value) -> f64 {
        if let Some(number) = value.as_f64() {
            return number;
        }
        if let Some(number) = value.as_u64() {
            return number as f64;
        }
        if let Some(number) = value.as_i64() {
            return number as f64;
        }
        0.0
    }

    fn has_positive_counter_for_stage(payload: &serde_json::Value, stage: &str) -> bool {
        let metrics = metrics_root(payload);
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        counters.iter().any(|(key, value)| {
            stage_from_metric_key(key.as_str()) == Some(stage) && metric_number(value) > 0.0
        })
    }

    fn assert_drilldown_stage_metrics_for_origin(payload: &serde_json::Value, origin: &str) {
        let metrics = metrics_root(payload);
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");

        let stage_prefix = format!("intellisense_v2_drilldown_stage_total_origin_{origin}_");
        let latency_prefix = format!("intellisense_v2_drilldown_stage_latency_ms_origin_{origin}_");

        assert!(
            counters.keys().any(|key| key.starts_with(&stage_prefix)),
            "missing drilldown stage_total counters for origin={origin}"
        );
        assert!(
            histograms
                .keys()
                .any(|key| key.starts_with(&latency_prefix)),
            "missing drilldown stage_latency histograms for origin={origin}"
        );
    }

    #[tokio::test]
    async fn p6_fast_did_change_series_publish_diagnostics_is_monotonic() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let (published_tx, mut published_rx) = tokio::sync::mpsc::unbounded_channel::<
            tower_lsp::lsp_types::PublishDiagnosticsParams,
        >();

        let drain_task = tokio::spawn(async move {
            while let Some(req) = socket.next().await {
                if req.method() != "textDocument/publishDiagnostics" {
                    continue;
                }
                let Some(params) = req.params().cloned() else {
                    continue;
                };
                let Ok(parsed) = serde_json::from_value::<
                    tower_lsp::lsp_types::PublishDiagnosticsParams,
                >(params) else {
                    continue;
                };
                let _ = published_tx.send(parsed);
            }
        });

        // LSP initialize handshake is required, otherwise client notifications are suppressed.
        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test.bsl").expect("test uri");

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: "Procedure Test()\nEndProcedure".to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        // Two fast didChange events with different versions. We want to ensure that the server
        // never publishes diagnostics for an older version after a newer one is published.
        let did_change_v2 = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "Procedure Test(\nEndProcedure".to_string(),
            }],
        };
        let did_change_req_v2 = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change_v2).expect("DidChangeTextDocumentParams v2"))
            .finish();
        let did_change_response_v2 = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req_v2)
            .await
            .expect("didChange v2 notification");
        assert!(
            did_change_response_v2.is_none(),
            "didChange is a notification"
        );

        let did_change_v3 = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 3,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "Procedure Test()\nEndProcedure".to_string(),
            }],
        };
        let did_change_req_v3 = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change_v3).expect("DidChangeTextDocumentParams v3"))
            .finish();
        let did_change_response_v3 = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req_v3)
            .await
            .expect("didChange v3 notification");
        assert!(
            did_change_response_v3.is_none(),
            "didChange is a notification"
        );

        let mut versions = Vec::new();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);

        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let Ok(next) = tokio::time::timeout(remaining, published_rx.recv()).await else {
                break;
            };
            let Some(params) = next else {
                break;
            };
            if params.uri != uri {
                continue;
            }
            let Some(version) = params.version else {
                continue;
            };

            versions.push(version);
            if version == 3 {
                break;
            }
        }

        assert!(
            versions.contains(&3),
            "expected diagnostics for version 3 to be published, got {:?}",
            versions
        );

        for pair in versions.windows(2) {
            assert!(
                pair[1] >= pair[0],
                "publishDiagnostics versions must not go backwards: {:?}",
                versions
            );
        }

        // After observing version 3, ensure we don't later publish version 1/2 (no jump-back).
        let after_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(300);
        while tokio::time::Instant::now() < after_deadline {
            let remaining = after_deadline.saturating_duration_since(tokio::time::Instant::now());
            let Ok(next) = tokio::time::timeout(remaining, published_rx.recv()).await else {
                break;
            };
            let Some(params) = next else {
                break;
            };
            if params.uri != uri {
                continue;
            }
            let Some(version) = params.version else {
                continue;
            };
            assert!(
                version >= 3,
                "unexpected jump-back diagnostics: v{}",
                version
            );
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let saw_stale_or_cancelled = counters.iter().any(|(key, value)| {
            key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
                && (key.contains("reason_superseded_version")
                    || key.contains("reason_superseded_generation"))
                && metric_number(value) > 0.0
        });
        assert!(
            saw_stale_or_cancelled
                || counters.iter().any(|(key, value)| {
                    key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
                        && key.contains("reason_cancelled")
                        && metric_number(value) > 0.0
            }),
            "expected diagnostics pipeline metrics to record stale/cancelled runs after rapid didChange series"
        );
        let saw_did_change_fast_profile = counters.iter().any(|(key, value)| {
            key.starts_with(
                "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_change_profile_fast_",
            ) && metric_number(value) > 0.0
        });
        assert!(
            saw_did_change_fast_profile,
            "expected didChange traffic to execute fast diagnostics profile"
        );
        let saw_did_change_idle_heavy_profile = counters.iter().any(|(key, value)| {
            key.starts_with(
                "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_change_profile_idle_heavy_",
            ) && metric_number(value) > 0.0
        });
        assert!(
            !saw_did_change_idle_heavy_profile,
            "idle_heavy diagnostics must not execute under trigger_did_change"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p7_completion_after_did_change_does_not_hang() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p7.bsl").expect("test uri");

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: "Procedure Test()\nEndProcedure".to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
        let mut service = crate::server::request_context::RequestContextService::new(service);

        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "Procedure Test()\n\t// p7\nEndProcedure".to_string(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");

        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };
        let completion_req = Request::build("textDocument/completion")
            .id(2)
            .params(serde_json::to_value(completion_params).expect("CompletionParams"))
            .finish();

        let completion_response = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            service.ready().await.unwrap().call(completion_req),
        )
        .await
        .expect("completion request timeout")
        .expect("completion request");

        assert!(
            completion_response.is_some(),
            "completion should return a response"
        );

        drain_task.abort();
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn p28_cancel_request_stops_completion_and_prevents_late_publish() {
        fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
            match response {
                CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
                CompletionResponse::Array(items) => items.is_empty(),
            }
        }
        struct EnvVarGuard {
            key: &'static str,
            previous: Option<String>,
        }

        impl EnvVarGuard {
            fn set(key: &'static str, value: &str) -> Self {
                let previous = std::env::var(key).ok();
                std::env::set_var(key, value);
                Self { key, previous }
            }
        }

        impl Drop for EnvVarGuard {
            fn drop(&mut self) {
                if let Some(previous) = &self.previous {
                    std::env::set_var(self.key, previous);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }

        static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        let _env_lock = ENV_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("env lock");
        let _delay_guard = EnvVarGuard::set("BSL_TEST_COMPLETION_DELAY_MS", "40");

        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));
        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;
        let mut service = crate::server::request_context::RequestContextService::new(service);

        let uri = Url::parse("file:///test_p28_cancel_request.bsl").expect("test uri");
        let mut base_text = String::from("Процедура Тест()\n    ЛокМассив = Новый Массив;\n");
        for value in 0..800 {
            base_text.push_str(&format!("    ЛокМассив.Добавить({value});\n"));
        }
        base_text.push_str("    ЛокМассив.\nКонецПроцедуры\n");
        let completion_line = 802_u32;
        let completion_character = "    ЛокМассив.".encode_utf16().count() as u32;

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: base_text.clone(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .as_ref()
            .cloned()
            .expect("server instance");
        let file_id = server.get_or_create_file_id_v2(&uri).await;

        let mut observed_cancelled_completion = false;
        for attempt in 0..8_i32 {
            let version = attempt + 2;
            let changed_text = format!("{base_text}// attempt {attempt}\n");
            let did_change = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: changed_text,
                }],
            };
            let did_change_req = Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish();
            let did_change_response = service
                .ready()
                .await
                .unwrap()
                .call(did_change_req)
                .await
                .expect("didChange notification");
            assert!(did_change_response.is_none(), "didChange is a notification");

            let request_id = 100_i64 + i64::from(attempt);
            let completion_params = CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(completion_line, completion_character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: Some("__bsl_shadow_internal__:46".to_string()),
                }),
            };
            let completion_req = Request::build("textDocument/completion")
                .id(request_id)
                .params(serde_json::to_value(completion_params).expect("CompletionParams"))
                .finish();
            let completion_future = service.ready().await.unwrap().call(completion_req);
            let completion_task = tokio::spawn(completion_future);
            let expected_epoch = u64::try_from(attempt + 1).expect("positive epoch");
            let mut before_state = None;
            for _ in 0..100 {
                if let Some((file_seq, epoch)) =
                    server.completion_dispatcher_v2.debug_state(file_id).await
                {
                    if epoch >= expected_epoch {
                        before_state = Some((file_seq, epoch));
                        break;
                    }
                }
                tokio::task::yield_now().await;
            }
            let (before_file_seq, before_epoch) =
                before_state.expect("dispatcher state before cancel");
            let request_id_string = request_id.to_string();
            let mut registration_present = false;
            for _ in 0..20 {
                if server
                    .completion_cancellation_registry_v2
                    .get(&request_id_string)
                    .is_some()
                {
                    registration_present = true;
                    break;
                }
                tokio::task::yield_now().await;
            }

            let cancel_req = Request::build("$/cancelRequest")
                .params(serde_json::json!({ "id": request_id }))
                .finish();
            let cancel_response = service
                .call(cancel_req)
                .await
                .expect("cancel request notification");
            assert!(cancel_response.is_none(), "cancel is a notification");

            let mut cancel_event_observed = false;
            for _ in 0..20 {
                if let Some((after_file_seq, after_epoch)) =
                    server.completion_dispatcher_v2.debug_state(file_id).await
                {
                    if after_file_seq > before_file_seq && after_epoch >= before_epoch {
                        cancel_event_observed = true;
                        break;
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            }

            let completion_response =
                tokio::time::timeout(tokio::time::Duration::from_secs(5), completion_task)
                    .await
                    .expect("completion request timeout")
                    .expect("completion task join")
                    .expect("completion request")
                    .expect("completion response");
            let completion_value =
                serde_json::to_value(&completion_response).expect("serialize completion");
            let completion_is_safe =
                if let Some(completion_result) = completion_value.get("result").cloned() {
                    let completion_lsp: Option<CompletionResponse> =
                        serde_json::from_value(completion_result).expect("parse completion result");
                    completion_lsp
                        .as_ref()
                        .is_some_and(completion_response_incomplete_empty)
                } else if let Some(error) = completion_value.get("error") {
                    let error_code = error
                        .get("code")
                        .and_then(|value| value.as_i64())
                        .unwrap_or_default();
                    let error_message = error
                        .get("message")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                    error_code == -32800 || error_message.contains("cancel")
                } else {
                    false
                };
            if registration_present && cancel_event_observed && completion_is_safe {
                observed_cancelled_completion = true;
                break;
            }
        }

        assert!(
            observed_cancelled_completion,
            "expected $/cancelRequest to enqueue Cancel(request_id) and avoid late completion publish"
        );

        drain_task.abort();
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn p29_completion_mode_matrix_parity_on_fixed_revision() {
        const CHANGE_ID: &str = "refactor-v2-completion-event-driven-pipeline";
        const ITERATIONS: usize = 40;
        const MAX_USER_FACING_DRIFT_RATE: f64 = 0.01;
        const MAX_SHADOW_PARITY_DRIFT_RATE: f64 = 0.01;
        const MIN_FIRST_TRIGGER_SUCCESS_RATE: f64 = 0.99;

        #[derive(Debug, Clone, PartialEq, Eq)]
        struct CompletionFingerprint {
            is_incomplete: bool,
            labels: Vec<String>,
        }

        #[derive(Debug, Clone, Copy)]
        struct ModeScenario {
            name: &'static str,
            completion_mode: &'static str,
            canary_percent: u8,
        }

        #[derive(Debug)]
        struct ModeOutcome {
            name: String,
            completion_p95_ms: f64,
            completion_p99_ms: f64,
            completion_total: u64,
            first_trigger_success_rate: f64,
            parity_drift_rate: f64,
            legacy_stage_total: u64,
            shadow_stage_total: u64,
            event_driven_stage_total: u64,
            dot_fingerprints: Vec<CompletionFingerprint>,
            invoked_fingerprints: Vec<CompletionFingerprint>,
        }

        struct CompletionModeEnvGuard {
            previous_mode: Option<String>,
            previous_canary_percent: Option<String>,
        }

        impl CompletionModeEnvGuard {
            fn new() -> Self {
                Self {
                    previous_mode: std::env::var("BSL_INTELLISENSE_V2_COMPLETION_MODE").ok(),
                    previous_canary_percent: std::env::var(
                        "BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT",
                    )
                    .ok(),
                }
            }

            fn apply(&self, completion_mode: &str, canary_percent: u8) {
                std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_MODE", completion_mode);
                std::env::set_var(
                    "BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT",
                    canary_percent.to_string(),
                );
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
        }

        impl Drop for CompletionModeEnvGuard {
            fn drop(&mut self) {
                if let Some(value) = &self.previous_mode {
                    std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_MODE", value);
                } else {
                    std::env::remove_var("BSL_INTELLISENSE_V2_COMPLETION_MODE");
                }
                if let Some(value) = &self.previous_canary_percent {
                    std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT", value);
                } else {
                    std::env::remove_var("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT");
                }
                bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
            }
        }

        fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
            value
                .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
                .unwrap_or(0.0)
        }

        fn completion_items_count(response: &CompletionResponse) -> usize {
            match response {
                CompletionResponse::Array(items) => items.len(),
                CompletionResponse::List(list) => list.items.len(),
            }
        }

        fn completion_fingerprint(response: &CompletionResponse) -> CompletionFingerprint {
            let (is_incomplete, labels) = match response {
                CompletionResponse::Array(items) => (
                    false,
                    items
                        .iter()
                        .map(|item| item.label.clone())
                        .collect::<BTreeSet<_>>(),
                ),
                CompletionResponse::List(list) => (
                    list.is_incomplete,
                    list.items
                        .iter()
                        .map(|item| item.label.clone())
                        .collect::<BTreeSet<_>>(),
                ),
            };
            CompletionFingerprint {
                is_incomplete,
                labels: labels.into_iter().collect(),
            }
        }

        fn sum_counters_by_prefix(
            counters: &serde_json::Map<String, serde_json::Value>,
            prefix: &str,
        ) -> u64 {
            counters
                .iter()
                .filter(|(key, _)| key.starts_with(prefix))
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        }

        fn completion_stage_mode_total(
            counters: &serde_json::Map<String, serde_json::Value>,
            mode: &str,
        ) -> u64 {
            counters
                .iter()
                .filter(|(key, _)| {
                    key.starts_with("intellisense_v2_drilldown_stage_total_")
                        && key.contains("_origin_lsp_")
                        && key.contains("_operation_completion_")
                        && key.contains(&format!("_mode_{mode}"))
                })
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        }

        async fn run_mode_scenario(scenario: ModeScenario, iterations: usize) -> ModeOutcome {
            let coordinator = Arc::new(SystemCoordinator::new());
            let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
                Arc::new(std::sync::Mutex::new(None));
            let (mut service, mut socket) = LspService::build({
                let coordinator = coordinator.clone();
                let server_holder = server_holder.clone();
                move |client| {
                    let server = BslLanguageServer::new(client, coordinator.clone());
                    *server_holder.lock().expect("server holder lock") = Some(server.clone());
                    server
                }
            })
            .finish();
            let drain_task =
                tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

            initialize_lsp_service(&mut service).await;

            let uri = Url::parse(&format!("file:///test_p29_mode_{}.bsl", scenario.name))
                .expect("test uri");
            let text = concat!(
                "Процедура Тест()\n",
                "    ЛокМассив = Новый Массив;\n",
                "    ЛокМассив.\n",
                "КонецПроцедуры\n"
            );
            let did_open = DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: text.to_string(),
                },
            };
            let did_open_req = Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish();
            let did_open_response = service
                .ready()
                .await
                .unwrap()
                .call(did_open_req)
                .await
                .expect("didOpen notification");
            assert!(did_open_response.is_none(), "didOpen is a notification");

            let server = server_holder
                .lock()
                .expect("server holder lock")
                .as_ref()
                .cloned()
                .expect("server instance");
            let member_character = "    ЛокМассив."
                .chars()
                .map(|ch| ch.len_utf16())
                .sum::<usize>() as u32;

            let mut dot_fingerprints = Vec::with_capacity(iterations);
            let mut invoked_fingerprints = Vec::with_capacity(iterations);
            let mut first_trigger_success_total = 0_u64;

            for _ in 0..iterations {
                let dot_completion = server
                    .completion(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(2, member_character),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context: Some(CompletionContext {
                            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                            trigger_character: Some(".".to_string()),
                        }),
                    })
                    .await
                    .expect("dot completion request")
                    .expect("dot completion response");
                if completion_items_count(&dot_completion) > 0 {
                    first_trigger_success_total += 1;
                }
                dot_fingerprints.push(completion_fingerprint(&dot_completion));

                let invoked_completion = server
                    .completion(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(2, member_character),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context: Some(CompletionContext {
                            trigger_kind: CompletionTriggerKind::INVOKED,
                            trigger_character: None,
                        }),
                    })
                    .await
                    .expect("invoked completion request")
                    .expect("invoked completion response");
                invoked_fingerprints.push(completion_fingerprint(&invoked_completion));
            }

            let metrics = coordinator.observability_metrics();
            let counters = metrics
                .get("counters")
                .and_then(|value| value.as_object())
                .expect("metrics.counters object");
            let histograms = metrics
                .get("histograms")
                .and_then(|value| value.as_object())
                .expect("metrics.histograms object");
            let completion_hist = histograms
                .get("completion_duration_ms")
                .and_then(|value| value.as_object())
                .expect("completion duration histogram");
            let completion_p95_ms = metric_as_f64(completion_hist.get("p95"));
            let completion_p99_ms = metric_as_f64(completion_hist.get("p99"));
            let completion_total = counters
                .get("completion_total")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let parity_pairs_total = (iterations as u64) * 2;
            let parity_drift_total = sum_counters_by_prefix(
                counters,
                "intellisense_v2_completion_parity_drift_total_mode_",
            );
            let parity_drift_rate = parity_drift_total as f64 / parity_pairs_total.max(1) as f64;
            let first_trigger_success_rate =
                first_trigger_success_total as f64 / iterations.max(1) as f64;
            let legacy_stage_total = completion_stage_mode_total(counters, "legacy");
            let shadow_stage_total = completion_stage_mode_total(counters, "shadow");
            let event_driven_stage_total = completion_stage_mode_total(counters, "event_driven");

            drain_task.abort();

            ModeOutcome {
                name: scenario.name.to_string(),
                completion_p95_ms,
                completion_p99_ms,
                completion_total,
                first_trigger_success_rate,
                parity_drift_rate,
                legacy_stage_total,
                shadow_stage_total,
                event_driven_stage_total,
                dot_fingerprints,
                invoked_fingerprints,
            }
        }

        static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        let _env_lock = ENV_LOCK
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("env lock");
        let env_guard = CompletionModeEnvGuard::new();

        let scenarios = [
            ModeScenario {
                name: "off",
                completion_mode: "off",
                canary_percent: 0,
            },
            ModeScenario {
                name: "shadow",
                completion_mode: "shadow",
                canary_percent: 0,
            },
            ModeScenario {
                name: "canary",
                completion_mode: "canary",
                canary_percent: 100,
            },
            ModeScenario {
                name: "on",
                completion_mode: "on",
                canary_percent: 0,
            },
        ];

        let mut outcomes = Vec::with_capacity(scenarios.len());
        for scenario in scenarios {
            env_guard.apply(scenario.completion_mode, scenario.canary_percent);
            let outcome = run_mode_scenario(scenario, ITERATIONS).await;
            assert!(
                outcome.first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE,
                "mode={} first-trigger success rate={:.4} < {:.4}",
                outcome.name,
                outcome.first_trigger_success_rate,
                MIN_FIRST_TRIGGER_SUCCESS_RATE
            );
            outcomes.push(outcome);
        }

        let off_outcome = outcomes
            .iter()
            .find(|outcome| outcome.name == "off")
            .expect("off mode outcome");
        let mut drift_by_mode = serde_json::Map::new();
        for outcome in outcomes.iter().filter(|outcome| outcome.name != "off") {
            let dot_mismatch_total = outcome
                .dot_fingerprints
                .iter()
                .zip(off_outcome.dot_fingerprints.iter())
                .filter(|(actual, expected)| actual != expected)
                .count() as u64;
            let invoked_mismatch_total = outcome
                .invoked_fingerprints
                .iter()
                .zip(off_outcome.invoked_fingerprints.iter())
                .filter(|(actual, expected)| actual != expected)
                .count() as u64;
            let mismatch_total = dot_mismatch_total + invoked_mismatch_total;
            let mismatch_rate = mismatch_total as f64 / ((ITERATIONS * 2) as f64);

            drift_by_mode.insert(
                outcome.name.clone(),
                serde_json::json!({
                    "mismatch_total": mismatch_total,
                    "mismatch_rate": mismatch_rate,
                    "dot_mismatch_total": dot_mismatch_total,
                    "invoked_mismatch_total": invoked_mismatch_total,
                }),
            );
            assert!(
                mismatch_rate <= MAX_USER_FACING_DRIFT_RATE,
                "mode={} user-facing completion drift rate={:.4} > {:.4}",
                outcome.name,
                mismatch_rate,
                MAX_USER_FACING_DRIFT_RATE
            );
        }

        let shadow_outcome = outcomes
            .iter()
            .find(|outcome| outcome.name == "shadow")
            .expect("shadow mode outcome");
        let canary_outcome = outcomes
            .iter()
            .find(|outcome| outcome.name == "canary")
            .expect("canary mode outcome");
        let on_outcome = outcomes
            .iter()
            .find(|outcome| outcome.name == "on")
            .expect("on mode outcome");

        assert!(
            off_outcome.legacy_stage_total > 0
                && off_outcome.shadow_stage_total == 0
                && off_outcome.event_driven_stage_total == 0,
            "off mode stage routing must be strictly legacy: {:?}",
            (
                off_outcome.legacy_stage_total,
                off_outcome.shadow_stage_total,
                off_outcome.event_driven_stage_total
            )
        );
        assert!(
            shadow_outcome.legacy_stage_total > 0
                && shadow_outcome.shadow_stage_total > 0
                && shadow_outcome.event_driven_stage_total == 0,
            "shadow mode must route user-facing via legacy and run shadow pipeline: {:?}",
            (
                shadow_outcome.legacy_stage_total,
                shadow_outcome.shadow_stage_total,
                shadow_outcome.event_driven_stage_total
            )
        );
        assert!(
            shadow_outcome.parity_drift_rate <= MAX_SHADOW_PARITY_DRIFT_RATE,
            "shadow mode parity drift rate={:.4} > {:.4}",
            shadow_outcome.parity_drift_rate,
            MAX_SHADOW_PARITY_DRIFT_RATE
        );
        assert!(
            canary_outcome.event_driven_stage_total > 0
                && canary_outcome.legacy_stage_total == 0
                && canary_outcome.shadow_stage_total == 0,
            "canary(100) mode must route completion via event-driven only: {:?}",
            (
                canary_outcome.legacy_stage_total,
                canary_outcome.shadow_stage_total,
                canary_outcome.event_driven_stage_total
            )
        );
        assert!(
            on_outcome.event_driven_stage_total > 0
                && on_outcome.legacy_stage_total == 0
                && on_outcome.shadow_stage_total == 0,
            "on mode must route completion via event-driven only: {:?}",
            (
                on_outcome.legacy_stage_total,
                on_outcome.shadow_stage_total,
                on_outcome.event_driven_stage_total
            )
        );

        let mut modes_report = serde_json::Map::new();
        for outcome in &outcomes {
            modes_report.insert(
                outcome.name.clone(),
                serde_json::json!({
                    "completion_total": outcome.completion_total,
                    "completion_p95_ms": outcome.completion_p95_ms,
                    "completion_p99_ms": outcome.completion_p99_ms,
                    "first_trigger_success_rate": outcome.first_trigger_success_rate,
                    "parity_drift_rate": outcome.parity_drift_rate,
                    "stage_totals": {
                        "legacy": outcome.legacy_stage_total,
                        "shadow": outcome.shadow_stage_total,
                        "event_driven": outcome.event_driven_stage_total
                    }
                }),
            );
        }
        let report = serde_json::json!({
            "change_id": CHANGE_ID,
            "profile": "p29_completion_mode_matrix_parity_on_fixed_revision",
            "iterations": ITERATIONS,
            "thresholds": {
                "max_user_facing_drift_rate": MAX_USER_FACING_DRIFT_RATE,
                "max_shadow_parity_drift_rate": MAX_SHADOW_PARITY_DRIFT_RATE,
                "min_first_trigger_success_rate": MIN_FIRST_TRIGGER_SUCCESS_RATE
            },
            "mode_user_facing_drift_vs_off": drift_by_mode,
            "modes": serde_json::Value::Object(modes_report),
        });
        let report_path = std::env::var("BSL_V2_COMPLETION_MODE_MATRIX_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!("{CHANGE_ID}-mode-parity-matrix.json"))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for completion mode matrix report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("failed to serialize completion mode matrix report"),
        )
        .expect("failed to write completion mode matrix report");
        println!("v2_completion_mode_matrix_report={}", report_path.display());
    }

    #[tokio::test]
    async fn p30_backpressure_fairness_interactive_vs_background_no_starvation() {
        const CHANGE_ID: &str = "refactor-v2-completion-event-driven-pipeline";
        const INTERACTIVE_PROBE_TOTAL: usize = 24;
        const BACKGROUND_BURST_TOTAL: usize = 24;
        const INTERACTIVE_BURST_TOTAL: usize = 32;
        const BACKGROUND_PROBE_TOTAL: usize = 16;
        const ROUND_TIMEOUT_SECS: u64 = 30;
        const MAX_REQUEST_LATENCY_MS: f64 = 10_000.0;

        async fn run_hover_requests(
            server: BslLanguageServer,
            uri: Url,
            position: Position,
            total: usize,
        ) -> (u64, f64) {
            let mut success_total = 0_u64;
            let mut max_latency_ms = 0.0_f64;
            for _ in 0..total {
                let started = Instant::now();
                let response = server
                    .hover(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .await;
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                max_latency_ms = max_latency_ms.max(elapsed_ms);
                if response.is_ok() {
                    success_total += 1;
                }
            }
            (success_total, max_latency_ms)
        }

        async fn run_hover_burst(
            server: BslLanguageServer,
            uri: Url,
            position: Position,
            total: usize,
        ) -> (u64, f64) {
            let mut handles = Vec::with_capacity(total);
            for _ in 0..total {
                let server = server.clone();
                let uri = uri.clone();
                handles.push(tokio::spawn(async move {
                    let started = Instant::now();
                    let response = server
                        .hover(HoverParams {
                            text_document_position_params: TextDocumentPositionParams {
                                text_document: TextDocumentIdentifier { uri },
                                position,
                            },
                            work_done_progress_params: WorkDoneProgressParams::default(),
                        })
                        .await;
                    (response.is_ok(), started.elapsed().as_secs_f64() * 1000.0)
                }));
            }
            let mut success_total = 0_u64;
            let mut max_latency_ms = 0.0_f64;
            for handle in handles {
                let (ok, latency_ms) = handle.await.expect("hover burst task join");
                if ok {
                    success_total += 1;
                }
                max_latency_ms = max_latency_ms.max(latency_ms);
            }
            (success_total, max_latency_ms)
        }

        async fn run_workspace_symbol_requests(
            server: BslLanguageServer,
            query: String,
            total: usize,
        ) -> (u64, f64) {
            let mut success_total = 0_u64;
            let mut max_latency_ms = 0.0_f64;
            for _ in 0..total {
                let started = Instant::now();
                let response = server
                    .symbol(WorkspaceSymbolParams {
                        query: query.clone(),
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .await;
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                max_latency_ms = max_latency_ms.max(elapsed_ms);
                if response.is_ok() {
                    success_total += 1;
                }
            }
            (success_total, max_latency_ms)
        }

        async fn run_workspace_symbol_burst(
            server: BslLanguageServer,
            query: String,
            total: usize,
        ) -> (u64, f64) {
            let mut handles = Vec::with_capacity(total);
            for _ in 0..total {
                let server = server.clone();
                let query = query.clone();
                handles.push(tokio::spawn(async move {
                    let started = Instant::now();
                    let response = server
                        .symbol(WorkspaceSymbolParams {
                            query,
                            work_done_progress_params: WorkDoneProgressParams::default(),
                            partial_result_params: PartialResultParams::default(),
                        })
                        .await;
                    (response.is_ok(), started.elapsed().as_secs_f64() * 1000.0)
                }));
            }
            let mut success_total = 0_u64;
            let mut max_latency_ms = 0.0_f64;
            for handle in handles {
                let (ok, latency_ms) = handle.await.expect("workspace_symbol burst task join");
                if ok {
                    success_total += 1;
                }
                max_latency_ms = max_latency_ms.max(latency_ms);
            }
            (success_total, max_latency_ms)
        }

        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));
        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let mut primary_uri: Option<Url> = None;
        for index in 0..8_u32 {
            let uri = Url::parse(&format!("file:///test_p30_fairness_{index}.bsl")).expect("uri");
            if primary_uri.is_none() {
                primary_uri = Some(uri.clone());
            }
            let mut text = format!("Процедура Тест{index}()\n    ЛокПерем = Новый Массив;\n");
            for value in 0..120_u32 {
                text.push_str(&format!("    ЛокПерем.Добавить({value});\n"));
            }
            text.push_str("    Возврат ЛокПерем.Количество();\nКонецПроцедуры\n");
            let did_open = DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text,
                },
            };
            let did_open_req = Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish();
            let did_open_response = service
                .ready()
                .await
                .unwrap()
                .call(did_open_req)
                .await
                .expect("didOpen notification");
            assert!(did_open_response.is_none(), "didOpen is a notification");
        }
        let primary_uri = primary_uri.expect("primary uri");
        let hover_position = Position::new(2, 8);

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .as_ref()
            .cloned()
            .expect("server instance");

        let (warm_interactive_success, _) =
            run_hover_requests(server.clone(), primary_uri.clone(), hover_position, 2).await;
        assert!(
            warm_interactive_success > 0,
            "warm-up interactive requests should succeed"
        );
        let (warm_background_success, _) =
            run_workspace_symbol_requests(server.clone(), "Тест".to_string(), 2).await;
        assert!(
            warm_background_success > 0,
            "warm-up background requests should succeed"
        );

        let round_a_background = tokio::spawn(run_workspace_symbol_burst(
            server.clone(),
            "Тест".to_string(),
            BACKGROUND_BURST_TOTAL,
        ));
        let round_a_interactive = tokio::spawn(run_hover_requests(
            server.clone(),
            primary_uri.clone(),
            hover_position,
            INTERACTIVE_PROBE_TOTAL,
        ));
        let (round_a_background_success, round_a_background_max_ms) =
            tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_a_background)
                .await
                .expect("background burst timeout in round A")
                .expect("background burst join in round A");
        let (round_a_interactive_success, round_a_interactive_max_ms) =
            tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_a_interactive)
                .await
                .expect("interactive probe timeout in round A")
                .expect("interactive probe join in round A");

        let round_b_interactive = tokio::spawn(run_hover_burst(
            server.clone(),
            primary_uri.clone(),
            hover_position,
            INTERACTIVE_BURST_TOTAL,
        ));
        let round_b_background = tokio::spawn(run_workspace_symbol_requests(
            server.clone(),
            "Тест".to_string(),
            BACKGROUND_PROBE_TOTAL,
        ));
        let (round_b_interactive_success, round_b_interactive_max_ms) =
            tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_b_interactive)
                .await
                .expect("interactive burst timeout in round B")
                .expect("interactive burst join in round B");
        let (round_b_background_success, round_b_background_max_ms) =
            tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_b_background)
                .await
                .expect("background probe timeout in round B")
                .expect("background probe join in round B");

        assert_eq!(
            round_a_interactive_success, INTERACTIVE_PROBE_TOTAL as u64,
            "interactive requests must progress under background burst"
        );
        assert_eq!(
            round_a_background_success, BACKGROUND_BURST_TOTAL as u64,
            "background burst must complete without starvation"
        );
        assert_eq!(
            round_b_interactive_success, INTERACTIVE_BURST_TOTAL as u64,
            "interactive burst must complete under mixed load"
        );
        assert_eq!(
            round_b_background_success, BACKGROUND_PROBE_TOTAL as u64,
            "background probe must progress under interactive burst"
        );
        for (name, value) in [
            ("round_a_background_max_ms", round_a_background_max_ms),
            ("round_a_interactive_max_ms", round_a_interactive_max_ms),
            ("round_b_background_max_ms", round_b_background_max_ms),
            ("round_b_interactive_max_ms", round_b_interactive_max_ms),
        ] {
            assert!(
                value <= MAX_REQUEST_LATENCY_MS,
                "{name} exceeded bounded latency: {value:.2}ms > {MAX_REQUEST_LATENCY_MS:.2}ms"
            );
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let interactive_queue_wait_total = counters
            .get("intellisense_v2_runtime_queue_wait_interactive_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let background_queue_wait_total = counters
            .get("intellisense_v2_runtime_queue_wait_background_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let interactive_exec_total = counters
            .get("intellisense_v2_runtime_exec_interactive_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let background_exec_total = counters
            .get("intellisense_v2_runtime_exec_background_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);

        assert!(
            interactive_queue_wait_total > 0,
            "interactive queue-wait counter must be present under mixed load"
        );
        assert!(
            background_queue_wait_total > 0,
            "background queue-wait counter must be present under mixed load"
        );
        assert!(
            interactive_exec_total > 0,
            "interactive exec counter must be present under mixed load"
        );
        assert!(
            background_exec_total > 0,
            "background exec counter must be present under mixed load"
        );

        let report = serde_json::json!({
            "change_id": CHANGE_ID,
            "profile": "p30_backpressure_fairness_interactive_vs_background_no_starvation",
            "thresholds": {
                "round_timeout_secs": ROUND_TIMEOUT_SECS,
                "max_request_latency_ms": MAX_REQUEST_LATENCY_MS,
            },
            "rounds": {
                "background_burst_vs_interactive_probe": {
                    "interactive_total": INTERACTIVE_PROBE_TOTAL,
                    "interactive_success": round_a_interactive_success,
                    "interactive_max_latency_ms": round_a_interactive_max_ms,
                    "background_total": BACKGROUND_BURST_TOTAL,
                    "background_success": round_a_background_success,
                    "background_max_latency_ms": round_a_background_max_ms,
                },
                "interactive_burst_vs_background_probe": {
                    "interactive_total": INTERACTIVE_BURST_TOTAL,
                    "interactive_success": round_b_interactive_success,
                    "interactive_max_latency_ms": round_b_interactive_max_ms,
                    "background_total": BACKGROUND_PROBE_TOTAL,
                    "background_success": round_b_background_success,
                    "background_max_latency_ms": round_b_background_max_ms,
                }
            },
            "metrics": {
                "intellisense_v2_runtime_queue_wait_interactive_total": interactive_queue_wait_total,
                "intellisense_v2_runtime_queue_wait_background_total": background_queue_wait_total,
                "intellisense_v2_runtime_exec_interactive_total": interactive_exec_total,
                "intellisense_v2_runtime_exec_background_total": background_exec_total,
            },
            "pass": true
        });
        let report_path = std::env::var("BSL_V2_COMPLETION_FAIRNESS_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!(
                        "{CHANGE_ID}-fairness-interactive-vs-background.json"
                    ))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for completion fairness report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("failed to serialize completion fairness report"),
        )
        .expect("failed to write completion fairness report");
        println!("v2_completion_fairness_report={}", report_path.display());

        drain_task.abort();
    }

    #[tokio::test]
    async fn p7_trigger_character_and_invoked_member_access_keep_semantic_parity() {
        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let uri = Url::parse("file:///test_p7_trigger_parity.bsl").expect("test uri");
        let text = concat!(
            "Процедура Тест()\n",
            "    ЛокМассив = Новый Массив;\n",
            "    ЛокМассив.\n",
            "КонецПроцедуры\n"
        );
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .clone()
            .expect("server must be created");
        let member_character = "    ЛокМассив."
            .chars()
            .map(|ch| ch.len_utf16())
            .sum::<usize>() as u32;
        let dot_response = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(2, member_character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                    trigger_character: Some(".".to_string()),
                }),
            })
            .await
            .expect("dot completion request")
            .expect("dot completion response");

        let invoked_response = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(2, member_character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
            })
            .await
            .expect("invoked completion request")
            .expect("invoked completion response");

        let extract_labels = |response: &CompletionResponse| -> Vec<String> {
            match response {
                CompletionResponse::Array(items) => {
                    items.iter().map(|item| item.label.clone()).collect()
                }
                CompletionResponse::List(list) => {
                    list.items.iter().map(|item| item.label.clone()).collect()
                }
            }
        };
        let dot_members = extract_labels(&dot_response);
        let invoked_members = extract_labels(&invoked_response);
        assert!(
            !dot_members.is_empty(),
            "trigger-character completion must return candidates"
        );
        assert!(
            !invoked_members.is_empty(),
            "invoked completion must return candidates"
        );
        assert!(
            dot_members.iter().any(|label| invoked_members.contains(label)),
            "trigger-character and invoked completion must have semantic overlap: dot={:?} invoked={:?}",
            dot_members,
            invoked_members
        );

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let trigger_char_total = counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_character")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let invoked_total = counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_invoked")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let overlap_metric_recorded = counters.iter().any(|(key, value)| {
            key.starts_with("intellisense_v2_completion_parity_overlap_total_mode_invoked_bucket_")
                && value.as_u64().unwrap_or(0) > 0
        });
        assert!(
            trigger_char_total > 0,
            "trigger-character completion metric must be recorded"
        );
        assert!(
            invoked_total > 0,
            "invoked completion metric must be recorded"
        );
        assert!(
            overlap_metric_recorded,
            "semantic-overlap parity metric must be recorded"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p7_completion_context_modes_are_supported() {
        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let uri = Url::parse("file:///test_p7_completion_context_modes.bsl").expect("test uri");
        let text =
            "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .clone()
            .expect("server must be created");
        let member_character = "    ЛокМассив."
            .chars()
            .map(|ch| ch.len_utf16())
            .sum::<usize>() as u32;
        let base_params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(2, member_character),
        };

        let contexts = [
            Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
            Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
            Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS,
                trigger_character: None,
            }),
            None,
        ];

        for context in contexts {
            let response = server
                .completion(CompletionParams {
                    text_document_position: base_params.clone(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context,
                })
                .await
                .expect("completion request");
            assert!(response.is_some(), "completion response must be present");
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        assert!(
            counters
                .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_character")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        );
        assert!(
            counters
                .get("intellisense_v2_completion_trigger_mode_total_mode_invoked")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        );
        assert!(
            counters
                .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_for_incomplete")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        );
        assert!(
            counters
                .get("intellisense_v2_completion_trigger_mode_total_mode_none")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p7_first_completion_after_received_advance_uses_stale_non_empty_result() {
        const STALE_FIXTURE: &str =
            "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";

        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let uri = Url::parse("file:///test_p7_stale_completion.bsl").expect("test uri");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: STALE_FIXTURE.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: STALE_FIXTURE.to_string(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .clone()
            .expect("server must be created");
        server
            .deps_update_v2("p7_stale_completion_setup", None, None)
            .await;
        server.sync_v2_globals().await;

        let file_id = server.get_or_create_file_id_v2(&uri).await;
        {
            let mut versions = server.latest_received_file_versions_v2.write().await;
            // Simulate the window right after the next didChange was received but before runtime apply.
            versions.insert(file_id, 3);
        }

        let completion = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(2, 13),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .expect("completion request")
            .expect("completion response");

        match completion {
            CompletionResponse::List(list) => {
                assert!(
                    list.is_incomplete,
                    "stale fallback completion must be marked isIncomplete=true"
                );
                assert!(
                    !list.items.is_empty(),
                    "first completion after received-version advance must not be empty"
                );
            }
            CompletionResponse::Array(items) => {
                assert!(
                    !items.is_empty(),
                    "first completion after received-version advance must not be empty"
                );
                panic!("completion fallback must return CompletionList with isIncomplete=true");
            }
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let stale_fallback_total = counters
            .get("intellisense_v2_completion_stale_fallback_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            stale_fallback_total > 0,
            "expected stale fallback counter to be incremented"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p8_deps_update_is_atomic_and_completion_uses_runtime_index_snapshot() {
        fn make_index_snapshot(id: &str, type_name: &str) -> IndexSnapshot {
            let mut snapshot = IndexSnapshot::empty(IndexSnapshotId::from_hash(id.to_string()));
            Arc::make_mut(&mut snapshot.type_index).insert(
                type_name.to_string(),
                Arc::new(IndexItem::new(
                    type_name.to_string(),
                    IndexItemKind::Type(TypeKind::Generic),
                    IndexKind::Type,
                )),
            );
            snapshot
        }

        fn extract_completion_labels(
            response: tower_lsp::lsp_types::CompletionResponse,
        ) -> Vec<String> {
            match response {
                tower_lsp::lsp_types::CompletionResponse::Array(items) => {
                    items.into_iter().map(|item| item.label).collect()
                }
                tower_lsp::lsp_types::CompletionResponse::List(list) => {
                    list.items.into_iter().map(|item| item.label).collect()
                }
            }
        }

        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().unwrap() = Some(server.clone());
                server
            }
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let server = server_holder
            .lock()
            .unwrap()
            .clone()
            .expect("server must be created");

        let snapshot_a = make_index_snapshot("p8_a", "P8TypeA");
        let snapshot_b = make_index_snapshot("p8_b", "P8TypeB");

        coordinator
            .intellisense_index()
            .replace_snapshot(snapshot_a.clone());
        let expected_deps_id_a = build_deps_bundle_v2(coordinator.as_ref(), None, None)
            .expect("bundle A")
            .deps_id;

        coordinator
            .intellisense_index()
            .replace_snapshot(snapshot_b.clone());
        let expected_deps_id_b = build_deps_bundle_v2(coordinator.as_ref(), None, None)
            .expect("bundle B")
            .deps_id;

        coordinator
            .intellisense_index()
            .replace_snapshot(snapshot_a.clone());
        server.deps_update_v2("p8_test_initial", None, None).await;

        let uri = Url::parse("file:///test_p8.bsl").expect("test uri");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: "Procedure Test()\n\t// P8\nEndProcedure".to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 1,
                    character: 6,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let completion_a = server
            .completion(completion_params.clone())
            .await
            .expect("completion")
            .expect("completion response");
        let labels_a = extract_completion_labels(completion_a);
        assert!(
            labels_a.iter().any(|label| label == "P8TypeA"),
            "expected completion to contain P8TypeA, got {:?}",
            labels_a
        );
        assert!(
            labels_a.iter().all(|label| label != "P8TypeB"),
            "unexpected P8TypeB in completion A: {:?}",
            labels_a
        );

        let update_task = tokio::spawn({
            let coordinator = coordinator.clone();
            let server = server.clone();
            let snapshot_b = snapshot_b.clone();
            async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                coordinator
                    .intellisense_index()
                    .replace_snapshot(snapshot_b);
                server.deps_update_v2("p8_test_update", None, None).await;
            }
        });

        for _ in 0..200 {
            let (_analysis, index_snapshot, deps_id) =
                server.analysis_v2.snapshot_with_deps().await;
            match index_snapshot.id.as_str() {
                "p8_a" => assert_eq!(deps_id.as_str(), expected_deps_id_a.as_str()),
                "p8_b" => assert_eq!(deps_id.as_str(), expected_deps_id_b.as_str()),
                other => panic!("unexpected index snapshot id: {}", other),
            }
        }

        update_task.await.expect("update task join");

        let completion_b = server
            .completion(completion_params)
            .await
            .expect("completion")
            .expect("completion response");
        let labels_b = extract_completion_labels(completion_b);
        assert!(
            labels_b.iter().any(|label| label == "P8TypeB"),
            "expected completion to contain P8TypeB, got {:?}",
            labels_b
        );
        assert!(
            labels_b.iter().all(|label| label != "P8TypeA"),
            "unexpected P8TypeA in completion B: {:?}",
            labels_b
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p9a_formatting_disabled_does_not_advertise_capability_and_returns_null() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        let response = response.expect("initialize should return a response");

        let response_value =
            serde_json::to_value(&response).expect("serialize initialize response");
        let capabilities = response_value
            .get("result")
            .and_then(|v| v.get("capabilities"))
            .expect("initialize capabilities");

        match capabilities.get("documentFormattingProvider") {
            None => {}
            Some(v) => assert!(
                v.is_null(),
                "documentFormattingProvider must be absent/null"
            ),
        }
        match capabilities.get("documentRangeFormattingProvider") {
            None => {}
            Some(v) => assert!(
                v.is_null(),
                "documentRangeFormattingProvider must be absent/null"
            ),
        }

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p9a_formatting_disabled.bsl").expect("test uri");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: "Процедура Тест()\nКонецПроцедуры\n".to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let formatting_params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            options: FormattingOptions::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let formatting_req = Request::build("textDocument/formatting")
            .id(2)
            .params(serde_json::to_value(formatting_params).expect("DocumentFormattingParams"))
            .finish();
        let formatting_response = service
            .ready()
            .await
            .unwrap()
            .call(formatting_req)
            .await
            .expect("formatting request");
        let formatting_response = formatting_response.expect("formatting should return a response");

        let response_value =
            serde_json::to_value(&formatting_response).expect("serialize formatting response");
        match response_value.get("error") {
            None => {}
            Some(v) => assert!(v.is_null(), "formatting must not return an error"),
        }
        let result = response_value
            .get("result")
            .cloned()
            .expect("formatting result field");
        assert!(
            result.is_null(),
            "disabled formatting should return null edits"
        );

        let range_formatting_params = DocumentRangeFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            options: FormattingOptions::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let range_req = Request::build("textDocument/rangeFormatting")
            .id(3)
            .params(
                serde_json::to_value(range_formatting_params)
                    .expect("DocumentRangeFormattingParams"),
            )
            .finish();
        let range_response = service
            .ready()
            .await
            .unwrap()
            .call(range_req)
            .await
            .expect("rangeFormatting request");
        let range_response = range_response.expect("rangeFormatting should return a response");

        let range_value =
            serde_json::to_value(&range_response).expect("serialize rangeFormatting response");
        match range_value.get("error") {
            None => {}
            Some(v) => assert!(v.is_null(), "rangeFormatting must not return an error"),
        }
        let range_result = range_value
            .get("result")
            .cloned()
            .expect("rangeFormatting result field");
        assert!(
            range_result.is_null(),
            "disabled rangeFormatting should return null edits"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p9_formatting_reindents_and_trims_when_enabled() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        // LSP initialize handshake is required, otherwise client notifications are suppressed.
        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        // Enable formatting through didChangeConfiguration (section `bsl`).
        let settings = DidChangeConfigurationParams {
            settings: serde_json::json!({
                "bsl": {
                    "hover": {
                        "detailLevel": "full",
                        "maxMethods": 10,
                        "maxProperties": 5,
                        "showCertainty": true
                    },
                    "diagnostics": {
                        "detailLevel": "standard",
                        "showHints": true
                    },
                    "formatting": {
                        "enabled": true,
                        "indentSize": 4
                    }
                }
            }),
        };
        let settings_req = Request::build("workspace/didChangeConfiguration")
            .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
            .finish();
        let settings_resp = service
            .ready()
            .await
            .unwrap()
            .call(settings_req)
            .await
            .expect("didChangeConfiguration notification");
        assert!(
            settings_resp.is_none(),
            "didChangeConfiguration is a notification"
        );

        let uri = Url::parse("file:///test_p9_formatting.bsl").expect("test uri");
        let text = "Процедура Тест()\nЕсли Истина Тогда  \nСообщить(1);\nИначе\nСообщить(2);   \nКонецЕсли;\nКонецПроцедуры\n";

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let formatting_params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            options: FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let formatting_req = Request::build("textDocument/formatting")
            .id(2)
            .params(serde_json::to_value(formatting_params).expect("DocumentFormattingParams"))
            .finish();
        let formatting_response = service
            .ready()
            .await
            .unwrap()
            .call(formatting_req)
            .await
            .expect("formatting request");
        let formatting_response = formatting_response.expect("formatting should return a response");

        let response_value =
            serde_json::to_value(&formatting_response).expect("serialize formatting response");
        let edits_value = response_value
            .get("result")
            .cloned()
            .expect("formatting result field");
        let edits: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
            serde_json::from_value(edits_value).expect("parse edits");
        let edits = edits.expect("edits present");
        assert!(!edits.is_empty(), "formatting must return edits");

        // Apply per-line edits (formatter emits full-line replacements).
        let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        for edit in edits {
            let line = edit.range.start.line as usize;
            lines[line] = edit.new_text;
        }
        let formatted = lines.join("\n");

        let expected = "Процедура Тест()\n    Если Истина Тогда\n        Сообщить(1);\n    Иначе\n        Сообщить(2);\n    КонецЕсли;\nКонецПроцедуры\n";
        assert_eq!(formatted, expected);

        drain_task.abort();
    }

    #[tokio::test]
    async fn p10_range_formatting_only_updates_selected_lines() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let settings = DidChangeConfigurationParams {
            settings: serde_json::json!({
                "bsl": {
                    "hover": {
                        "detailLevel": "full",
                        "maxMethods": 10,
                        "maxProperties": 5,
                        "showCertainty": true
                    },
                    "diagnostics": {
                        "detailLevel": "standard",
                        "showHints": true
                    },
                    "formatting": {
                        "enabled": true,
                        "indentSize": 4
                    }
                }
            }),
        };
        let settings_req = Request::build("workspace/didChangeConfiguration")
            .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
            .finish();
        let settings_resp = service
            .ready()
            .await
            .unwrap()
            .call(settings_req)
            .await
            .expect("didChangeConfiguration notification");
        assert!(
            settings_resp.is_none(),
            "didChangeConfiguration is a notification"
        );

        let uri = Url::parse("file:///test_p10_range_formatting.bsl").expect("test uri");
        let text = concat!(
            "Процедура Тест()\n",
            "    Сообщить(\"a\");\n",
            "Если Истина Тогда\n",
            "Сообщить(1);\n",
            "КонецЕсли;\n",
            "    Сообщить(\"b\");\n",
            "КонецПроцедуры\n",
        );

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let range_formatting_params = DocumentRangeFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range {
                start: Position {
                    line: 2,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
            },
            options: FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let range_req = Request::build("textDocument/rangeFormatting")
            .id(2)
            .params(
                serde_json::to_value(range_formatting_params)
                    .expect("DocumentRangeFormattingParams"),
            )
            .finish();

        let response_a = service
            .ready()
            .await
            .unwrap()
            .call(range_req)
            .await
            .expect("rangeFormatting request");
        let response_a = response_a.expect("rangeFormatting should return a response");

        let response_value =
            serde_json::to_value(&response_a).expect("serialize rangeFormatting response");
        let edits_value = response_value
            .get("result")
            .cloned()
            .expect("rangeFormatting result field");
        let edits: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
            serde_json::from_value(edits_value).expect("parse edits");
        let edits = edits.expect("edits present");

        assert_eq!(edits.len(), 3, "expected 3 line edits inside the range");
        for edit in &edits {
            assert!(
                (2..=4).contains(&edit.range.start.line),
                "unexpected edit line {:?}",
                edit.range.start.line
            );
        }
        let projected_a: Vec<(u32, String)> = edits
            .iter()
            .map(|edit| (edit.range.start.line, edit.new_text.clone()))
            .collect();

        // Apply per-line edits.
        let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        for edit in edits {
            let line = edit.range.start.line as usize;
            lines[line] = edit.new_text;
        }
        let formatted = lines.join("\n");

        let expected = concat!(
            "Процедура Тест()\n",
            "    Сообщить(\"a\");\n",
            "    Если Истина Тогда\n",
            "        Сообщить(1);\n",
            "    КонецЕсли;\n",
            "    Сообщить(\"b\");\n",
            "КонецПроцедуры\n",
        );
        assert_eq!(formatted, expected);

        // Determinism: second request returns identical edits.
        let range_req_2 = Request::build("textDocument/rangeFormatting")
            .id(3)
            .params(
                serde_json::to_value(DocumentRangeFormattingParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    range: Range {
                        start: Position {
                            line: 2,
                            character: 0,
                        },
                        end: Position {
                            line: 5,
                            character: 0,
                        },
                    },
                    options: FormattingOptions {
                        tab_size: 4,
                        insert_spaces: true,
                        ..Default::default()
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .expect("DocumentRangeFormattingParams"),
            )
            .finish();

        let response_b = service
            .ready()
            .await
            .unwrap()
            .call(range_req_2)
            .await
            .expect("rangeFormatting request (2)");
        let response_b = response_b.expect("rangeFormatting (2) should return a response");

        let value_b = serde_json::to_value(&response_b).expect("serialize response");
        let edits_b_value = value_b.get("result").cloned().expect("result field");
        let edits_b: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
            serde_json::from_value(edits_b_value).expect("parse edits");
        let edits_b = edits_b.expect("edits present");
        let projected_b: Vec<(u32, String)> = edits_b
            .iter()
            .map(|edit| (edit.range.start.line, edit.new_text.clone()))
            .collect();
        assert_eq!(
            projected_b, projected_a,
            "range formatting must be deterministic"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p11_document_symbol_groups_routines_by_region() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p11_document_symbol.bsl").expect("test uri");
        let text = concat!(
            "#Область Public\n",
            "Процедура Inside() Экспорт\n",
            "КонецПроцедуры\n",
            "#КонецОбласти\n",
            "Функция Outside() Экспорт\n",
            "КонецФункции\n",
        );

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };

        let req = Request::build("textDocument/documentSymbol")
            .id(2)
            .params(serde_json::to_value(params.clone()).expect("DocumentSymbolParams"))
            .finish();
        let response_a = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("documentSymbol request");
        let response_a = response_a.expect("documentSymbol should return a response");

        let value_a = serde_json::to_value(&response_a).expect("serialize response");
        let result_a_value = value_a.get("result").cloned().expect("result field");

        let parsed_a: Option<DocumentSymbolResponse> =
            serde_json::from_value(result_a_value.clone()).expect("parse result");
        let parsed_a = parsed_a.expect("result present");

        let DocumentSymbolResponse::Nested(top_level) = parsed_a else {
            panic!("expected nested document symbols");
        };

        let region = top_level
            .iter()
            .find(|sym| sym.name == "Public")
            .expect("expected region Public");
        assert_eq!(region.kind, SymbolKind::NAMESPACE);

        let children = region.children.as_ref().expect("region must have children");
        let inside = children
            .iter()
            .find(|sym| sym.name == "Inside")
            .expect("expected Inside");
        assert_eq!(inside.kind, SymbolKind::METHOD);
        assert_eq!(inside.detail.as_deref(), Some("export"));
        assert_eq!(inside.range.start.line, 1);
        assert_eq!(inside.selection_range.start.line, 1);
        assert_eq!(inside.selection_range.start.character, 10);
        assert_eq!(inside.selection_range.end.character, 16);

        let outside = top_level
            .iter()
            .find(|sym| sym.name == "Outside")
            .expect("expected Outside");
        assert_eq!(outside.kind, SymbolKind::FUNCTION);
        assert_eq!(outside.detail.as_deref(), Some("export"));
        assert_eq!(outside.selection_range.start.line, 4);
        assert_eq!(outside.selection_range.start.character, 8);
        assert_eq!(outside.selection_range.end.character, 15);

        // Determinism: second request returns identical JSON result.
        let req_2 = Request::build("textDocument/documentSymbol")
            .id(3)
            .params(serde_json::to_value(params).expect("DocumentSymbolParams"))
            .finish();
        let response_b = service
            .ready()
            .await
            .unwrap()
            .call(req_2)
            .await
            .expect("documentSymbol request (2)");
        let response_b = response_b.expect("documentSymbol (2) should return a response");
        let value_b = serde_json::to_value(&response_b).expect("serialize response");
        let result_b_value = value_b.get("result").cloned().expect("result field");
        assert_eq!(
            result_a_value, result_b_value,
            "documentSymbol must be deterministic"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p12_workspace_symbol_searches_open_documents() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri_a = Url::parse("file:///test_p12_a.bsl").expect("test uri a");
        let uri_b = Url::parse("file:///test_p12_b.bsl").expect("test uri b");

        let did_open_a = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri_a.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: "Процедура FooOne() Экспорт\nКонецПроцедуры\n".to_string(),
            },
        };
        let did_open_b = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri_b.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: "Функция FooTwo() Экспорт\nКонецФункции\n".to_string(),
            },
        };

        for did_open in [did_open_a, did_open_b] {
            let req = Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish();
            let resp = service
                .ready()
                .await
                .unwrap()
                .call(req)
                .await
                .expect("didOpen notification");
            assert!(resp.is_none(), "didOpen is a notification");
        }

        let params = WorkspaceSymbolParams {
            query: "Foo".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let req = Request::build("workspace/symbol")
            .id(2)
            .params(serde_json::to_value(params).expect("WorkspaceSymbolParams"))
            .finish();

        let response = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("workspace/symbol request");
        let response = response.expect("workspace/symbol should return a response");

        let value = serde_json::to_value(&response).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<Vec<SymbolInformation>> =
            serde_json::from_value(result_value).expect("parse result");
        let parsed = parsed.expect("result present");

        assert!(
            parsed
                .iter()
                .any(|sym| sym.name == "FooOne" && sym.location.uri == uri_a),
            "expected FooOne in uri_a, got {:?}",
            parsed
                .iter()
                .map(|s| (s.name.clone(), s.location.uri.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            parsed
                .iter()
                .any(|sym| sym.name == "FooTwo" && sym.location.uri == uri_b),
            "expected FooTwo in uri_b, got {:?}",
            parsed
                .iter()
                .map(|s| (s.name.clone(), s.location.uri.clone()))
                .collect::<Vec<_>>()
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p13_unclosed_region_is_closed_at_eof() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p13_unclosed_region.bsl").expect("test uri");
        let text = concat!(
            "#Область Unclosed\n",
            "Процедура Inside() Экспорт\n",
            "КонецПроцедуры\n",
        );

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let req = Request::build("textDocument/documentSymbol")
            .id(2)
            .params(
                serde_json::to_value(DocumentSymbolParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .expect("DocumentSymbolParams"),
            )
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("documentSymbol request");
        let response = response.expect("documentSymbol should return a response");

        let value = serde_json::to_value(&response).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<DocumentSymbolResponse> =
            serde_json::from_value(result_value).expect("parse result");
        let parsed = parsed.expect("result present");

        let DocumentSymbolResponse::Nested(top_level) = parsed else {
            panic!("expected nested document symbols");
        };

        let region = top_level
            .iter()
            .find(|sym| sym.name == "Unclosed")
            .expect("expected region Unclosed");
        assert_eq!(region.kind, SymbolKind::NAMESPACE);
        assert_eq!(
            region.range.end,
            Position {
                line: 3,
                character: 0,
            },
            "unclosed region should be closed at EOF"
        );

        let children = region.children.as_ref().expect("region must have children");
        assert!(
            children.iter().any(|sym| sym.name == "Inside"),
            "expected Inside inside Unclosed region"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p14_references_returns_local_var_locations_and_respects_include_declaration() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p14_references.bsl").expect("test uri");
        let text = concat!(
            "Процедура T()\n",
            "    Перем X;\n",
            "    X = 1;\n",
            "    Сообщить(X);\n",
            "КонецПроцедуры\n",
        );

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let params_with_decl = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 2,
                    character: 4,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let req_with_decl = Request::build("textDocument/references")
            .id(2)
            .params(serde_json::to_value(params_with_decl).expect("ReferenceParams"))
            .finish();
        let response_with_decl = service
            .ready()
            .await
            .unwrap()
            .call(req_with_decl)
            .await
            .expect("references request");
        let response_with_decl = response_with_decl.expect("references should return a response");

        let value = serde_json::to_value(&response_with_decl).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<Vec<Location>> =
            serde_json::from_value(result_value).expect("parse result");
        let parsed = parsed.expect("result present");

        assert_eq!(parsed.len(), 3, "expected declaration + 2 usages");
        assert!(
            parsed.iter().any(|loc| loc.range
                == Range {
                    start: Position {
                        line: 1,
                        character: 10
                    },
                    end: Position {
                        line: 1,
                        character: 11
                    }
                }),
            "expected declaration location for X"
        );
        assert!(
            parsed.iter().any(|loc| loc.range
                == Range {
                    start: Position {
                        line: 2,
                        character: 4
                    },
                    end: Position {
                        line: 2,
                        character: 5
                    }
                }),
            "expected assignment target usage for X"
        );
        assert!(
            parsed.iter().any(|loc| loc.range
                == Range {
                    start: Position {
                        line: 3,
                        character: 13
                    },
                    end: Position {
                        line: 3,
                        character: 14
                    }
                }),
            "expected call argument usage for X"
        );

        let params_no_decl = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 2,
                    character: 4,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: false,
            },
        };

        let req_no_decl = Request::build("textDocument/references")
            .id(3)
            .params(serde_json::to_value(params_no_decl).expect("ReferenceParams"))
            .finish();
        let response_no_decl = service
            .ready()
            .await
            .unwrap()
            .call(req_no_decl)
            .await
            .expect("references request (no decl)");
        let response_no_decl =
            response_no_decl.expect("references (no decl) should return a response");
        let value = serde_json::to_value(&response_no_decl).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<Vec<Location>> =
            serde_json::from_value(result_value).expect("parse result");
        let parsed = parsed.expect("result present");
        assert_eq!(parsed.len(), 2, "expected 2 usages without declaration");

        drain_task.abort();
    }

    #[tokio::test]
    async fn p15_rename_updates_only_target_symbol_and_prepare_rename_is_supported() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p15_rename.bsl").expect("test uri");
        let text = concat!(
            "Процедура T()\n",
            "    Перем X;\n",
            "    Перем XX;\n",
            "    X = 1;\n",
            "    XX = 2;\n",
            "    Сообщить(X);\n",
            "    Сообщить(XX);\n",
            "КонецПроцедуры\n",
        );

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let prepare_req = Request::build("textDocument/prepareRename")
            .id(2)
            .params(
                serde_json::to_value(TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position {
                        line: 5,
                        character: 13,
                    },
                })
                .expect("TextDocumentPositionParams"),
            )
            .finish();
        let prepare_resp = service
            .ready()
            .await
            .unwrap()
            .call(prepare_req)
            .await
            .expect("prepareRename request");
        let prepare_resp = prepare_resp.expect("prepareRename should return a response");
        let value = serde_json::to_value(&prepare_resp).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<PrepareRenameResponse> =
            serde_json::from_value(result_value).expect("parse prepareRename");
        let parsed = parsed.expect("result present");
        match parsed {
            PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } => {
                assert_eq!(placeholder, "X");
                assert_eq!(
                    range,
                    Range {
                        start: Position {
                            line: 5,
                            character: 13
                        },
                        end: Position {
                            line: 5,
                            character: 14
                        }
                    }
                );
            }
            other => panic!("unexpected prepareRename response: {:?}", other),
        }

        let rename_params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 5,
                    character: 13,
                },
            },
            new_name: "Y".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let rename_req = Request::build("textDocument/rename")
            .id(3)
            .params(serde_json::to_value(rename_params).expect("RenameParams"))
            .finish();

        let rename_resp = service
            .ready()
            .await
            .unwrap()
            .call(rename_req)
            .await
            .expect("rename request");
        let rename_resp = rename_resp.expect("rename should return a response");

        let value = serde_json::to_value(&rename_resp).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<WorkspaceEdit> =
            serde_json::from_value(result_value).expect("parse workspace edit");
        let parsed = parsed.expect("result present");
        let changes = parsed.changes.expect("changes present");
        let edits = changes.get(&uri).expect("edits for uri");
        assert_eq!(edits.len(), 3, "expected declaration + 2 usages for X");
        assert!(
            edits.iter().all(|e| e.new_text == "Y"),
            "all edits must rename to Y"
        );
        assert!(
            edits.iter().all(|e| e.range.start.line != 2),
            "must not touch XX declaration line"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p16_references_returns_routine_declaration_and_calls() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p16_routine_references.bsl").expect("test uri");
        let text = concat!(
            "Процедура Foo() Экспорт\n",
            "КонецПроцедуры\n",
            "\n",
            "Процедура Bar()\n",
            "    Foo();\n",
            "    Foo();\n",
            "КонецПроцедуры\n",
        );

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 4,
                    character: 4,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let req = Request::build("textDocument/references")
            .id(2)
            .params(serde_json::to_value(params).expect("ReferenceParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("references request");
        let response = response.expect("references should return a response");

        let value = serde_json::to_value(&response).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<Vec<Location>> =
            serde_json::from_value(result_value).expect("parse result");
        let parsed = parsed.expect("result present");
        assert_eq!(parsed.len(), 3, "expected declaration + 2 call sites");

        drain_task.abort();
    }

    #[tokio::test]
    async fn p17_rename_routine_updates_declaration_and_calls_only() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let uri = Url::parse("file:///test_p17_routine_rename.bsl").expect("test uri");
        let text = concat!(
            "Процедура Foo() Экспорт\n",
            "КонецПроцедуры\n",
            "Процедура FooX() Экспорт\n",
            "КонецПроцедуры\n",
            "Процедура Bar()\n",
            "    Foo();\n",
            "    FooX();\n",
            "    Foo();\n",
            "КонецПроцедуры\n",
        );

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let rename_params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 5,
                    character: 4,
                },
            },
            new_name: "Baz".to_string(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let rename_req = Request::build("textDocument/rename")
            .id(2)
            .params(serde_json::to_value(rename_params).expect("RenameParams"))
            .finish();

        let rename_resp = service
            .ready()
            .await
            .unwrap()
            .call(rename_req)
            .await
            .expect("rename request");
        let rename_resp = rename_resp.expect("rename should return a response");

        let value = serde_json::to_value(&rename_resp).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let parsed: Option<WorkspaceEdit> =
            serde_json::from_value(result_value).expect("parse workspace edit");
        let parsed = parsed.expect("result present");
        let changes = parsed.changes.expect("changes present");
        let edits = changes.get(&uri).expect("edits for uri");

        assert!(
            edits.iter().all(|e| e.new_text == "Baz"),
            "all edits must rename to Baz"
        );
        assert_eq!(
            edits.len(),
            3,
            "expected declaration + 2 call sites for Foo"
        );
        assert!(
            edits.iter().all(|e| e.range.start.line != 6),
            "must not touch FooX() call"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p18_capabilities_gate_inlay_hints_and_code_actions() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            initialization_options: Some(serde_json::json!({
                "enableTypeHints": true,
                "enableCodeActions": false
            })),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();

        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request")
            .expect("initialize response");

        let response_value =
            serde_json::to_value(&response).expect("serialize initialize response");
        let caps = response_value
            .get("result")
            .and_then(|v| v.get("capabilities"))
            .expect("initialize result.capabilities");

        assert!(
            caps.get("inlayHintProvider").is_some(),
            "inlayHintProvider must be present when enableTypeHints=true"
        );
        let code_actions = caps.get("codeActionProvider");
        assert!(
            code_actions.is_none() || code_actions.is_some_and(|v| v.is_null()),
            "codeActionProvider must be absent/null when enableCodeActions=false"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p19_inlay_hints_returns_type_hints_when_enabled() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            initialization_options: Some(serde_json::json!({
                "enableTypeHints": true,
                "enableCodeActions": false
            })),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let settings = DidChangeConfigurationParams {
            settings: serde_json::json!({
                "bsl": {
                    "hover": {
                        "detailLevel": "full",
                        "maxMethods": 10,
                        "maxProperties": 5,
                        "showCertainty": true
                    },
                    "diagnostics": {
                        "detailLevel": "standard",
                        "showHints": true
                    },
                    "formatting": {
                        "enabled": false,
                        "indentSize": 4
                    },
                    "typeHints": {
                        "enabled": true,
                        "showVariableTypes": true,
                        "showReturnTypes": false,
                        "showUnionDetails": true,
                        "minCertainty": 0.7
                    },
                    "codeActions": {
                        "enabled": false
                    }
                }
            }),
        };
        let settings_req = Request::build("workspace/didChangeConfiguration")
            .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
            .finish();
        let settings_resp = service
            .ready()
            .await
            .unwrap()
            .call(settings_req)
            .await
            .expect("didChangeConfiguration notification");
        assert!(
            settings_resp.is_none(),
            "didChangeConfiguration is a notification"
        );

        let uri = Url::parse("file:///test_p19_inlay_hints.bsl").expect("test uri");
        let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let params = InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range::new(Position::new(0, 0), Position::new(10, 0)),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let req = Request::build("textDocument/inlayHint")
            .id(2)
            .params(serde_json::to_value(params).expect("InlayHintParams"))
            .finish();
        let resp = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("inlayHint request")
            .expect("inlayHint response");

        let value = serde_json::to_value(&resp).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let hints: Option<Vec<InlayHint>> =
            serde_json::from_value(result_value).expect("parse hints");
        let hints = hints.expect("hints present");

        assert!(!hints.is_empty(), "expected at least one hint");
        assert!(
            hints.iter().any(|hint| matches!(&hint.label, InlayHintLabel::String(text) if text.contains(": Число"))),
            "expected ': Число' hint"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p20_code_actions_return_quickfix_add_type_annotation() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            initialization_options: Some(serde_json::json!({
                "enableTypeHints": true,
                "enableCodeActions": true
            })),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let settings = DidChangeConfigurationParams {
            settings: serde_json::json!({
                "bsl": {
                    "hover": {
                        "detailLevel": "full",
                        "maxMethods": 10,
                        "maxProperties": 5,
                        "showCertainty": true
                    },
                    "diagnostics": {
                        "detailLevel": "standard",
                        "showHints": true
                    },
                    "formatting": {
                        "enabled": false,
                        "indentSize": 4
                    },
                    "typeHints": {
                        "enabled": true,
                        "showVariableTypes": true,
                        "showReturnTypes": false,
                        "showUnionDetails": true,
                        "minCertainty": 0.7
                    },
                    "codeActions": {
                        "enabled": true
                    }
                }
            }),
        };
        let settings_req = Request::build("workspace/didChangeConfiguration")
            .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
            .finish();
        let settings_resp = service
            .ready()
            .await
            .unwrap()
            .call(settings_req)
            .await
            .expect("didChangeConfiguration notification");
        assert!(
            settings_resp.is_none(),
            "didChangeConfiguration is a notification"
        );

        let uri = Url::parse("file:///test_p20_code_actions.bsl").expect("test uri");
        let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range::new(Position::new(2, 0), Position::new(2, 5)),
            context: CodeActionContext {
                diagnostics: Vec::new(),
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let req = Request::build("textDocument/codeAction")
            .id(2)
            .params(serde_json::to_value(params).expect("CodeActionParams"))
            .finish();
        let resp = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("codeAction request")
            .expect("codeAction response");

        let value = serde_json::to_value(&resp).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let actions: Option<Vec<CodeActionOrCommand>> =
            serde_json::from_value(result_value).expect("parse actions");
        let actions = actions.expect("actions present");

        assert!(
            actions.iter().any(|action| matches!(action, CodeActionOrCommand::CodeAction(action) if action.kind.as_ref() == Some(&tower_lsp::lsp_types::CodeActionKind::QUICKFIX))),
            "expected at least one quickfix action"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p21_code_actions_return_extract_refactor_on_selection() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            initialization_options: Some(serde_json::json!({
                "enableTypeHints": true,
                "enableCodeActions": true
            })),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(response.is_some(), "initialize should return a response");

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let settings = DidChangeConfigurationParams {
            settings: serde_json::json!({
                "bsl": {
                    "hover": {
                        "detailLevel": "full",
                        "maxMethods": 10,
                        "maxProperties": 5,
                        "showCertainty": true
                    },
                    "diagnostics": {
                        "detailLevel": "standard",
                        "showHints": true
                    },
                    "formatting": {
                        "enabled": false,
                        "indentSize": 4
                    },
                    "typeHints": {
                        "enabled": true,
                        "showVariableTypes": true,
                        "showReturnTypes": false,
                        "showUnionDetails": true,
                        "minCertainty": 0.7
                    },
                    "codeActions": {
                        "enabled": true
                    }
                }
            }),
        };
        let settings_req = Request::build("workspace/didChangeConfiguration")
            .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
            .finish();
        let settings_resp = service
            .ready()
            .await
            .unwrap()
            .call(settings_req)
            .await
            .expect("didChangeConfiguration notification");
        assert!(
            settings_resp.is_none(),
            "didChangeConfiguration is a notification"
        );

        let uri = Url::parse("file:///test_p21_code_actions.bsl").expect("test uri");
        let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range::new(Position::new(2, 4), Position::new(2, 5)),
            context: CodeActionContext {
                diagnostics: Vec::new(),
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let req = Request::build("textDocument/codeAction")
            .id(2)
            .params(serde_json::to_value(params).expect("CodeActionParams"))
            .finish();
        let resp = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("codeAction request")
            .expect("codeAction response");

        let value = serde_json::to_value(&resp).expect("serialize response");
        let result_value = value.get("result").cloned().expect("result field");
        let actions: Option<Vec<CodeActionOrCommand>> =
            serde_json::from_value(result_value).expect("parse actions");
        let actions = actions.expect("actions present");

        assert!(
            actions.iter().any(|action| matches!(action, CodeActionOrCommand::CodeAction(action) if action.kind.as_ref() == Some(&tower_lsp::lsp_types::CodeActionKind::REFACTOR_EXTRACT))),
            "expected refactor.extract action"
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p22_get_observability_metrics_exposes_unified_stage_contract() {
        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        let initialize_params = InitializeParams {
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        let initialize = Request::build("initialize")
            .id(1)
            .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
            .finish();
        let initialize_response = service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .expect("initialize request");
        assert!(
            initialize_response.is_some(),
            "initialize should return a response"
        );

        let initialized = Request::build("initialized")
            .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
            .finish();
        let initialized_response = service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );

        let execute = Request::build("workspace/executeCommand")
            .id(2)
            .params(serde_json::json!({
                "command": "bsl.getObservabilityMetrics",
                "arguments": [],
            }))
            .finish();
        let execute_response = service
            .ready()
            .await
            .unwrap()
            .call(execute)
            .await
            .expect("workspace/executeCommand request")
            .expect("workspace/executeCommand response");

        let value = serde_json::to_value(&execute_response).expect("serialize response");
        let result = value.get("result").cloned().expect("result field");
        assert_unified_intellisense_v2_stage_contract(&result);

        drain_task.abort();
    }

    #[tokio::test]
    async fn p23_cross_interface_semantic_parity_lsp_web_mcp_diagnostics() {
        const PARITY_FIXTURE: &str = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.НесуществующийМетод();\nКонецПроцедуры\n";

        let lsp_coordinator = Arc::new(SystemCoordinator::new());
        let (mut service, mut socket) = LspService::build({
            let coordinator = lsp_coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();
        let (published_tx, mut published_rx) =
            tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
        let drain_task = tokio::spawn(async move {
            while let Some(req) = socket.next().await {
                if req.method() != "textDocument/publishDiagnostics" {
                    continue;
                }
                let Some(params) = req.params().cloned() else {
                    continue;
                };
                let Ok(parsed) = serde_json::from_value::<
                    tower_lsp::lsp_types::PublishDiagnosticsParams,
                >(params) else {
                    continue;
                };
                let _ = published_tx.send(parsed);
            }
        });

        initialize_lsp_service(&mut service).await;
        let lsp_uri = Url::parse("file:///parity_fixture.bsl").expect("lsp uri");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: lsp_uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: PARITY_FIXTURE.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
        let lsp_diagnostics = wait_lsp_publish_diagnostics(&mut published_rx, &lsp_uri).await;
        let lsp_normalized = normalize_lsp_semantic_diagnostics(&lsp_diagnostics);
        assert!(
            !lsp_normalized.is_empty(),
            "expected non-empty LSP diagnostics"
        );
        drain_task.abort();

        let app = create_router(build_web_test_state(), "backend/static", true);
        let web_response = app
            .oneshot(
                AxumRequest::post("/api/diagnostics")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "code": PARITY_FIXTURE }).to_string(),
                    ))
                    .expect("web diagnostics request"),
            )
            .await
            .expect("web diagnostics response");
        assert!(
            web_response.status().is_success(),
            "unexpected web status: {}",
            web_response.status()
        );
        let web_body = axum::body::to_bytes(web_response.into_body(), usize::MAX)
            .await
            .expect("web body");
        let web_payload: serde_json::Value =
            serde_json::from_slice(&web_body).expect("web diagnostics payload");
        let web_normalized = normalize_web_semantic_diagnostics(&web_payload);
        assert!(
            !web_normalized.is_empty(),
            "expected non-empty Web diagnostics, payload={web_payload}"
        );

        let temp = tempfile::TempDir::new().expect("tempdir");
        let module_path = temp.path().join("Module.bsl");
        std::fs::write(&module_path, PARITY_FIXTURE).expect("write module");
        let mcp_manager = Arc::new(SessionManager::new());
        let mcp_job_manager = Arc::new(JobManager::new());
        let open = mcp_manager
            .open(
                WorkspaceOpenParams {
                    roots: vec![temp.path().to_string_lossy().to_string()],
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                },
                mcp_job_manager.clone(),
            )
            .await
            .expect("mcp workspace open");
        wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;
        let mcp_diagnostics = mcp_manager
            .bsl_diagnostics(BslDiagnosticsParams {
                session_id: open.session_id,
                scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Project),
                limit: 200,
                include_impact: false,
                include_coverage: false,
                include_flow_sensitive: false,
            })
            .await
            .expect("mcp diagnostics");
        let mcp_normalized = normalize_mcp_semantic_diagnostics(&mcp_diagnostics.diagnostics);
        assert!(
            !mcp_normalized.is_empty(),
            "expected non-empty MCP diagnostics"
        );

        assert_eq!(
            lsp_normalized, web_normalized,
            "LSP/Web semantic diagnostics drift detected"
        );
        assert_eq!(
            lsp_normalized, mcp_normalized,
            "LSP/MCP semantic diagnostics drift detected"
        );
    }

    #[tokio::test]
    async fn p24_real_scenario_observability_stage_parity_lsp_vs_mcp() {
        const OBSERVABILITY_FIXTURE: &str =
            "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";

        let lsp_coordinator = Arc::new(SystemCoordinator::new());
        let (mut service, mut socket) = LspService::build({
            let coordinator = lsp_coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();
        let (published_tx, mut published_rx) =
            tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
        let drain_task = tokio::spawn(async move {
            while let Some(req) = socket.next().await {
                if req.method() != "textDocument/publishDiagnostics" {
                    continue;
                }
                let Some(params) = req.params().cloned() else {
                    continue;
                };
                let Ok(parsed) = serde_json::from_value::<
                    tower_lsp::lsp_types::PublishDiagnosticsParams,
                >(params) else {
                    continue;
                };
                let _ = published_tx.send(parsed);
            }
        });

        initialize_lsp_service(&mut service).await;

        let uri = Url::parse("file:///observability_fixture.bsl").expect("lsp uri");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: OBSERVABILITY_FIXTURE.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
        let _ = wait_lsp_publish_diagnostics(&mut published_rx, &uri).await;

        let completion = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 13),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };
        let completion_req = Request::build("textDocument/completion")
            .id(2)
            .params(serde_json::to_value(completion).expect("CompletionParams"))
            .finish();
        let completion_response = service
            .ready()
            .await
            .unwrap()
            .call(completion_req)
            .await
            .expect("completion request");
        assert!(
            completion_response.is_some(),
            "completion should return a response"
        );

        let execute = Request::build("workspace/executeCommand")
            .id(3)
            .params(serde_json::json!({
                "command": "bsl.getObservabilityMetrics",
                "arguments": [],
            }))
            .finish();
        let execute_response = service
            .ready()
            .await
            .unwrap()
            .call(execute)
            .await
            .expect("workspace/executeCommand request")
            .expect("workspace/executeCommand response");
        let lsp_metrics_payload =
            serde_json::to_value(&execute_response).expect("serialize execute response");
        let lsp_metrics_payload = lsp_metrics_payload
            .get("result")
            .cloned()
            .expect("execute result field");
        drain_task.abort();

        let temp = tempfile::TempDir::new().expect("tempdir");
        let module_path = temp.path().join("Module.bsl");
        std::fs::write(&module_path, OBSERVABILITY_FIXTURE).expect("write module");
        let mcp_manager = Arc::new(SessionManager::new());
        let mcp_job_manager = Arc::new(JobManager::new());
        let open = mcp_manager
            .open(
                WorkspaceOpenParams {
                    roots: vec![temp.path().to_string_lossy().to_string()],
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                },
                mcp_job_manager.clone(),
            )
            .await
            .expect("mcp workspace open");
        wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;

        let _diagnostics = mcp_manager
            .bsl_diagnostics(BslDiagnosticsParams {
                session_id: open.session_id.clone(),
                scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Project),
                limit: 200,
                include_impact: false,
                include_coverage: false,
                include_flow_sensitive: false,
            })
            .await
            .expect("mcp diagnostics");
        let _members = mcp_manager
            .bsl_members(BslMembersParams {
                session_id: open.session_id.clone(),
                file: McpFileRef {
                    doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                    text: None,
                    version: None,
                },
                position: McpPosition {
                    line: 2,
                    character: 13,
                },
                limit: 50,
                include_flow_sensitive: false,
            })
            .await
            .expect("mcp members");
        let mcp_metrics_payload = mcp_manager
            .observability_metrics_get(&open.session_id)
            .await
            .expect("mcp observability")
            .metrics;

        let lsp_stages = collect_observed_stages(&lsp_metrics_payload);
        let mcp_stages = collect_observed_stages(&mcp_metrics_payload);
        let required_stages = [
            "runtime_snapshot_with_deps",
            "semantic_diagnostics_query",
            "ir_query",
            "parse_result_query",
        ];

        for stage in required_stages {
            assert!(
                lsp_stages.contains(stage),
                "LSP metrics missing required stage {stage}, stages={lsp_stages:?}"
            );
            assert!(
                mcp_stages.contains(stage),
                "MCP metrics missing required stage {stage}, stages={mcp_stages:?}"
            );
            assert!(
                has_positive_counter_for_stage(&lsp_metrics_payload, stage),
                "LSP stage {stage} has no positive counters"
            );
            assert!(
                has_positive_counter_for_stage(&mcp_metrics_payload, stage),
                "MCP stage {stage} has no positive counters"
            );
        }

        assert_drilldown_stage_metrics_for_origin(&lsp_metrics_payload, "lsp");
        assert_drilldown_stage_metrics_for_origin(&mcp_metrics_payload, "agent");
    }

    #[tokio::test]
    async fn p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools() {
        const TOOLS_FIXTURE: &str = "Процедура Foo() Экспорт\nКонецПроцедуры\n\nПроцедура Bar()\n    Arr = Новый Массив;\n    Arr.Добавить(1);\n    Foo();\nКонецПроцедуры\n";
        const TARGET_SYMBOL: &str = "Foo";
        const TYPE_LINE: u32 = 5;
        const TYPE_CHARACTER: u32 = 5;
        const MEMBERS_LINE: u32 = 5;
        const MEMBERS_CHARACTER: u32 = 7;
        const SYMBOL_CALL_LINE: u32 = 6;
        const SYMBOL_CALL_CHARACTER: u32 = 5;

        let lsp_coordinator = Arc::new(SystemCoordinator::new());
        let (mut service, mut socket) = LspService::build({
            let coordinator = lsp_coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let lsp_uri = Url::parse("file:///Module.bsl").expect("lsp uri");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: lsp_uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: TOOLS_FIXTURE.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: lsp_uri.clone(),
                },
                position: Position {
                    line: MEMBERS_LINE,
                    character: MEMBERS_CHARACTER,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };
        let completion_req = Request::build("textDocument/completion")
            .id(2)
            .params(serde_json::to_value(completion_params).expect("CompletionParams"))
            .finish();
        let completion_response = service
            .ready()
            .await
            .unwrap()
            .call(completion_req)
            .await
            .expect("completion request")
            .expect("completion response");
        let completion_value =
            serde_json::to_value(&completion_response).expect("serialize response");
        let completion_result = completion_value
            .get("result")
            .cloned()
            .expect("result field");
        let lsp_completion: Option<CompletionResponse> =
            serde_json::from_value(completion_result).expect("parse completion result");
        let lsp_completion = lsp_completion.expect("completion result present");
        let lsp_members = normalize_lsp_member_labels(&lsp_completion);

        let symbol_req = Request::build("workspace/symbol")
            .id(3)
            .params(
                serde_json::to_value(WorkspaceSymbolParams {
                    query: TARGET_SYMBOL.to_string(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .expect("WorkspaceSymbolParams"),
            )
            .finish();
        let symbol_response = service
            .ready()
            .await
            .unwrap()
            .call(symbol_req)
            .await
            .expect("workspace/symbol request")
            .expect("workspace/symbol response");
        let symbol_value = serde_json::to_value(&symbol_response).expect("serialize response");
        let symbol_result = symbol_value.get("result").cloned().expect("result field");
        let lsp_symbols: Option<Vec<SymbolInformation>> =
            serde_json::from_value(symbol_result).expect("parse symbol result");
        let lsp_symbols = lsp_symbols.expect("symbol result present");
        let lsp_symbols = normalize_lsp_workspace_symbols(&lsp_symbols);
        assert!(
            !lsp_symbols.is_empty(),
            "expected non-empty LSP symbol_search result"
        );

        let definition_req = Request::build("textDocument/definition")
            .id(4)
            .params(
                serde_json::to_value(GotoDefinitionParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier {
                            uri: lsp_uri.clone(),
                        },
                        position: Position {
                            line: SYMBOL_CALL_LINE,
                            character: SYMBOL_CALL_CHARACTER,
                        },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .expect("GotoDefinitionParams"),
            )
            .finish();
        let definition_response = service
            .ready()
            .await
            .unwrap()
            .call(definition_req)
            .await
            .expect("textDocument/definition request")
            .expect("textDocument/definition response");
        let definition_value =
            serde_json::to_value(&definition_response).expect("serialize response");
        let definition_result = definition_value
            .get("result")
            .cloned()
            .expect("result field");
        let lsp_definition: Option<GotoDefinitionResponse> =
            serde_json::from_value(definition_result).expect("parse definition result");
        let lsp_definition = normalize_lsp_definition(lsp_definition);
        assert!(
            !lsp_definition.is_empty(),
            "expected non-empty LSP definition result"
        );

        let references_req = Request::build("textDocument/references")
            .id(5)
            .params(
                serde_json::to_value(ReferenceParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier {
                            uri: lsp_uri.clone(),
                        },
                        position: Position {
                            line: SYMBOL_CALL_LINE,
                            character: SYMBOL_CALL_CHARACTER,
                        },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: ReferenceContext {
                        include_declaration: false,
                    },
                })
                .expect("ReferenceParams"),
            )
            .finish();
        let references_response = service
            .ready()
            .await
            .unwrap()
            .call(references_req)
            .await
            .expect("textDocument/references request")
            .expect("textDocument/references response");
        let references_value =
            serde_json::to_value(&references_response).expect("serialize response");
        let references_result = references_value
            .get("result")
            .cloned()
            .expect("result field");
        let lsp_references: Option<Vec<Location>> =
            serde_json::from_value(references_result).expect("parse references result");
        let lsp_references = normalize_lsp_locations(&lsp_references.unwrap_or_default());
        assert!(
            !lsp_references.is_empty(),
            "expected non-empty LSP references result"
        );

        let hover_req = Request::build("textDocument/hover")
            .id(6)
            .params(
                serde_json::to_value(HoverParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier {
                            uri: lsp_uri.clone(),
                        },
                        position: Position {
                            line: TYPE_LINE,
                            character: TYPE_CHARACTER,
                        },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .expect("HoverParams"),
            )
            .finish();
        let hover_response = service
            .ready()
            .await
            .unwrap()
            .call(hover_req)
            .await
            .expect("textDocument/hover request")
            .expect("textDocument/hover response");
        let hover_value = serde_json::to_value(&hover_response).expect("serialize response");
        let hover_result = hover_value.get("result").cloned().expect("result field");
        let lsp_hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover");
        let lsp_hover_text = lsp_hover
            .and_then(extract_hover_text)
            .unwrap_or_else(|| String::from(""));
        assert!(
            !lsp_hover_text.is_empty(),
            "expected non-empty LSP hover response at type position"
        );
        drain_task.abort();

        // Web currently exposes hover/diagnostics for semantic parity, while MCP-only tools below
        // are validated via LSP/MCP pairs.
        let app = create_router(build_web_test_state(), "backend/static", true);
        let web_hover_response = app
            .oneshot(
                AxumRequest::post("/api/hover")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "code": TOOLS_FIXTURE,
                            "line": TYPE_LINE,
                            "column": TYPE_CHARACTER
                        })
                        .to_string(),
                    ))
                    .expect("web hover request"),
            )
            .await
            .expect("web hover response");
        assert!(
            web_hover_response.status().is_success(),
            "unexpected web hover status: {}",
            web_hover_response.status()
        );
        let web_hover_body = axum::body::to_bytes(web_hover_response.into_body(), usize::MAX)
            .await
            .expect("web hover body");
        let web_hover_payload: serde_json::Value =
            serde_json::from_slice(&web_hover_body).expect("web hover payload");
        let web_hover_text = web_hover_payload
            .get("hover")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        assert!(
            !web_hover_text.is_empty(),
            "expected non-empty Web hover text, payload={web_hover_payload}"
        );

        let temp = tempfile::TempDir::new().expect("tempdir");
        let module_path = temp.path().join("Module.bsl");
        std::fs::write(&module_path, TOOLS_FIXTURE).expect("write module");
        let mcp_manager = Arc::new(SessionManager::new());
        let mcp_job_manager = Arc::new(JobManager::new());
        let open = mcp_manager
            .open(
                WorkspaceOpenParams {
                    roots: vec![temp.path().to_string_lossy().to_string()],
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                },
                mcp_job_manager.clone(),
            )
            .await
            .expect("mcp workspace open");
        wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;

        let mcp_type = mcp_manager
            .bsl_type_at_position(BslTypeAtPositionParams {
                session_id: open.session_id.clone(),
                file: McpFileRef {
                    doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                    text: None,
                    version: None,
                },
                position: McpPosition {
                    line: TYPE_LINE,
                    character: TYPE_CHARACTER,
                },
                include_flow_sensitive: false,
            })
            .await
            .expect("mcp type_at_position");
        assert!(
            mcp_type.warnings.is_empty(),
            "mcp type_at_position returned warnings: {:?}",
            mcp_type.warnings
        );
        let mcp_type_name = mcp_type
            .type_info
            .as_ref()
            .map(|type_info| type_info.name.clone())
            .expect("mcp type_at_position type_info");

        let mcp_members = mcp_manager
            .bsl_members(BslMembersParams {
                session_id: open.session_id.clone(),
                file: McpFileRef {
                    doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                    text: None,
                    version: None,
                },
                position: McpPosition {
                    line: MEMBERS_LINE,
                    character: MEMBERS_CHARACTER,
                },
                limit: 100,
                include_flow_sensitive: false,
            })
            .await
            .expect("mcp members");
        let mcp_members = normalize_mcp_member_labels(&mcp_members.members);

        let mcp_symbol_search = mcp_manager
            .bsl_symbol_search(BslSymbolSearchParams {
                session_id: open.session_id.clone(),
                query: TARGET_SYMBOL.to_string(),
                limit: 20,
            })
            .await
            .expect("mcp symbol_search");
        let mcp_symbols = normalize_mcp_workspace_symbols(&mcp_symbol_search.symbols);
        assert!(
            !mcp_symbols.is_empty(),
            "expected non-empty MCP symbol_search result"
        );
        let mcp_target_symbol_id = mcp_symbol_search
            .symbols
            .iter()
            .find(|symbol| symbol.name == TARGET_SYMBOL)
            .map(|symbol| symbol.symbol_id.clone())
            .expect("mcp target symbol id");

        let mcp_references = mcp_manager
            .bsl_references(BslReferencesParams {
                session_id: open.session_id.clone(),
                symbol_id: mcp_target_symbol_id,
                limit: 50,
                include_snippets: false,
            })
            .await
            .expect("mcp references");
        let mcp_references = normalize_mcp_references(&mcp_references.references);
        assert!(
            !mcp_references.is_empty(),
            "expected non-empty MCP references result"
        );

        let mcp_definition = mcp_manager
            .bsl_definition(BslDefinitionParams {
                session_id: open.session_id,
                symbol_id: None,
                file: Some(McpFileRef {
                    doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                    text: None,
                    version: None,
                }),
                position: Some(McpPosition {
                    line: SYMBOL_CALL_LINE,
                    character: SYMBOL_CALL_CHARACTER,
                }),
            })
            .await
            .expect("mcp definition");
        let mcp_definition = normalize_mcp_definition(mcp_definition.location.as_ref());
        assert!(
            !mcp_definition.is_empty(),
            "expected non-empty MCP definition result"
        );

        assert_eq!(lsp_members, mcp_members, "LSP/MCP members drift detected");
        assert_eq!(
            lsp_symbols, mcp_symbols,
            "LSP/MCP symbol_search drift detected"
        );
        assert_eq!(
            lsp_references, mcp_references,
            "LSP/MCP references drift detected"
        );
        assert_eq!(
            lsp_definition, mcp_definition,
            "LSP/MCP definition drift detected"
        );

        assert!(
            lsp_hover_text.contains(&mcp_type_name),
            "LSP hover/type_at_position drift detected: expected '{mcp_type_name}' in hover text, got '{lsp_hover_text}'"
        );
        assert!(
            web_hover_text.contains(&mcp_type_name),
            "Web hover/type_at_position drift detected: expected '{mcp_type_name}' in hover text, got '{web_hover_text}'"
        );
    }

    #[tokio::test]
    async fn p26_interactive_warm_path_completion_slo_smoke_conf_big() {
        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

        fn conf_big_root() -> Option<std::path::PathBuf> {
            let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("workspace root")
                .to_path_buf();
            let candidates = [
                workspace_root.join("examples").join("conf_big"),
                std::path::PathBuf::from("examples/conf_big"),
                std::path::PathBuf::from("../examples/conf_big"),
            ];
            candidates
                .into_iter()
                .find(|path| path.join("Configuration.xml").exists())
        }

        let Some(root) = conf_big_root() else {
            if allow_fixture_skip {
                eprintln!(
                    "skipping p26 warm-path SLO smoke: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
                );
                return;
            }
            panic!(
                "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip this test explicitly"
            );
        };

        let module_rel = std::path::PathBuf::from("Documents")
            .join("РеализацияТоваровУслуг")
            .join("Forms")
            .join("ФормаДокументаОбщая")
            .join("Ext")
            .join("Form")
            .join("Module.bsl");
        let module_path = root.join(&module_rel);
        if !module_path.exists() {
            if allow_fixture_skip {
                eprintln!(
                    "skipping p26 warm-path SLO smoke: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                    module_path.display()
                );
                return;
            }
            panic!(
                "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip this test explicitly",
                module_path.display()
            );
        }

        let module_text = std::fs::read_to_string(&module_path).expect("read conf_big module");
        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));
        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let uri = Url::parse("file:///conf_big_perf_module.bsl").expect("uri");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: module_text,
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .clone()
            .expect("server must be created");
        for _ in 0..50_u64 {
            let completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(0, 0),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: None,
                })
                .await
                .expect("completion request");
            assert!(completion.is_some(), "completion response expected");
        }

        // Dedicated concurrent parse burst to exercise parse_result singleflight sharing
        // without polluting completion duration SLO samples.
        let file_id = server
            .get_file_id_v2(&uri)
            .await
            .expect("file_id must be available after didOpen");
        let parse_context = Arc::new(
            server
                .build_execution_context_v2(
                    bsl_runtime::application::SemanticOperation::Diagnostics,
                    file_id,
                    None,
                    false,
                )
                .await,
        );
        let parse_barrier = Arc::new(std::sync::Barrier::new(9));
        std::thread::scope(|scope| {
            let mut workers = Vec::new();
            for _ in 0..8_u32 {
                let runtime = server.analysis_v2.clone();
                let parse_context = parse_context.clone();
                let parse_barrier = parse_barrier.clone();
                let coordinator = coordinator.clone();
                workers.push(scope.spawn(move || {
                    parse_barrier.wait();
                    let analysis = futures::executor::block_on(runtime.snapshot());
                    bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                        parse_context.as_ref(),
                        &analysis,
                        true,
                        Some(coordinator.as_ref()),
                        file_id,
                    )
                }));
            }
            parse_barrier.wait();
            for worker in workers {
                let result = worker.join().expect("parse burst worker should not panic");
                assert!(
                    result.is_ok(),
                    "parse burst worker should complete without hard cancellation"
                );
            }
        });

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let rates = metrics
            .get("rates")
            .and_then(|value| value.as_object())
            .expect("metrics.rates object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");

        let wait_hist = histograms
            .get("intellisense_v2_wait_for_file_version_completion_ms")
            .or_else(|| histograms.get("intellisense_v2_wait_for_file_version_other_ms"))
            .and_then(|value| value.as_object());
        let completion_hist = histograms
            .get("completion_duration_ms")
            .and_then(|value| value.as_object())
            .expect("completion duration histogram");

        let completion_count = completion_hist
            .get("count")
            .and_then(|value| value.as_u64())
            .expect("completion count");
        assert!(
            completion_count >= 50,
            "expected at least 50 completion duration samples, got {completion_count}"
        );

        let wait_p95 = wait_hist
            .and_then(|hist| hist.get("p95"))
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0);
        let queue_wait_interactive_p95 = histograms
            .get("intellisense_v2_runtime_queue_wait_interactive_ms")
            .and_then(|value| value.as_object())
            .and_then(|hist| hist.get("p95"))
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0);
        let completion_p95 = completion_hist
            .get("p95")
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .expect("completion p95");
        let parse_result_shared_rate = rates
            .get("intellisense_v2_parse_result_singleflight_shared_rate")
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0);
        let parse_result_cancel_rate = rates
            .get("intellisense_v2_parse_result_query_cancel_rate")
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0);
        let wait_budget_ms = bsl_runtime::system::global_runtime_config()
            .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
            .unwrap_or(120)
            .clamp(10, 2000) as f64;

        assert!(
            wait_p95 <= wait_budget_ms + 20.0,
            "warm-path wait p95 regression: wait_p95={}ms budget={}ms",
            wait_p95,
            wait_budget_ms
        );
        assert!(
            completion_p95 <= 1500.0,
            "warm-path completion p95 regression: completion_p95={}ms > 1500ms",
            completion_p95
        );
        assert!(
            queue_wait_interactive_p95 <= wait_budget_ms + 250.0,
            "warm-path interactive queue-wait p95 regression: queue_wait_interactive_p95={}ms budget={}ms",
            queue_wait_interactive_p95,
            wait_budget_ms
        );
        assert!(
            parse_result_shared_rate >= 0.01,
            "parse_result singleflight shared-rate regression: shared_rate={:.3}",
            parse_result_shared_rate
        );
        assert!(
            parse_result_cancel_rate <= 0.30,
            "parse_result cancel-rate regression: cancel_rate={:.3}",
            parse_result_cancel_rate
        );
        let completion_total = counters
            .get("completion_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            completion_total >= 50,
            "expected completion_total >= 50, got {completion_total}"
        );
        let completion_cancelled_total = counters
            .get("intellisense_v2_completion_result_total_cancelled")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let completion_cancelled_rate =
            completion_cancelled_total as f64 / completion_total.max(1) as f64;
        assert!(
            completion_cancelled_rate <= 0.10,
            "warm-path completion cancel-rate regression: cancelled={} total={} rate={:.3}",
            completion_cancelled_total,
            completion_total,
            completion_cancelled_rate
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn p27_interactive_completion_acceptance_gates_emit_artifact() {
        const CHANGE_ID: &str = "refactor-v2-completion-event-driven-pipeline";
        const ITERATIONS: u64 = 120;
        const MAX_P95_MS: f64 = 300.0;
        const MAX_P99_MS: f64 = 800.0;
        const MIN_FIRST_TRIGGER_SUCCESS_RATE: f64 = 0.99;
        const MAX_TERMINAL_EMPTY_RATE: f64 = 0.005;
        const MAX_PARITY_MISMATCH_RATE: f64 = 0.01;

        fn completion_items_count(response: &CompletionResponse) -> usize {
            match response {
                CompletionResponse::Array(items) => items.len(),
                CompletionResponse::List(list) => list.items.len(),
            }
        }

        fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
            value
                .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
                .unwrap_or(0.0)
        }

        fn sum_counters_by_prefix(
            counters: &serde_json::Map<String, serde_json::Value>,
            prefix: &str,
        ) -> u64 {
            counters
                .iter()
                .filter(|(key, _)| key.starts_with(prefix))
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        }

        fn sum_counters_by_prefix_and_substring(
            counters: &serde_json::Map<String, serde_json::Value>,
            prefix: &str,
            needle: &str,
        ) -> u64 {
            counters
                .iter()
                .filter(|(key, _)| key.starts_with(prefix) && key.contains(needle))
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        }

        fn stage_mode_counter_total(
            counters: &serde_json::Map<String, serde_json::Value>,
            stage: &str,
            mode: &str,
        ) -> u64 {
            let stage_token = format!("_stage_{stage}");
            counters
                .iter()
                .filter(|(key, _)| {
                    key.starts_with("intellisense_v2_drilldown_stage_total_")
                        && key.contains("_origin_lsp_")
                        && key.contains("_operation_completion_")
                        && (key.contains(&format!("{stage_token}_")) || key.ends_with(&stage_token))
                        && key.contains(&format!("_mode_{mode}"))
                })
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        }

        fn stage_mode_latency_p95(
            histograms: &serde_json::Map<String, serde_json::Value>,
            stage: &str,
            mode: &str,
        ) -> f64 {
            let stage_token = format!("_stage_{stage}");
            histograms
                .iter()
                .filter(|(key, _)| {
                    key.starts_with("intellisense_v2_drilldown_stage_latency_ms_")
                        && key.contains("_origin_lsp_")
                        && key.contains("_operation_completion_")
                        && (key.contains(&format!("{stage_token}_")) || key.ends_with(&stage_token))
                        && key.contains(&format!("_mode_{mode}"))
                })
                .filter_map(|(_, value)| value.as_object())
                .map(|hist| metric_as_f64(hist.get("p95")))
                .fold(0.0, f64::max)
        }

        fn collect_mode_split_stage_metrics(
            counters: &serde_json::Map<String, serde_json::Value>,
            histograms: &serde_json::Map<String, serde_json::Value>,
        ) -> serde_json::Value {
            const STAGES: &[&str] = &[
                "runtime_wait_for_file_version",
                "runtime_snapshot_with_deps",
                "ir_query",
                "parse_result_query",
            ];
            const MODES: &[&str] = &["legacy", "event_driven", "shadow"];

            let mut by_mode = serde_json::Map::new();
            for mode in MODES {
                let mut by_stage = serde_json::Map::new();
                for stage in STAGES {
                    by_stage.insert(
                        (*stage).to_string(),
                        serde_json::json!({
                            "total": stage_mode_counter_total(counters, stage, mode),
                            "p95_ms": stage_mode_latency_p95(histograms, stage, mode),
                        }),
                    );
                }
                by_mode.insert((*mode).to_string(), serde_json::Value::Object(by_stage));
            }
            serde_json::Value::Object(by_mode)
        }

        let coordinator = Arc::new(SystemCoordinator::new());
        let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
            Arc::new(std::sync::Mutex::new(None));
        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let server_holder = server_holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *server_holder.lock().expect("server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();
        let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let uri = Url::parse("file:///test_p27_interactive_acceptance_gate.bsl").expect("test uri");
        let text = concat!(
            "Процедура Тест()\n",
            "    ЛокМассив = Новый Массив;\n",
            "    ЛокМассив.\n",
            "КонецПроцедуры\n"
        );
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .clone()
            .expect("server must be created");
        let member_character = "    ЛокМассив."
            .chars()
            .map(|ch| ch.len_utf16())
            .sum::<usize>() as u32;

        let mut first_trigger_success_total = 0_u64;
        let mut first_trigger_total = 0_u64;
        let mut parity_pairs_total = 0_u64;

        for iteration in 0..ITERATIONS {
            let version = (iteration + 2) as i32;
            let did_change = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.to_string(),
                }],
            };
            let did_change_req = Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish();
            let did_change_response = service
                .ready()
                .await
                .unwrap()
                .call(did_change_req)
                .await
                .expect("didChange notification");
            assert!(did_change_response.is_none(), "didChange is a notification");

            let dot_completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(2, member_character),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: Some(CompletionContext {
                        trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                        trigger_character: Some(".".to_string()),
                    }),
                })
                .await
                .expect("dot completion request")
                .expect("dot completion response");
            first_trigger_total += 1;
            if completion_items_count(&dot_completion) > 0 {
                first_trigger_success_total += 1;
            }

            let invoked_completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(2, member_character),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: Some(CompletionContext {
                        trigger_kind: CompletionTriggerKind::INVOKED,
                        trigger_character: None,
                    }),
                })
                .await
                .expect("invoked completion request")
                .expect("invoked completion response");
            assert!(
                completion_items_count(&invoked_completion) > 0,
                "invoked completion must return non-empty candidates in acceptance gate loop"
            );
            parity_pairs_total += 1;
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        let completion_hist = histograms
            .get("completion_duration_ms")
            .and_then(|value| value.as_object())
            .expect("completion duration histogram");
        let completion_p95 = metric_as_f64(completion_hist.get("p95"));
        let completion_p99 = metric_as_f64(completion_hist.get("p99"));
        let mode_split_stage_metrics = collect_mode_split_stage_metrics(counters, histograms);

        let first_trigger_success_rate =
            first_trigger_success_total as f64 / first_trigger_total.max(1) as f64;
        let terminal_empty_missing_ir_total = sum_counters_by_prefix_and_substring(
            counters,
            "intellisense_v2_completion_member_access_terminal_empty_total_",
            "_reason_missing_ir",
        );
        let terminal_empty_rate =
            terminal_empty_missing_ir_total as f64 / first_trigger_total.max(1) as f64;
        let parity_drift_total = sum_counters_by_prefix(
            counters,
            "intellisense_v2_completion_parity_drift_total_mode_",
        );
        let parity_mismatch_rate = parity_drift_total as f64 / parity_pairs_total.max(1) as f64;
        let completion_mode = bsl_runtime::system::global_runtime_config()
            .get_string(bsl_runtime::system::RuntimeKey::IntellisenseV2CompletionMode)
            .unwrap_or_else(|| "on".to_string())
            .to_ascii_lowercase();
        let canary_percent = bsl_runtime::system::global_runtime_config()
            .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2CompletionCanaryPercent)
            .unwrap_or(0)
            .clamp(0, 100) as u8;
        let report_mode_suffix = if completion_mode == "canary" {
            format!("canary-{canary_percent}")
        } else {
            completion_mode.clone()
        };

        let pass = completion_p95 <= MAX_P95_MS
            && completion_p99 <= MAX_P99_MS
            && first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE
            && terminal_empty_rate <= MAX_TERMINAL_EMPTY_RATE
            && parity_mismatch_rate <= MAX_PARITY_MISMATCH_RATE;

        let report = serde_json::json!({
            "change_id": CHANGE_ID,
            "profile": "p27_interactive_completion_acceptance_gates",
            "mode": completion_mode,
            "canary_percent": canary_percent,
            "iterations": ITERATIONS,
            "thresholds": {
                "completion_p95_ms_max": MAX_P95_MS,
                "completion_p99_ms_max": MAX_P99_MS,
                "first_trigger_success_rate_min": MIN_FIRST_TRIGGER_SUCCESS_RATE,
                "terminal_empty_missing_ir_rate_max": MAX_TERMINAL_EMPTY_RATE,
                "parity_mismatch_rate_max": MAX_PARITY_MISMATCH_RATE
            },
            "results": {
                "completion_p95_ms": completion_p95,
                "completion_p99_ms": completion_p99,
                "first_trigger_success_total": first_trigger_success_total,
                "first_trigger_total": first_trigger_total,
                "first_trigger_success_rate": first_trigger_success_rate,
                "terminal_empty_missing_ir_total": terminal_empty_missing_ir_total,
                "terminal_empty_missing_ir_rate": terminal_empty_rate,
                "parity_drift_total": parity_drift_total,
                "parity_pairs_total": parity_pairs_total,
                "parity_mismatch_rate": parity_mismatch_rate,
                "mode_split_stage_metrics": mode_split_stage_metrics
            },
            "pass": pass
        });

        let report_path = std::env::var("BSL_V2_COMPLETION_GATE_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!("{CHANGE_ID}-gate-{report_mode_suffix}.json"))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for v2 completion gate report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("failed to serialize v2 completion gate report"),
        )
        .expect("failed to write v2 completion gate report");
        println!("v2_completion_gate_report={}", report_path.display());

        assert!(
            completion_p95 <= MAX_P95_MS,
            "acceptance gate failed: completion p95={}ms > {}ms",
            completion_p95,
            MAX_P95_MS
        );
        assert!(
            completion_p99 <= MAX_P99_MS,
            "acceptance gate failed: completion p99={}ms > {}ms",
            completion_p99,
            MAX_P99_MS
        );
        assert!(
            first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE,
            "acceptance gate failed: first-trigger success rate={:.4} < {:.4}",
            first_trigger_success_rate,
            MIN_FIRST_TRIGGER_SUCCESS_RATE
        );
        assert!(
            terminal_empty_rate <= MAX_TERMINAL_EMPTY_RATE,
            "acceptance gate failed: terminal-empty(missing_ir) rate={:.4} > {:.4}",
            terminal_empty_rate,
            MAX_TERMINAL_EMPTY_RATE
        );
        assert!(
            parity_mismatch_rate <= MAX_PARITY_MISMATCH_RATE,
            "acceptance gate failed: parity mismatch rate={:.4} > {:.4}",
            parity_mismatch_rate,
            MAX_PARITY_MISMATCH_RATE
        );

        drain_task.abort();
    }

    #[derive(Clone, Copy)]
    struct ScaleAwarePhase {
        name: &'static str,
        warmup: u64,
        iterations: u64,
    }

    fn read_numeric_metric(value: Option<&serde_json::Value>) -> f64 {
        value
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))
            .unwrap_or(0.0)
    }

    fn read_u64_metric(value: Option<&serde_json::Value>) -> u64 {
        value.and_then(|v| v.as_u64()).unwrap_or(0)
    }

    fn histogram_metric_value(
        histograms: &serde_json::Map<String, serde_json::Value>,
        primary_key: &str,
        fallback_key: Option<&str>,
    ) -> serde_json::Value {
        let histogram = histograms
            .get(primary_key)
            .or_else(|| fallback_key.and_then(|key| histograms.get(key)))
            .and_then(|value| value.as_object())
            .unwrap_or_else(|| panic!("missing histogram key {primary_key}"));

        serde_json::json!({
            "count": read_u64_metric(histogram.get("count")),
            "p50": read_numeric_metric(histogram.get("p50")),
            "p95": read_numeric_metric(histogram.get("p95")),
            "p99": read_numeric_metric(histogram.get("p99")),
        })
    }

    fn find_utf16_position_after_marker(source: &str, marker: &str) -> Position {
        let byte_index = source
            .find(marker)
            .unwrap_or_else(|| panic!("marker not found: {marker}"));
        let prefix = &source[..byte_index + marker.len()];
        let line = prefix.lines().count().saturating_sub(1) as u32;
        let last_line = prefix.lines().last().unwrap_or("");
        let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        Position::new(line, character)
    }

    async fn run_scale_aware_profile(
        profile_name: &str,
        uri: Url,
        text: String,
        position: Position,
        phases: &[ScaleAwarePhase],
    ) -> serde_json::Value {
        let mut profile_report = serde_json::Map::new();

        for phase in phases {
            let coordinator = Arc::new(SystemCoordinator::new());
            let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
                Arc::new(std::sync::Mutex::new(None));
            let (mut service, mut socket) = LspService::build({
                let coordinator = coordinator.clone();
                let server_holder = server_holder.clone();
                move |client| {
                    let server = BslLanguageServer::new(client, coordinator.clone());
                    *server_holder.lock().expect("server holder lock") = Some(server.clone());
                    server
                }
            })
            .finish();
            let drain_task =
                tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

            initialize_lsp_service(&mut service).await;

            let did_open = DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "bsl".to_string(),
                    version: 1,
                    text: text.clone(),
                },
            };
            let did_open_req = Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish();
            let did_open_response = service
                .ready()
                .await
                .unwrap()
                .call(did_open_req)
                .await
                .expect("didOpen notification");
            assert!(did_open_response.is_none(), "didOpen is a notification");

            let server = server_holder
                .lock()
                .expect("server holder lock")
                .clone()
                .expect("server must be created");

            let total_requests = phase.warmup + phase.iterations;
            for _ in 0..total_requests {
                let completion = server
                    .completion(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context: Some(CompletionContext {
                            trigger_kind: CompletionTriggerKind::INVOKED,
                            trigger_character: None,
                        }),
                    })
                    .await
                    .expect("completion request");
                assert!(
                    completion.is_some(),
                    "completion response expected for profile={profile_name}, phase={}",
                    phase.name
                );
            }

            let metrics = coordinator.observability_metrics();
            let counters = metrics
                .get("counters")
                .and_then(|value| value.as_object())
                .expect("metrics.counters object");
            let histograms = metrics
                .get("histograms")
                .and_then(|value| value.as_object())
                .expect("metrics.histograms object");

            let completion_total = read_u64_metric(counters.get("completion_total"));
            let completion_cancelled_total =
                read_u64_metric(counters.get("intellisense_v2_completion_result_total_cancelled"));
            let completion_cancelled_rate =
                completion_cancelled_total as f64 / completion_total.max(1) as f64;

            let phase_report = serde_json::json!({
                "warmup": phase.warmup,
                "iterations": phase.iterations,
                "completion_total": completion_total,
                "completion_cancelled_total": completion_cancelled_total,
                "completion_cancelled_rate": completion_cancelled_rate,
                "metrics": {
                    "completion_duration_ms": histogram_metric_value(histograms, "completion_duration_ms", None),
                    "intellisense_v2_wait_for_file_version_completion_ms": histogram_metric_value(
                        histograms,
                        "intellisense_v2_wait_for_file_version_completion_ms",
                        Some("intellisense_v2_wait_for_file_version_other_ms")
                    ),
                    "intellisense_v2_snapshot_completion_ms": histogram_metric_value(
                        histograms,
                        "intellisense_v2_snapshot_completion_ms",
                        Some("intellisense_v2_snapshot_other_ms")
                    ),
                    "intellisense_v2_ir_query_completion_ms": histogram_metric_value(
                        histograms,
                        "intellisense_v2_ir_query_completion_ms",
                        Some("intellisense_v2_ir_query_other_ms")
                    ),
                }
            });
            profile_report.insert(phase.name.to_string(), phase_report);

            drain_task.abort();
        }

        serde_json::Value::Object(profile_report)
    }

    fn get_report_metric_f64(report: &serde_json::Value, path: &[&str]) -> Result<f64, String> {
        let mut cursor = report;
        for segment in path {
            cursor = cursor
                .get(*segment)
                .ok_or_else(|| format!("missing field '{}'", path.join(".")))?;
        }
        cursor
            .as_f64()
            .or_else(|| cursor.as_u64().map(|n| n as f64))
            .ok_or_else(|| format!("field '{}' must be numeric", path.join(".")))
    }

    fn get_report_u64(report: &serde_json::Value, path: &[&str]) -> Result<u64, String> {
        let mut cursor = report;
        for segment in path {
            cursor = cursor
                .get(*segment)
                .ok_or_else(|| format!("missing field '{}'", path.join(".")))?;
        }
        cursor
            .as_u64()
            .ok_or_else(|| format!("field '{}' must be u64", path.join(".")))
    }

    fn evaluate_scale_aware_gate(
        current_report: &serde_json::Value,
        baseline_report: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        const LARGE_WAIT_RATIO_MAX: f64 = 0.60;
        const LARGE_COMPLETION_RATIO_MAX: f64 = 0.75;
        const SMALL_COMPLETION_RATIO_MAX: f64 = 1.25;
        const MAX_CANCELLED_RATE: f64 = 0.10;
        const MIN_COMPLETION_TOTAL: u64 = 50;

        let large_current_wait = get_report_metric_f64(
            current_report,
            &[
                "profiles",
                "large",
                "warm",
                "metrics",
                "intellisense_v2_wait_for_file_version_completion_ms",
                "p95",
            ],
        )?;
        let large_current_completion = get_report_metric_f64(
            current_report,
            &[
                "profiles",
                "large",
                "warm",
                "metrics",
                "completion_duration_ms",
                "p95",
            ],
        )?;
        let small_current_completion = get_report_metric_f64(
            current_report,
            &[
                "profiles",
                "small",
                "warm",
                "metrics",
                "completion_duration_ms",
                "p95",
            ],
        )?;

        let large_baseline_wait = get_report_metric_f64(
            baseline_report,
            &[
                "profiles",
                "large",
                "warm",
                "metrics",
                "intellisense_v2_wait_for_file_version_completion_ms",
                "p95",
            ],
        )?;
        let large_baseline_completion = get_report_metric_f64(
            baseline_report,
            &[
                "profiles",
                "large",
                "warm",
                "metrics",
                "completion_duration_ms",
                "p95",
            ],
        )?;
        let small_baseline_completion = get_report_metric_f64(
            baseline_report,
            &[
                "profiles",
                "small",
                "warm",
                "metrics",
                "completion_duration_ms",
                "p95",
            ],
        )?;

        let large_completion_total = get_report_u64(
            current_report,
            &["profiles", "large", "warm", "completion_total"],
        )?;
        let small_completion_total = get_report_u64(
            current_report,
            &["profiles", "small", "warm", "completion_total"],
        )?;
        let large_cancelled_total = get_report_u64(
            current_report,
            &["profiles", "large", "warm", "completion_cancelled_total"],
        )?;
        let small_cancelled_total = get_report_u64(
            current_report,
            &["profiles", "small", "warm", "completion_cancelled_total"],
        )?;

        let large_wait_ratio = large_current_wait / large_baseline_wait.max(0.000_001);
        let large_completion_ratio =
            large_current_completion / large_baseline_completion.max(0.000_001);
        let small_completion_ratio =
            small_current_completion / small_baseline_completion.max(0.000_001);

        let large_cancelled_rate =
            large_cancelled_total as f64 / large_completion_total.max(1) as f64;
        let small_cancelled_rate =
            small_cancelled_total as f64 / small_completion_total.max(1) as f64;

        let pass = large_wait_ratio <= LARGE_WAIT_RATIO_MAX
            && large_completion_ratio <= LARGE_COMPLETION_RATIO_MAX
            && small_completion_ratio <= SMALL_COMPLETION_RATIO_MAX
            && large_cancelled_rate <= MAX_CANCELLED_RATE
            && small_cancelled_rate <= MAX_CANCELLED_RATE
            && large_completion_total >= MIN_COMPLETION_TOTAL
            && small_completion_total >= MIN_COMPLETION_TOTAL;

        Ok(serde_json::json!({
            "pass": pass,
            "ratios": {
                "large_wait_ratio": large_wait_ratio,
                "large_completion_ratio": large_completion_ratio,
                "small_completion_ratio": small_completion_ratio
            },
            "rates": {
                "large_completion_cancelled_rate": large_cancelled_rate,
                "small_completion_cancelled_rate": small_cancelled_rate
            },
            "counts": {
                "large_completion_total": large_completion_total,
                "small_completion_total": small_completion_total
            },
            "thresholds": {
                "large_wait_ratio_max": LARGE_WAIT_RATIO_MAX,
                "large_completion_ratio_max": LARGE_COMPLETION_RATIO_MAX,
                "small_completion_ratio_max": SMALL_COMPLETION_RATIO_MAX,
                "completion_cancelled_rate_max": MAX_CANCELLED_RATE,
                "min_completion_total": MIN_COMPLETION_TOTAL
            }
        }))
    }

    #[tokio::test]
    async fn p31_scale_aware_large_small_completion_gate_live() {
        const CHANGE_ID: &str = "add-large-module-completion-acceleration-gate";
        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let conf_big_root = [
            workspace_root.join("examples").join("conf_big"),
            std::path::PathBuf::from("examples/conf_big"),
            std::path::PathBuf::from("../examples/conf_big"),
        ]
        .into_iter()
        .find(|path| path.join("Configuration.xml").exists());

        let Some(conf_big_root) = conf_big_root else {
            if allow_fixture_skip {
                eprintln!(
                    "skipping p31 scale-aware gate: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
                );
                return;
            }
            panic!(
                "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
            );
        };

        let large_module_rel = std::path::PathBuf::from("Documents")
            .join("РеализацияТоваровУслуг")
            .join("Forms")
            .join("ФормаДокументаОбщая")
            .join("Ext")
            .join("Form")
            .join("Module.bsl");
        let large_module_path = conf_big_root.join(&large_module_rel);
        if !large_module_path.exists() {
            if allow_fixture_skip {
                eprintln!(
                    "skipping p31 scale-aware gate: conf_big module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                    large_module_path.display()
                );
                return;
            }
            panic!(
                "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
                large_module_path.display()
            );
        }

        let small_module_path = workspace_root.join("examples").join("test_lsp.bsl");
        assert!(
            small_module_path.exists(),
            "small module fixture not found: {}",
            small_module_path.display()
        );

        let large_text = std::fs::read_to_string(&large_module_path)
            .expect("read conf_big module text for p31 scale-aware gate");
        let small_text = std::fs::read_to_string(&small_module_path)
            .expect("read small module text for p31 scale-aware gate");

        let large_position = find_utf16_position_after_marker(&large_text, "Объект.");
        let small_position = find_utf16_position_after_marker(&small_text, "Arr.");
        let phases = [
            ScaleAwarePhase {
                name: "start",
                warmup: 0,
                iterations: 1,
            },
            ScaleAwarePhase {
                name: "cold",
                warmup: 0,
                iterations: 5,
            },
            ScaleAwarePhase {
                name: "warm",
                warmup: 5,
                iterations: 50,
            },
        ];

        let large_profile = run_scale_aware_profile(
            "large",
            Url::parse("file:///p31_scale_large_module.bsl").expect("large uri"),
            large_text,
            large_position,
            &phases,
        )
        .await;
        let small_profile = run_scale_aware_profile(
            "small",
            Url::parse("file:///p31_scale_small_module.bsl").expect("small uri"),
            small_text,
            small_position,
            &phases,
        )
        .await;

        let mut report = serde_json::json!({
            "change_id": CHANGE_ID,
            "profile": "p31_scale_aware_large_small_completion_gate_live",
            "schema_version": 1,
            "phases": phases.iter().map(|phase| {
                serde_json::json!({
                    "name": phase.name,
                    "warmup": phase.warmup,
                    "iterations": phase.iterations
                })
            }).collect::<Vec<_>>(),
            "profiles": {
                "large": large_profile,
                "small": small_profile
            }
        });

        let baseline_path = std::env::var("BSL_V2_SCALE_AWARE_GATE_BASELINE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("baselines")
                    .join("add-large-module-completion-acceleration-gate.json")
            });
        let enforce_gate = std::env::var("BSL_V2_SCALE_AWARE_GATE_ENFORCE")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if baseline_path.exists() {
            let baseline_raw =
                std::fs::read_to_string(&baseline_path).expect("read scale-aware baseline file");
            let baseline_report: serde_json::Value =
                serde_json::from_str(&baseline_raw).expect("parse scale-aware baseline json");
            let gate = evaluate_scale_aware_gate(&report, &baseline_report)
                .expect("evaluate scale-aware large/small gate");
            report["baseline"] = serde_json::json!({
                "path": baseline_path,
                "present": true
            });
            report["gate"] = gate.clone();

            if enforce_gate {
                let pass = gate
                    .get("pass")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                assert!(
                    pass,
                    "p31 scale-aware gate failed in enforce mode: {}",
                    serde_json::to_string_pretty(&gate)
                        .unwrap_or_else(|_| "<gate json>".to_string())
                );
            }
        } else {
            report["baseline"] = serde_json::json!({
                "path": baseline_path,
                "present": false
            });
            report["gate"] = serde_json::json!({
                "evaluated": false,
                "reason": "baseline_missing"
            });
            if enforce_gate {
                panic!(
                    "BSL_V2_SCALE_AWARE_GATE_ENFORCE is enabled, but baseline is missing: {}",
                    baseline_path.display()
                );
            }
        }

        let report_path = std::env::var("BSL_V2_SCALE_AWARE_GATE_REPORT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests")
                    .join("perf")
                    .join("reports")
                    .join(format!("{CHANGE_ID}-live.json"))
            });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p31 scale-aware report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report).expect("serialize p31 scale-aware report"),
        )
        .expect("write p31 scale-aware report");
        println!("p31_scale_aware_gate_report={}", report_path.display());

        let large_warm_total =
            get_report_u64(&report, &["profiles", "large", "warm", "completion_total"])
                .expect("large warm completion_total");
        let small_warm_total =
            get_report_u64(&report, &["profiles", "small", "warm", "completion_total"])
                .expect("small warm completion_total");
        assert!(
            large_warm_total >= 50 && small_warm_total >= 50,
            "expected >=50 warm completion samples for both profiles, got large={} small={}",
            large_warm_total,
            small_warm_total
        );
    }
}
