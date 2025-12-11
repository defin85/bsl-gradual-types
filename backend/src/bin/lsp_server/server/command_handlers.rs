//! Command-specific handlers for BslLanguageServer
//!
//! Contains implementation of helper methods for command handling.

use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::Url;
use tracing::{error, info};

use crate::handlers::{find_containing_function_in_dto, CurrentContextResponse};
use crate::types::GetCurrentContextParams;

use super::BslLanguageServer;

impl BslLanguageServer {
    /// Handle bsl.getCurrentContext command
    pub(crate) async fn handle_get_current_context(
        &self,
        params: GetCurrentContextParams,
    ) -> JsonRpcResult<CurrentContextResponse> {
        info!(
            "Custom command: bsl.getCurrentContext - {}:{}:{}",
            params.uri, params.line, params.character
        );

        let uri = Url::parse(&params.uri).map_err(|e| {
            tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
        })?;

        let file_content = match self.documents.read().await.get(&uri) {
            Some(content) => content.clone(),
            None => match uri.to_file_path() {
                Ok(path) => std::fs::read_to_string(&path).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?,
                Err(_) => return Ok(CurrentContextResponse::empty()),
            },
        };

        let file_path = uri
            .to_file_path()
            .unwrap_or_else(|_| std::path::PathBuf::from("untitled"))
            .to_string_lossy()
            .to_string();

        if let Some(service) = self.get_type_service() {
            match service.get_semantic_tree(&file_content, &file_path).await {
                Ok(semantic_tree_dto) => {
                    match find_containing_function_in_dto(
                        &semantic_tree_dto,
                        params.line,
                        params.character,
                    ) {
                        Some((name, kind, params_list, return_type)) => Ok(CurrentContextResponse {
                            function_name: Some(name),
                            function_kind: kind,
                            params: Some(params_list),
                            return_type,
                        }),
                        None => Ok(CurrentContextResponse::empty()),
                    }
                }
                Err(e) => {
                    error!("Failed to get semantic tree: {}", e);
                    Err(tower_lsp::jsonrpc::Error::internal_error())
                }
            }
        } else {
            Ok(CurrentContextResponse::empty())
        }
    }
}
