//! Core functionality: constructor and helper methods

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::Client;
use tracing::{info, warn};

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

        let mut analysis_host_v2 = AnalysisHostV2::default();
        analysis_host_v2.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
            deps_id: compute_deps_id_v2(&coordinator),
            deps: compute_semantic_deps_v2(&coordinator),
        });
        analysis_host_v2.apply_change(bsl_analysis_v2::Change::SetSettingsId {
            settings_id: compute_settings_id_v2(&BslSettings::default()),
        });

        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            diagnostics_counts: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(BslSettings::default())),
            completion_snippet_support: Arc::new(RwLock::new(false)),
            auto_reindex_paused: Arc::new(RwLock::new(false)),
            coordinator,

            use_salsa_v2,
            analysis_host_v2: Arc::new(Mutex::new(analysis_host_v2)),
            file_key_to_file_id_v2: Arc::new(RwLock::new(HashMap::new())),
            next_file_id_v2: Arc::new(std::sync::atomic::AtomicU32::new(1)),
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

        let mut host = self.analysis_host_v2.lock().await;
        if host.deps_id() != deps_id {
            host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
                deps_id,
                deps: compute_semantic_deps_v2(&self.coordinator),
            });
        }
        if host.settings_id() != settings_id {
            host.apply_change(bsl_analysis_v2::Change::SetSettingsId { settings_id });
        }
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
