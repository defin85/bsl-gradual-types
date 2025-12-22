//! Command-specific handlers for BslLanguageServer
//!
//! Contains implementation of helper methods for command handling.

use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::Url;
use tracing::{error, info};

use bsl_backend::system::fs_utils::read_bsl_file;
use crate::commands::{
    handle_incremental_update, handle_parse_configuration, ParseConfigurationParams,
};
use crate::handlers::{find_containing_function_in_dto, CurrentContextResponse};
use crate::types::{
    BuildIndexParams, BuildIndexResponse, GetCurrentContextParams, IncrementalUpdateParams,
    IncrementalUpdateResponse,
};

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
                Ok(path) => read_bsl_file(&path)
                    .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?,
                Err(_) => return Ok(CurrentContextResponse::empty()),
            },
        };

        let file_path = uri
            .to_file_path()
            .unwrap_or_else(|_| std::path::PathBuf::from("untitled"))
            .to_string_lossy()
            .to_string();

        if let Some(service) = self.get_type_service() {
            match service
                .get_semantic_tree(&file_content, &file_path, false, true, true)
                .await
            {
                Ok(semantic_tree_dto) => {
                    match find_containing_function_in_dto(
                        &semantic_tree_dto,
                        params.line,
                        params.character,
                    ) {
                        Some((name, kind, params_list, return_type)) => {
                            Ok(CurrentContextResponse {
                                function_name: Some(name),
                                function_kind: kind,
                                params: Some(params_list),
                                return_type,
                            })
                        }
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

    /// Custom request: bsl/buildIndex
    ///
    /// MVP: переиспользуем pipeline parseConfiguration (сервер — источник истины, прогресс через $/progress).
    pub(crate) async fn handle_build_index(
        &self,
        _params: BuildIndexParams,
    ) -> JsonRpcResult<BuildIndexResponse> {
        let cfg = self.config.read().await.clone();
        let Some(cfg) = cfg else {
            return Ok(BuildIndexResponse {
                success: false,
                types_count: 0,
                message: "LSP config not available (initializationOptions not received)".to_string(),
            });
        };

        let Some(config_path) = cfg.configuration_path else {
            return Ok(BuildIndexResponse {
                success: false,
                types_count: 0,
                message: "configurationPath is not configured".to_string(),
            });
        };

        let resp = handle_parse_configuration(
            ParseConfigurationParams { config_path },
            self.coordinator.get_analysis_engine(),
            self.client.clone(),
            "bsl-build-index",
            "Building BSL index",
            Some(self.coordinator.clone()),
        )
        .await;

        Ok(BuildIndexResponse {
            success: resp.success,
            types_count: resp.loaded_types,
            message: resp
                .message
                .unwrap_or_else(|| "Index build completed".to_string()),
        })
    }

    /// Custom request: bsl/incrementalUpdate
    ///
    /// MVP: сейчас это честная переиндексация конфигурации без перезапуска LSP.
    pub(crate) async fn handle_incremental_update(
        &self,
        params: IncrementalUpdateParams,
    ) -> JsonRpcResult<IncrementalUpdateResponse> {
        let resp =
            handle_incremental_update(params, self.coordinator.clone(), self.client.clone()).await;

        Ok(IncrementalUpdateResponse {
            success: resp.success,
            message: resp.message,
        })
    }
}
