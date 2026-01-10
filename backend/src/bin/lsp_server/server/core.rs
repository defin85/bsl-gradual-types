//! Core functionality: constructor and helper methods

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::Client;
use tracing::{debug, info, warn};

use bsl_analysis_v2::{AnalysisHostV2, DepsSnapshotId, FileId as V2FileId, SettingsId};
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;

use crate::config::BslSettings;
use crate::converters::{semantic_error_to_diagnostic, syntax_errors_to_diagnostics};

use super::{BslLanguageServer, Url, V2FileKey};
use super::analysis_v2_runtime::AnalysisV2Runtime;
use super::deps_v2::{DepsBundleV2, DepsBundleV2Meta, build_deps_bundle_v2};

impl BslLanguageServer {
    pub fn new(client: Client, coordinator: Arc<SystemCoordinator>) -> Self {
        let use_salsa_v2 = matches!(
            std::env::var("BSL_INTELLISENSE_V2_SALSA")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        );
        info!("IntelliSense v2 salsa enabled: {}", use_salsa_v2);

        let default_settings = BslSettings::default();
        let default_diagnostics_detail_level =
            bsl_shared::formatting::DetailLevel::parse(&default_settings.diagnostics.detail_level);

        let mut analysis_host_v2 = AnalysisHostV2::default();
        let initial_deps_bundle =
            build_deps_bundle_v2(&coordinator, None, None).unwrap_or_else(|err| {
                warn!("Failed to build initial deps bundle v2: {}", err);

                let repository: Arc<dyn TypeRepository> =
                    Arc::new(InMemoryTypeRepository::new());
                let signature_index = repository.get_signature_index_clone();
                let resolver = Some(Arc::new(TypeResolver::new(repository.clone())));

                let semantic_deps = Arc::new(bsl_analysis_v2::SemanticDeps {
                    repository,
                    signature_index,
                    resolver,
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
        let analysis_v2 =
            AnalysisV2Runtime::new(analysis_host_v2, initial_deps_bundle.index_snapshot.clone());

        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            diagnostics_counts: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(default_settings)),
            completion_snippet_support: Arc::new(RwLock::new(false)),
            auto_reindex_paused: Arc::new(RwLock::new(false)),
            coordinator,

            use_salsa_v2,
            analysis_v2,
            file_key_to_file_id_v2: Arc::new(RwLock::new(HashMap::new())),
            next_file_id_v2: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            diagnostics_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
            latest_received_file_versions_v2: Arc::new(RwLock::new(HashMap::new())),
            last_deps_id_v2: Arc::new(RwLock::new(Some(initial_deps_id))),
            last_settings_id_v2: Arc::new(RwLock::new(Some(initial_settings_id))),
        }
    }

    /// Get current TypeSystemService (always fresh after reload)
    pub fn get_type_service(&self) -> Option<Arc<TypeSystemService>> {
        self.coordinator.type_service()
    }

    /// Get document content from cache
    pub async fn get_document_content(&self, uri: &Url) -> Result<String, String> {
        let documents = self.documents.read().await;
        documents
            .get(uri)
            .cloned()
            .ok_or_else(|| format!("Document not found: {}", uri))
    }

    /// Revalidate document (used after platform types loading)
    pub async fn revalidate_document(&self, uri: &Url, text: &str) -> Result<(), String> {
        let mut diagnostics = Vec::new();

        // PHASE 1: Syntax validation
        if let Some(type_service) = self.get_type_service() {
            let file_path = uri
                .to_file_path()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let parse_result = if let Some(ref path) = file_path {
                type_service.parse_and_validate_for_file(text, path)
            } else {
                type_service.parse_and_validate(text)
            };
            match parse_result {
                Ok(errors) => {
                    if !errors.is_empty() {
                        info!(
                            "Found {} syntax errors in {} (revalidation)",
                            errors.len(),
                            uri
                        );
                        diagnostics.extend(syntax_errors_to_diagnostics(&errors, uri));
                    }
                }
                Err(e) => {
                    warn!("Syntax validation failed for {} (revalidation): {}", uri, e);
                }
            }
        }

        // PHASE 2: Semantic validation
        if let Some(type_service) = self.get_type_service() {
            let settings = self.settings.read().await;
            let detail_level =
                bsl_shared::formatting::DetailLevel::parse(&settings.diagnostics.detail_level);

            let file_path = uri
                .to_file_path()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let semantic_result = match file_path {
                Some(ref path) => type_service
                    .validate_semantics_for_file(text, path, Some(detail_level))
                    .await,
                None => type_service.validate_semantics(text, Some(detail_level)).await,
            };
            match semantic_result {
                Ok(semantic_errors) => {
                    if !semantic_errors.is_empty() {
                        info!(
                            "Found {} semantic errors in {} (revalidation)",
                            semantic_errors.len(),
                            uri
                        );
                        for error in semantic_errors {
                            diagnostics.push(semantic_error_to_diagnostic(&error));
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Semantic validation failed for {} (revalidation): {}",
                        uri, e
                    );
                }
            }
        }

        // Send updated diagnostics
        let diagnostics_len = diagnostics.len();
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
        self.update_diagnostics_count(uri, diagnostics_len).await;

        Ok(())
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

    pub(crate) async fn sync_v2_globals(&self) {
        if !self.use_salsa_v2 {
            return;
        }

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
        if !self.use_salsa_v2 {
            return;
        }

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
                warn!("deps_update_v2 build failed: reason={}, error={}", reason, err);
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

        self.coordinator.record_intellisense_v2_deps_update_success();
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
        if let Some((_version, handle)) = tasks.remove(&file_id) {
            handle.abort();
        }
    }

    pub(crate) async fn schedule_diagnostics_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
    ) {
        if !self.use_salsa_v2 {
            return;
        }

        {
            let mut tasks = self.diagnostics_tasks_v2.lock().await;
            if let Some((_version, handle)) = tasks.remove(&file_id) {
                handle.abort();
            }
        }

        let server = self.clone();
        let uri_for_task = uri.clone();
        let handle = tokio::spawn(async move {
            let show_hints = {
                let settings = server.settings.read().await;
                settings.diagnostics.show_hints
            };

            let wait_started = Instant::now();
            let wait_ok = server
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            let wait_elapsed = wait_started.elapsed();
            server.coordinator.record_intellisense_v2_wait_for_file_version(
                "diagnostics",
                wait_elapsed,
            );
            if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri_for_task,
                        file_id = file_id.0,
                        expected_version,
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "diagnostics_v2: wait_for_file_version is slow"
                    );
                }
            }

            if !wait_ok {
                debug!(
                    uri = %uri_for_task,
                    file_id = file_id.0,
                    expected_version,
                    "diagnostics_v2: skip publish (wait_for_file_version failed)"
                );

                let mut tasks = server.diagnostics_tasks_v2.lock().await;
                if let Some((registered_version, _)) = tasks.get(&file_id) {
                    if *registered_version == expected_version {
                        tasks.remove(&file_id);
                    }
                }
                return;
            }

            let (
                diagnostics,
                observed_deps_id,
                observed_settings_id,
                observed_index_snapshot_id,
                was_cancelled,
            ) = {
                let snapshot_started = Instant::now();
                let (analysis, index_snapshot, deps_id) =
                    server.analysis_v2.snapshot_with_deps().await;
                let snapshot_elapsed = snapshot_started.elapsed();
                server
                    .coordinator
                    .record_intellisense_v2_snapshot_latency("diagnostics", snapshot_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                    if snapshot_elapsed >= threshold {
                        warn!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version,
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
                    expected_version,
                    deps_id = observed_deps_id.as_deref().unwrap_or_default(),
                    settings_id = observed_settings_id.as_deref().unwrap_or_default(),
                    index_snapshot_id = observed_index_snapshot_id,
                    "diagnostics_v2 observed snapshot"
                );

                let mut diagnostics = Vec::new();
                let mut was_cancelled = false;

                let syntax_started = Instant::now();
                let syntax_result = analysis.syntax_diagnostics(file_id);
                let syntax_elapsed = syntax_started.elapsed();
                server
                    .coordinator
                    .record_intellisense_v2_syntax_diagnostics_query_latency(syntax_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                    if syntax_elapsed >= threshold {
                        warn!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version,
                            syntax_diagnostics_ms = syntax_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "diagnostics_v2: syntax_diagnostics query is slow"
                        );
                    }
                }

                match syntax_result {
                    Ok(Some(syntax_errors)) => {
                        diagnostics.extend(syntax_errors_to_diagnostics(
                            &syntax_errors,
                            &uri_for_task,
                        ));
                    }
                    Ok(None) => {}
                    Err(cancelled) => {
                        was_cancelled = true;
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version,
                            error = ?cancelled,
                            "diagnostics_v2: syntax diagnostics cancelled"
                        );
                    }
                }

                let semantic_started = Instant::now();
                let semantic_result = analysis.semantic_diagnostics(file_id);
                let semantic_elapsed = semantic_started.elapsed();
                server
                    .coordinator
                    .record_intellisense_v2_semantic_diagnostics_query_latency(semantic_elapsed);
                if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                    if semantic_elapsed >= threshold {
                        warn!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version,
                            semantic_diagnostics_ms = semantic_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "diagnostics_v2: semantic_diagnostics query is slow"
                        );
                    }
                }

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
                            diagnostics.push(semantic_error_to_diagnostic(error));
                        }
                    }
                    Ok(None) => {}
                    Err(cancelled) => {
                        was_cancelled = true;
                        debug!(
                            uri = %uri_for_task,
                            file_id = file_id.0,
                            expected_version,
                            error = ?cancelled,
                            "diagnostics_v2: semantic diagnostics cancelled"
                        );
                    }
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
                    && current_version == Some(expected_version)
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
                        expected_version,
                        "diagnostics_v2: skip publish (cancelled)"
                    );
                } else if current_version.is_none() {
                    debug!(
                        uri = %uri_for_task,
                        file_id = file_id.0,
                        expected_version,
                        "diagnostics_v2: skip publish (no file)"
                    );
                } else if current_version != Some(expected_version) {
                    debug!(
                        uri = %uri_for_task,
                        file_id = file_id.0,
                        expected_version,
                        current_version = current_version.unwrap_or(-1),
                        "diagnostics_v2: skip publish (stale version)"
                    );
                } else if current_deps_id != observed_deps_id {
                    debug!(
                        uri = %uri_for_task,
                        file_id = file_id.0,
                        expected_version,
                        observed_deps_id = ?observed_deps_id,
                        current_deps_id = ?current_deps_id,
                        "diagnostics_v2: skip publish (stale deps_id)"
                    );
                } else {
                    debug!(
                        uri = %uri_for_task,
                        file_id = file_id.0,
                        expected_version,
                        observed_settings_id = ?observed_settings_id,
                        current_settings_id = ?current_settings_id,
                        "diagnostics_v2: skip publish (stale settings_id)"
                    );
                }

                let mut tasks = server.diagnostics_tasks_v2.lock().await;
                if let Some((registered_version, _)) = tasks.get(&file_id) {
                    if *registered_version == expected_version {
                        tasks.remove(&file_id);
                    }
                }
                return;
            }

            debug!(
                uri = %uri_for_task,
                file_id = file_id.0,
                expected_version,
                deps_id = observed_deps_id.as_deref().unwrap_or_default(),
                settings_id = observed_settings_id.as_deref().unwrap_or_default(),
                index_snapshot_id = observed_index_snapshot_id,
                diagnostics_len,
                "diagnostics_v2: publish diagnostics"
            );

            server
                .client
                .publish_diagnostics(uri_for_task.clone(), diagnostics, Some(expected_version))
                .await;
            server
                .update_diagnostics_count(&uri_for_task, diagnostics_len)
                .await;

            let mut tasks = server.diagnostics_tasks_v2.lock().await;
            if let Some((registered_version, _)) = tasks.get(&file_id) {
                if *registered_version == expected_version {
                    tasks.remove(&file_id);
                }
            }
        });

        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        tasks.insert(file_id, (expected_version, handle));
    }
}

