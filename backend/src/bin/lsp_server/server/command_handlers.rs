//! Command-specific handlers for BslLanguageServer
//!
//! Contains implementation of helper methods for command handling.

use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{MessageType, Url};
use tracing::{info, warn};

use std::path::{Path, PathBuf};

use crate::commands::{
    handle_incremental_update, handle_parse_configuration, ParseConfigurationParams,
};
use crate::handlers::{find_containing_function_in_dto, CurrentContextResponse};
use crate::types::{
    AutoReindexCommandParams, AutoReindexStateResponse, BuildIndexParams, BuildIndexResponse,
    GetCurrentContextParams, IncrementalUpdateParams, IncrementalUpdateResponse,
    ObservabilityMetricsResponse, WorkspaceStatsResponse,
};
use bsl_backend::system::fs_utils::read_bsl_file;

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

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();

        let ok = if let Some(expected_version) = expected_version {
            self.analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await
        } else {
            let path = match uri.to_file_path() {
                Ok(path) => path.to_string_lossy().to_string(),
                Err(_) => uri.to_string(),
            };

            let file_content = match uri.to_file_path() {
                Ok(path) => match read_bsl_file(&path) {
                    Ok(content) => Some(content),
                    Err(err) => {
                        warn!("Failed to read file for getCurrentContext: {}", err);
                        None
                    }
                },
                Err(_) => None,
            };

            match file_content {
                Some(file_content) => {
                    self.analysis_v2
                        .apply_changes(vec![bsl_analysis_v2::Change::SetFile {
                            file_id,
                            text: std::sync::Arc::from(file_content),
                            version: 0,
                            path: std::sync::Arc::from(path),
                        }]);
                    self.analysis_v2.wait_for_file_version(file_id, 0).await
                }
                None => false,
            }
        };

        if !ok {
            return Ok(CurrentContextResponse::empty());
        }

        let analysis = self.analysis_v2.snapshot().await;
        let ir_program = match analysis.ir(file_id).ok().flatten() {
            Some(ir_program) => ir_program,
            None => return Ok(CurrentContextResponse::empty()),
        };

        let (Some(file_text), Some(line_index)) = (
            analysis.file_text(file_id).ok().flatten(),
            analysis.line_index(file_id).ok().flatten(),
        ) else {
            return Ok(CurrentContextResponse::empty());
        };

        let semantic_tree_dto =
            ir_program.to_dto(true, true, file_text.as_ref(), line_index.as_ref());
        match find_containing_function_in_dto(&semantic_tree_dto, params.line, params.character) {
            Some((name, kind, params_list, return_type)) => Ok(CurrentContextResponse {
                function_name: Some(name),
                function_kind: kind,
                params: Some(params_list),
                return_type,
            }),
            None => Ok(CurrentContextResponse::empty()),
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
                message: "LSP config not available (initializationOptions not received)"
                    .to_string(),
            });
        };

        let platform_docs_root = cfg.platform_docs_archive.as_deref().map(PathBuf::from);

        let Some(config_path) = cfg.configuration_path else {
            return Ok(BuildIndexResponse {
                success: false,
                types_count: 0,
                message: "configurationPath is not configured".to_string(),
            });
        };

        let config_root = PathBuf::from(&config_path);

        let resp = handle_parse_configuration(
            ParseConfigurationParams { config_path },
            self.coordinator.get_domain_bundle(),
            self.client.clone(),
            "bsl-build-index",
            "Building BSL index",
            Some(self.coordinator.clone()),
        )
        .await;

        if resp.success {
            self.deps_update_v2("bsl/buildIndex", platform_docs_root, Some(config_root))
                .await;
            self.sync_v2_globals().await;
        }

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

        let platform_docs_root = {
            let config = self.config.read().await;
            config
                .as_ref()
                .and_then(|cfg| cfg.platform_docs_archive.as_deref())
                .map(PathBuf::from)
        };
        let config_root = PathBuf::from(&params.config_path);

        let resp =
            handle_incremental_update(params, self.coordinator.clone(), self.client.clone()).await;

        if resp.success {
            self.deps_update_v2(
                "bsl/incrementalUpdate",
                platform_docs_root,
                Some(config_root),
            )
            .await;
            self.sync_v2_globals().await;
        }

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
    pub(crate) async fn handle_get_workspace_stats(&self) -> JsonRpcResult<WorkspaceStatsResponse> {
        let config = self.config.read().await.clone();
        let root = resolve_workspace_root(config);
        let bsl_files = root.as_deref().map(count_bsl_files).unwrap_or(0);

        let diagnostics = {
            let counts = self.diagnostics_counts.read().await;
            counts.values().sum()
        };

        Ok(WorkspaceStatsResponse {
            bsl_files,
            diagnostics,
        })
    }

    /// Custom request: bsl/getObservabilityMetrics
    pub(crate) async fn handle_get_observability_metrics(
        &self,
    ) -> JsonRpcResult<ObservabilityMetricsResponse> {
        Ok(ObservabilityMetricsResponse {
            metrics: self.coordinator.observability_metrics(),
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
