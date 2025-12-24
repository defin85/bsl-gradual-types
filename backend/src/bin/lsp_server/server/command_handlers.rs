//! Command-specific handlers for BslLanguageServer
//!
//! Contains implementation of helper methods for command handling.

use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{MessageType, Url};
use tracing::{error, info, warn};

use std::path::{Path, PathBuf};

use bsl_backend::system::fs_utils::read_bsl_file;
use crate::commands::{
    handle_incremental_update, handle_parse_configuration, ParseConfigurationParams,
};
use crate::handlers::{find_containing_function_in_dto, CurrentContextResponse};
use crate::types::{
    AutoReindexCommandParams, AutoReindexStateResponse, BuildIndexParams, BuildIndexResponse,
    GetCurrentContextParams, IncrementalUpdateParams, IncrementalUpdateResponse,
    WorkspaceStatsResponse,
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
        if params.is_auto {
            let paused = *self.auto_reindex_paused.read().await;
            if paused {
                warn!("Auto reindex skipped: paused");
                self.client
                    .log_message(
                        MessageType::INFO,
                        "Auto reindex is paused; incrementalUpdate skipped.",
                    )
                    .await;
                return Ok(IncrementalUpdateResponse {
                    success: false,
                    message: "Auto reindex paused".to_string(),
                });
            }
        }

        let resp =
            handle_incremental_update(params, self.coordinator.clone(), self.client.clone()).await;

        Ok(IncrementalUpdateResponse {
            success: resp.success,
            message: resp.message,
        })
    }

    /// Custom request: bsl/pauseAutoReindex
    pub(crate) async fn handle_pause_auto_reindex(
        &self,
        _params: AutoReindexCommandParams,
    ) -> JsonRpcResult<AutoReindexStateResponse> {
        let mut paused = self.auto_reindex_paused.write().await;
        if !*paused {
            *paused = true;
            info!("Auto reindex paused via LSP");
        }

        self.client
            .log_message(MessageType::INFO, "Auto reindex paused.")
            .await;

        Ok(AutoReindexStateResponse {
            success: true,
            paused: true,
            message: "Auto reindex paused".to_string(),
        })
    }

    /// Custom request: bsl/resumeAutoReindex
    pub(crate) async fn handle_resume_auto_reindex(
        &self,
        _params: AutoReindexCommandParams,
    ) -> JsonRpcResult<AutoReindexStateResponse> {
        let mut paused = self.auto_reindex_paused.write().await;
        if *paused {
            *paused = false;
            info!("Auto reindex resumed via LSP");
        }

        self.client
            .log_message(MessageType::INFO, "Auto reindex resumed.")
            .await;

        Ok(AutoReindexStateResponse {
            success: true,
            paused: false,
            message: "Auto reindex resumed".to_string(),
        })
    }

    /// Custom request: bsl/getWorkspaceStats
    pub(crate) async fn handle_get_workspace_stats(
        &self,
    ) -> JsonRpcResult<WorkspaceStatsResponse> {
        let config = self.config.read().await.clone();
        let root = resolve_workspace_root(config);
        let bsl_files = root
            .as_deref()
            .map(count_bsl_files)
            .unwrap_or(0);

        let diagnostics = {
            let counts = self.diagnostics_counts.read().await;
            counts.values().sum()
        };

        Ok(WorkspaceStatsResponse {
            bsl_files,
            diagnostics,
        })
    }
}

fn resolve_workspace_root(config: Option<crate::config::LspConfig>) -> Option<PathBuf> {
    let config_path = config.and_then(|cfg| cfg.configuration_path);
    let path = config_path.map(PathBuf::from)?;
    if path.is_dir() {
        return Some(path);
    }

    if path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
        return path.parent().map(|parent| parent.to_path_buf());
    }

    None
}

fn count_bsl_files(root: &Path) -> usize {
    let mut count = 0usize;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.')
                        || name == "target"
                        || name == "node_modules"
                        || name == ".bsl_cache"
                    {
                        continue;
                    }
                }
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("bsl") {
                count += 1;
            }
        }
    }

    count
}
