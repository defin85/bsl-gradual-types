//! Core functionality: constructor and helper methods

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tracing::{info, warn};

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;

use crate::config::BslSettings;
use crate::converters::{semantic_error_to_diagnostic, syntax_errors_to_diagnostics};

use super::{BslLanguageServer, Url};

impl BslLanguageServer {
    pub fn new(client: Client, coordinator: Arc<SystemCoordinator>) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(BslSettings::default())),
            coordinator,
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
            match type_service.parse_and_validate(text) {
                Ok(errors) => {
                    if !errors.is_empty() {
                        info!(
                            "Found {} syntax errors in {} (revalidation)",
                            errors.len(),
                            uri
                        );
                        diagnostics.extend(syntax_errors_to_diagnostics(&errors));
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

            match type_service
                .validate_semantics(text, Some(detail_level))
                .await
            {
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
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;

        Ok(())
    }
}
