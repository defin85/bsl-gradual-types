//! Core functionality: constructor and helper methods

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::Client;
use tracing::{debug, info, warn};

use bsl_analysis_v2::{
    AnalysisHostV2, DepsSnapshotId, FileId as V2FileId, SemanticDeps, SettingsId,
};
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;

use crate::config::BslSettings;
use crate::converters::{semantic_error_to_diagnostic, syntax_errors_to_diagnostics};

use super::{BslLanguageServer, Url, V2FileKey};

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
        analysis_host_v2.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
            deps_id: compute_deps_id_v2(&coordinator),
            deps: compute_semantic_deps_v2(&coordinator),
        });
        analysis_host_v2.apply_change(bsl_analysis_v2::Change::SetSettingsSnapshot {
            settings_id: compute_settings_id_v2(&default_settings),
            diagnostics_detail_level: default_diagnostics_detail_level,
        });

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
            analysis_host_v2: Arc::new(Mutex::new(analysis_host_v2)),
            file_key_to_file_id_v2: Arc::new(RwLock::new(HashMap::new())),
            next_file_id_v2: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            diagnostics_tasks_v2: Arc::new(Mutex::new(HashMap::new())),
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

        let deps_id = compute_deps_id_v2(&self.coordinator);
        let settings = self.settings.read().await.clone();
        let settings_id = compute_settings_id_v2(&settings);
        let diagnostics_detail_level =
            bsl_shared::formatting::DetailLevel::parse(&settings.diagnostics.detail_level);

        let mut host = self.analysis_host_v2.lock().await;
        if host.deps_id() != deps_id {
            host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
                deps_id,
                deps: compute_semantic_deps_v2(&self.coordinator),
            });
        }
        if host.settings_id() != settings_id {
            host.apply_change(bsl_analysis_v2::Change::SetSettingsSnapshot {
                settings_id,
                diagnostics_detail_level,
            });
        }
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

            let (diagnostics, observed_deps_id, observed_settings_id, was_cancelled) = {
                let analysis = {
                    let host = server.analysis_host_v2.lock().await;
                    host.analysis()
                };

                let observed_deps_id = analysis
                    .deps_id()
                    .ok()
                    .map(|id| id.as_str().to_string());
                let observed_settings_id = analysis
                    .settings_id()
                    .ok()
                    .map(|id| id.as_str().to_string());

                let mut diagnostics = Vec::new();
                let mut was_cancelled = false;

                match analysis.syntax_diagnostics(file_id) {
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

                match analysis.semantic_diagnostics(file_id) {
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
                    was_cancelled,
                )
            };

            let diagnostics_len = diagnostics.len();

            let (is_current, current_version, current_deps_id, current_settings_id) = {
                let host = server.analysis_host_v2.lock().await;
                let analysis = host.analysis();
                let current_version = match analysis.file_version(file_id) {
                    Ok(version) => version,
                    Err(_) => None,
                };
                let current_deps_id = analysis.deps_id().ok().map(|id| id.as_str().to_string());
                let current_settings_id =
                    analysis.settings_id().ok().map(|id| id.as_str().to_string());
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

fn compute_deps_id_v2(coordinator: &SystemCoordinator) -> DepsSnapshotId {
    let snapshot_id = coordinator.intellisense_index_snapshot_id();
    let payload = format!(
        "schema={};index_snapshot_id={}",
        bsl_analysis_v2::DEPS_SCHEMA_VERSION,
        snapshot_id.as_str()
    );
    DepsSnapshotId::from_hash(blake3::hash(payload.as_bytes()).to_hex().to_string())
}

fn compute_semantic_deps_v2(coordinator: &SystemCoordinator) -> Arc<SemanticDeps> {
    match coordinator.analysis_engine() {
        Some(engine) => {
            let repository = engine.get_repository();
            let signature_index = repository.get_signature_index_clone();
            let resolver = Some(engine.get_resolver());
            Arc::new(SemanticDeps {
                repository,
                signature_index,
                resolver,
            })
        }
        None => {
            let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
            let signature_index = repository.get_signature_index_clone();
            let resolver = Some(Arc::new(TypeResolver::new(repository.clone())));
            Arc::new(SemanticDeps {
                repository,
                signature_index,
                resolver,
            })
        }
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
    use futures::StreamExt;
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::LspService;
    use tower_lsp::jsonrpc::Request;
    use tower_lsp::lsp_types::{
        ClientCapabilities, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
        InitializeParams, InitializedParams, TextDocumentContentChangeEvent, TextDocumentItem,
        VersionedTextDocumentIdentifier,
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
            process_id: None,
            root_path: None,
            root_uri: None,
            initialization_options: None,
            capabilities: ClientCapabilities::default(),
            trace: None,
            workspace_folders: None,
            client_info: None,
            locale: None,
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
}