fn compute_settings_id_v2(settings: &BslSettings) -> SettingsId {
    let payload = format!(
        "schema={};hover.detail_level={};hover.max_methods={};hover.max_properties={};hover.show_certainty={};diagnostics.detail_level={};diagnostics.show_hints={}",
        bsl_analysis_v2::SETTINGS_SCHEMA_VERSION,
        settings.hover.detail_level,
        settings.hover.max_methods,
        settings.hover.max_properties,
        settings.hover.show_certainty,
        settings.diagnostics.detail_level,
        settings.diagnostics.show_hints
    );
    SettingsId::from_hash(blake3::hash(payload.as_bytes()).to_hex().to_string())
}

#[cfg(test)]
    mod tests {
        use super::*;
        use bsl_backend::system::{
            IndexItem, IndexItemKind, IndexKind, IndexSnapshot, IndexSnapshotId, TypeKind,
        };
        use futures::StreamExt;
        use tower::Service;
        use tower::ServiceExt;
        use tower_lsp::LspService;
        use tower_lsp::jsonrpc::Request;
        use tower_lsp::LanguageServer;
        use tower_lsp::lsp_types::{
            ClientCapabilities, CompletionParams, DidChangeTextDocumentParams,
            DidOpenTextDocumentParams, InitializeParams, InitializedParams,
            PartialResultParams, Position, TextDocumentContentChangeEvent, TextDocumentIdentifier,
            TextDocumentItem, TextDocumentPositionParams, VersionedTextDocumentIdentifier,
            WorkDoneProgressParams,
        };

    #[tokio::test]
        async fn p6_fast_did_change_series_publish_diagnostics_is_monotonic() {
        let old_flag = std::env::var("BSL_INTELLISENSE_V2_SALSA").ok();
        std::env::set_var("BSL_INTELLISENSE_V2_SALSA", "1");

        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let (published_tx, mut published_rx) =
            tokio::sync::mpsc::unbounded_channel::<tower_lsp::lsp_types::PublishDiagnosticsParams>();

        let drain_task = tokio::spawn(async move {
            while let Some(req) = socket.next().await {
                if req.method() != "textDocument/publishDiagnostics" {
                    continue;
                }
                let Some(params) = req.params().cloned() else {
                    continue;
                };
                let Ok(parsed) =
                    serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
                else {
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
        assert!(initialized_response.is_none(), "initialized is a notification");

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
        assert!(did_change_response_v2.is_none(), "didChange is a notification");

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
        assert!(did_change_response_v3.is_none(), "didChange is a notification");

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
            versions.iter().any(|v| *v == 3),
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
            let remaining =
                after_deadline.saturating_duration_since(tokio::time::Instant::now());
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
            assert!(version >= 3, "unexpected jump-back diagnostics: v{}", version);
        }

        drain_task.abort();

        match old_flag {
            Some(value) => std::env::set_var("BSL_INTELLISENSE_V2_SALSA", value),
            None => std::env::remove_var("BSL_INTELLISENSE_V2_SALSA"),
        }
    }

    #[tokio::test]
    async fn p7_completion_after_did_change_does_not_hang() {
        let old_flag = std::env::var("BSL_INTELLISENSE_V2_SALSA").ok();
        std::env::set_var("BSL_INTELLISENSE_V2_SALSA", "1");

        let coordinator = Arc::new(SystemCoordinator::new());

        let (mut service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            move |client| BslLanguageServer::new(client, coordinator.clone())
        })
        .finish();

        let drain_task = tokio::spawn(async move {
            while let Some(_req) = socket.next().await {}
        });

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
        assert!(initialized_response.is_none(), "initialized is a notification");

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

        assert!(completion_response.is_some(), "completion should return a response");

        drain_task.abort();

        match old_flag {
            Some(value) => std::env::set_var("BSL_INTELLISENSE_V2_SALSA", value),
            None => std::env::remove_var("BSL_INTELLISENSE_V2_SALSA"),
        }
    }

    #[tokio::test]
    async fn p8_deps_update_is_atomic_and_completion_uses_runtime_index_snapshot() {
        fn make_index_snapshot(id: &str, type_name: &str) -> IndexSnapshot {
            let mut snapshot = IndexSnapshot::empty(IndexSnapshotId::from_hash(id.to_string()));
            snapshot.type_index.insert(
                type_name.to_string(),
                IndexItem::new(
                    type_name.to_string(),
                    IndexItemKind::Type(TypeKind::Generic),
                    IndexKind::Type,
                ),
            );
            snapshot
        }

        fn extract_completion_labels(response: tower_lsp::lsp_types::CompletionResponse) -> Vec<String> {
            match response {
                tower_lsp::lsp_types::CompletionResponse::Array(items) => {
                    items.into_iter().map(|item| item.label).collect()
                }
                tower_lsp::lsp_types::CompletionResponse::List(list) => {
                    list.items.into_iter().map(|item| item.label).collect()
                }
            }
        }

        let old_flag = std::env::var("BSL_INTELLISENSE_V2_SALSA").ok();
        std::env::set_var("BSL_INTELLISENSE_V2_SALSA", "1");

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

        let drain_task = tokio::spawn(async move {
            while let Some(_req) = socket.next().await {}
        });

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
        assert!(initialized_response.is_none(), "initialized is a notification");

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
        let expected_deps_id_a =
            build_deps_bundle_v2(coordinator.as_ref(), None, None)
                .expect("bundle A")
                .deps_id;

        coordinator
            .intellisense_index()
            .replace_snapshot(snapshot_b.clone());
        let expected_deps_id_b =
            build_deps_bundle_v2(coordinator.as_ref(), None, None)
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
                position: Position { line: 1, character: 6 },
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
                coordinator.intellisense_index().replace_snapshot(snapshot_b);
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

        match old_flag {
            Some(value) => std::env::set_var("BSL_INTELLISENSE_V2_SALSA", value),
            None => std::env::remove_var("BSL_INTELLISENSE_V2_SALSA"),
        }
    }
}
