//! Core functionality: constructor and helper methods

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::Client;
use tracing::{info, warn};

use bsl_analysis_v2::{AnalysisHostV2, FileId as V2FileId};
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;

use crate::config::BslSettings;
use crate::converters::{semantic_error_to_diagnostic, syntax_errors_to_diagnostics};

use super::{BslLanguageServer, Url};

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
            analysis_host_v2: Arc::new(Mutex::new(AnalysisHostV2::default())),
            url_to_file_id_v2: Arc::new(RwLock::new(HashMap::new())),
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
        if let Some(&file_id) = self.url_to_file_id_v2.read().await.get(uri) {
            return file_id;
        }

        let mut map = self.url_to_file_id_v2.write().await;
        if let Some(&file_id) = map.get(uri) {
            return file_id;
        }

        let raw = self.next_file_id_v2.fetch_add(1, Ordering::Relaxed);
        let file_id = V2FileId(raw);
        map.insert(uri.clone(), file_id);
        file_id
    }

    pub(crate) async fn take_file_id_v2(&self, uri: &Url) -> Option<V2FileId> {
        self.url_to_file_id_v2.write().await.remove(uri)
    }
}
