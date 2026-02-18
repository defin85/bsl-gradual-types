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
    BslLanguageServer, CodeActionsCapabilityState, FormattingCapabilityState,
    InlayHintsCapabilityState, Url, V2FileKey,
};

fn diagnostics_debounce_duration() -> Duration {
    // Diagnostics are triggered on every `textDocument/didChange`. Computing full diagnostics is
    // CPU-bound and not preemptible (abort only works at await points). Without debouncing, rapid
    // typing can build up a backlog and make completion/hover feel "frozen".
    //
    // Default: 250ms. Can be overridden via env for experiments.
    let raw = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::LspDiagnosticsDebounceMs)
        .unwrap_or(250);
    Duration::from_millis(raw)
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
            latest_received_file_versions_v2: Arc::new(RwLock::new(HashMap::new())),
            completion_seen_files_v2: Arc::new(RwLock::new(std::collections::HashSet::new())),
            completion_stale_fallback_cache_v2: Arc::new(RwLock::new(HashMap::new())),
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
                        text: Arc::from(file_content),
                        version: 0,
                        path: Arc::from(path_string),
                    }]);
                0
            }
        };

        let context = self
            .build_execution_context_v2(operation, file_id, Some(min_file_version), flow_sensitive)
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
        if let Some(task) = tasks.remove(&file_id) {
            task.handle.abort();
        }
    }

    pub(crate) async fn schedule_diagnostics_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
        debounce: bool,
    ) {
        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        if let Some(task) = tasks.get_mut(&file_id) {
            task.requested_version = expected_version;
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

                let (requested_version, debounce) = {
                    let tasks = server.diagnostics_tasks_v2.lock().await;
                    let Some(task) = tasks.get(&file_id) else {
                        break;
                    };
                    (task.requested_version, task.debounce)
                };

                // Coalesce rapid edits: while user is typing, keep moving the target forward.
                if debounce {
                    let delay = diagnostics_debounce_duration();
                    if delay != Duration::from_millis(0) {
                        tokio::time::sleep(delay).await;
                    }

                    let current_requested = {
                        let tasks = server.diagnostics_tasks_v2.lock().await;
                        let Some(task) = tasks.get(&file_id) else {
                            break;
                        };
                        task.requested_version
                    };
                    if current_requested != requested_version {
                        continue;
                    }
                }

                let show_hints = {
                    let settings = server.settings.read().await;
                    settings.diagnostics.show_hints
                };
                let include_flow_sensitive = {
                    let settings = server.settings.read().await;
                    settings.enable_flow_sensitive
                };

                // If a newer version is requested, skip work for the stale one early.
                let current_requested = {
                    let tasks = server.diagnostics_tasks_v2.lock().await;
                    let Some(task) = tasks.get(&file_id) else {
                        break;
                    };
                    task.requested_version
                };
                if current_requested != requested_version {
                    continue;
                }

                let context = server
                    .build_execution_context_v2(
                        bsl_runtime::application::SemanticOperation::Diagnostics,
                        file_id,
                        Some(requested_version),
                        include_flow_sensitive,
                    )
                    .await;
                let prepared = match server
                    .analysis_v2
                    .prepare_stateful_operation(&context, Some(server.coordinator.as_ref()))
                    .await
                {
                    Ok(prepared) => prepared,
                    Err(outcome) => {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            outcome = outcome.as_str(),
                            "diagnostics_v2: skip publish (stateful operation not ready)"
                        );
                        break;
                    }
                };

                let wait_elapsed = prepared.wait_elapsed.unwrap_or(Duration::ZERO);
                if wait_elapsed > Duration::ZERO {
                    if let Some(threshold) = super::intellisense_v2_slow_client_log_threshold() {
                        if wait_elapsed >= threshold {
                            server
                                .client
                                .log_message(
                                    MessageType::INFO,
                                    format!(
                                        "[perf] diagnostics_v2 wait_for_file_version: wait_ms={} uri={} file_id={} expected_version={}",
                                        wait_elapsed.as_millis(),
                                        uri_for_task,
                                        file_id.0,
                                        requested_version
                                    ),
                                )
                                .await;
                        }
                    }
                    if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                        if wait_elapsed >= threshold {
                            warn!(
                                uri = %uri_for_task,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                wait_ms = wait_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: wait_for_file_version is slow"
                            );
                        }
                    }
                }

                // If a newer version is requested, skip work for the stale one before snapshot.
                let current_requested = {
                    let tasks = server.diagnostics_tasks_v2.lock().await;
                    let Some(task) = tasks.get(&file_id) else {
                        break;
                    };
                    task.requested_version
                };
                if current_requested != requested_version {
                    continue;
                }

                let (
                    diagnostics,
                    observed_deps_id,
                    observed_settings_id,
                    observed_index_snapshot_id,
                    was_cancelled,
                ) = {
                    let analysis = prepared.snapshot.analysis;
                    let index_snapshot = prepared.snapshot.index_snapshot;
                    let deps_id = prepared.snapshot.deps_id;
                    let snapshot_elapsed = prepared.snapshot_elapsed;
                    if let Some(threshold) = super::intellisense_v2_slow_client_log_threshold() {
                        if snapshot_elapsed >= threshold {
                            server
                                .client
                                .log_message(
                                    MessageType::INFO,
                                    format!(
                                        "[perf] diagnostics_v2 snapshot: snapshot_ms={} uri={} file_id={} expected_version={}",
                                        snapshot_elapsed.as_millis(),
                                        uri_for_task,
                                        file_id.0,
                                        requested_version
                                    ),
                                )
                                .await;
                        }
                    }
                    if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                        if snapshot_elapsed >= threshold {
                            warn!(
                                uri = %uri_for_task,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                snapshot_ms = snapshot_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: snapshot acquisition is slow"
                            );
                        }
                    }

                    let observed_deps_id = Some(deps_id.as_str().to_string());
                    let observed_settings_id = analysis
                        .settings_id()
                        .ok()
                        .map(|id| id.as_str().to_string());
                    let observed_index_snapshot_id = index_snapshot.id.as_str().to_string();

                    debug!(
                        uri = %uri_for_task,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        deps_id = observed_deps_id.as_deref().unwrap_or_default(),
                        settings_id = observed_settings_id.as_deref().unwrap_or_default(),
                        index_snapshot_id = observed_index_snapshot_id,
                        "diagnostics_v2 observed snapshot"
                    );

                    let uri_for_blocking = uri_for_task.clone();
                    let coordinator_for_blocking = server.coordinator.clone();
                    let context_for_blocking = context.clone();
                    let (
                        diagnostics,
                        was_cancelled,
                        parse_result_elapsed,
                        syntax_elapsed,
                        semantic_elapsed,
                        parse_result_cancelled_error,
                        syntax_cancelled_error,
                        semantic_cancelled_error,
                    ) = match bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                        bsl_runtime::application::CpuWorkClass::Background,
                        context_for_blocking.origin.as_str(),
                        Some(server.coordinator.as_ref()),
                        move || {
                            let mut diagnostics = Vec::new();
                            let mut was_cancelled = false;
                            let mut parse_result_cancelled_error = None;
                            let mut syntax_cancelled_error = None;
                            let mut semantic_cancelled_error = None;

                        let file_text = analysis.file_text(file_id).ok().flatten();
                        let line_index = analysis.line_index(file_id).ok().flatten();

                        let parse_result_elapsed = if bsl_runtime::application::should_query_parse_result(
                            context_for_blocking.operation,
                            false,
                        ) {
                            let parse_result_started = Instant::now();
                            let parse_result_query =
                                bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                                    &context_for_blocking,
                                    &analysis,
                                    false,
                                    Some(coordinator_for_blocking.as_ref()),
                                    file_id,
                                );
                            if let Err(cancelled) = parse_result_query {
                                was_cancelled = true;
                                parse_result_cancelled_error = Some(format!("{cancelled:?}"));
                            }
                            parse_result_started.elapsed()
                        } else {
                            std::time::Duration::ZERO
                        };

                        let syntax_started = Instant::now();
                        let syntax_result =
                            bsl_runtime::application::IntellisenseV2Facade::run_syntax_diagnostics_query_singleflight(
                                &context_for_blocking,
                                &analysis,
                                Some(coordinator_for_blocking.as_ref()),
                                file_id,
                            );
                        let syntax_elapsed = syntax_started.elapsed();
                        match syntax_result {
                            Ok(Some(syntax_errors)) => {
                                if let (Some(text), Some(index)) =
                                    (file_text.as_deref(), line_index.as_deref())
                                {
                                    diagnostics.extend(syntax_errors_to_diagnostics(
                                        &syntax_errors,
                                        &uri_for_blocking,
                                        text,
                                        index,
                                    ));
                                }
                            }
                            Ok(None) => {}
                            Err(cancelled) => {
                                was_cancelled = true;
                                syntax_cancelled_error = Some(format!("{cancelled:?}"));
                            }
                        }

                        let semantic_started = Instant::now();
                        let semantic_result =
                            bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
                                &context_for_blocking,
                                bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
                                &analysis,
                                Some(coordinator_for_blocking.as_ref()),
                                |analysis| {
                                    if include_flow_sensitive {
                                        analysis.semantic_diagnostics_flow_sensitive(file_id)
                                    } else {
                                        analysis.semantic_diagnostics(file_id)
                                    }
                                },
                            );
                        let semantic_elapsed = semantic_started.elapsed();
                        match semantic_result {
                            Ok(Some(semantic_errors)) => {
                                for error in semantic_errors.iter() {
                                    if !show_hints
                                        && matches!(
                                            error.severity,
                                            bsl_shared::domain::types::DiagnosticSeverity::Hint
                                        )
                                    {
                                        continue;
                                    }
                                    if let (Some(text), Some(index)) =
                                        (file_text.as_deref(), line_index.as_deref())
                                    {
                                        diagnostics
                                            .push(semantic_error_to_diagnostic(error, text, index));
                                    }
                                }
                            }
                            Ok(None) => {}
                            Err(cancelled) => {
                                was_cancelled = true;
                                semantic_cancelled_error = Some(format!("{cancelled:?}"));
                            }
                        }

                            (
                                diagnostics,
                                was_cancelled,
                                parse_result_elapsed,
                                syntax_elapsed,
                                semantic_elapsed,
                                parse_result_cancelled_error,
                                syntax_cancelled_error,
                                semantic_cancelled_error,
                            )
                        },
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(err) => {
                            warn!(
                                uri = %uri_for_task,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                error = ?err,
                                "diagnostics_v2: spawn_blocking failed"
                            );
                            (
                                Vec::new(),
                                true,
                                Duration::from_millis(0),
                                Duration::from_millis(0),
                                Duration::from_millis(0),
                                None,
                                None,
                                None,
                            )
                        }
                    };

                    if parse_result_elapsed > std::time::Duration::ZERO {
                        if let Some(threshold) = super::intellisense_v2_slow_client_log_threshold()
                        {
                            if parse_result_elapsed >= threshold {
                                server
                                    .client
                                    .log_message(
                                        MessageType::INFO,
                                        format!(
                                            "[perf] diagnostics_v2 parse_result: parse_ms={} uri={} file_id={} expected_version={}",
                                            parse_result_elapsed.as_millis(),
                                            uri_for_task,
                                            file_id.0,
                                            requested_version
                                        ),
                                    )
                                    .await;
                            }
                        }
                        if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold()
                        {
                            if parse_result_elapsed >= threshold {
                                warn!(
                                    uri = %uri_for_task,
                                    file_id = file_id.0,
                                    expected_version = requested_version,
                                    parse_result_ms = parse_result_elapsed.as_millis(),
                                    threshold_ms = threshold.as_millis(),
                                    "diagnostics_v2: parse_result query is slow"
                                );
                            }
                        }
                    }
                    if let Some(cancelled_error) = parse_result_cancelled_error {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            error = %cancelled_error,
                            "diagnostics_v2: parse_result query cancelled"
                        );
                    }

                    if let Some(threshold) = super::intellisense_v2_slow_client_log_threshold() {
                        if syntax_elapsed >= threshold {
                            server
                                .client
                                .log_message(
                                    MessageType::INFO,
                                    format!(
                                        "[perf] diagnostics_v2 syntax_diagnostics: syntax_ms={} uri={} file_id={} expected_version={}",
                                        syntax_elapsed.as_millis(),
                                        uri_for_task,
                                        file_id.0,
                                        requested_version
                                    ),
                                )
                                .await;
                        }
                    }
                    if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                        if syntax_elapsed >= threshold {
                            warn!(
                                uri = %uri_for_task,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                syntax_diagnostics_ms = syntax_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: syntax_diagnostics query is slow"
                            );
                        }
                    }
                    if let Some(cancelled_error) = syntax_cancelled_error {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            error = %cancelled_error,
                            "diagnostics_v2: syntax diagnostics cancelled"
                        );
                    }

                    if let Some(threshold) = super::intellisense_v2_slow_client_log_threshold() {
                        if semantic_elapsed >= threshold {
                            server
                                .client
                                .log_message(
                                    MessageType::INFO,
                                    format!(
                                        "[perf] diagnostics_v2 semantic_diagnostics: semantic_ms={} uri={} file_id={} expected_version={}",
                                        semantic_elapsed.as_millis(),
                                        uri_for_task,
                                        file_id.0,
                                        requested_version
                                    ),
                                )
                                .await;
                        }
                    }
                    if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                        if semantic_elapsed >= threshold {
                            warn!(
                                uri = %uri_for_task,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                semantic_diagnostics_ms = semantic_elapsed.as_millis(),
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: semantic_diagnostics query is slow"
                            );
                        }
                    }
                    if let Some(cancelled_error) = semantic_cancelled_error {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            error = %cancelled_error,
                            "diagnostics_v2: semantic diagnostics cancelled"
                        );
                    }

                    (
                        diagnostics,
                        observed_deps_id,
                        observed_settings_id,
                        observed_index_snapshot_id,
                        was_cancelled,
                    )
                };

                let diagnostics_len = diagnostics.len();

                let (is_current, current_version, current_deps_id, current_settings_id) = {
                    let current_version = server
                        .latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied();
                    let current_deps_id = server
                        .last_deps_id_v2
                        .read()
                        .await
                        .as_ref()
                        .map(|id| id.as_str().to_string());
                    let current_settings_id = server
                        .last_settings_id_v2
                        .read()
                        .await
                        .as_ref()
                        .map(|id| id.as_str().to_string());
                    let is_current = !was_cancelled
                        && current_version == Some(requested_version)
                        && current_deps_id == observed_deps_id
                        && current_settings_id == observed_settings_id;
                    (
                        is_current,
                        current_version,
                        current_deps_id,
                        current_settings_id,
                    )
                };

                if !is_current {
                    if was_cancelled {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            "diagnostics_v2: skip publish (cancelled)"
                        );
                    } else if current_version.is_none() {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            "diagnostics_v2: skip publish (no file)"
                        );
                    } else if current_version != Some(requested_version) {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            current_version = current_version.unwrap_or(-1),
                            "diagnostics_v2: skip publish (stale version)"
                        );
                    } else if current_deps_id != observed_deps_id {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            observed_deps_id = ?observed_deps_id,
                            current_deps_id = ?current_deps_id,
                            "diagnostics_v2: skip publish (stale deps_id)"
                        );
                    } else {
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            observed_settings_id = ?observed_settings_id,
                            current_settings_id = ?current_settings_id,
                            "diagnostics_v2: skip publish (stale settings_id)"
                        );
                    }
                    continue;
                }

                debug!(
                    uri = %uri_for_task,
                    file_id = file_id.0,
                    expected_version = requested_version,
                    deps_id = observed_deps_id.as_deref().unwrap_or_default(),
                    settings_id = observed_settings_id.as_deref().unwrap_or_default(),
                    index_snapshot_id = observed_index_snapshot_id,
                    diagnostics_len,
                    "diagnostics_v2: publish diagnostics"
                );

                server
                    .client
                    .publish_diagnostics(uri_for_task.clone(), diagnostics, Some(requested_version))
                    .await;
                server
                    .update_diagnostics_count(&uri_for_task, diagnostics_len)
                    .await;

                // If nothing newer was requested while we were working, we can stop.
                let mut tasks = server.diagnostics_tasks_v2.lock().await;
                let Some(task) = tasks.get(&file_id) else {
                    break;
                };
                if task.requested_version == requested_version {
                    tasks.remove(&file_id);
                    break;
                }
            }
        });

        tasks.insert(
            file_id,
            super::DiagnosticsTaskV2 {
                requested_version: expected_version,
                debounce,
                handle,
            },
        );
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
        CompletionItemKind, CompletionParams, CompletionResponse, DidChangeConfigurationParams,
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, DocumentFormattingParams,
        DocumentRangeFormattingParams, DocumentSymbolParams, DocumentSymbolResponse,
        FormattingOptions, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
        HoverParams, InitializeParams, InitializedParams, InlayHint, InlayHintLabel,
        InlayHintParams, Location, MarkedString, PartialResultParams, Position,
        PrepareRenameResponse, PublishDiagnosticsParams, Range, ReferenceContext, ReferenceParams,
        RenameParams, SymbolInformation, SymbolKind, TextDocumentContentChangeEvent,
        TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url,
        VersionedTextDocumentIdentifier, WorkDoneProgressParams, WorkspaceEdit,
        WorkspaceSymbolParams,
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
            HoverContents::Array(values) => values.into_iter().find_map(|value| match value {
                MarkedString::String(value) => Some(value),
                MarkedString::LanguageString(value) => Some(value.value),
            }),
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
            if std::env::var_os("CI").is_some() {
                panic!("examples/conf_big fixture is missing");
            }
            eprintln!("skipping p26 warm-path SLO smoke: examples/conf_big fixture is missing");
            return;
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
            if std::env::var_os("CI").is_some() {
                panic!(
                    "conf_big module fixture is missing: {}",
                    module_path.display()
                );
            }
            eprintln!(
                "skipping p26 warm-path SLO smoke: module fixture is missing: {}",
                module_path.display()
            );
            return;
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

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
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
        let completion_p95 = completion_hist
            .get("p95")
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .expect("completion p95");
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
}
