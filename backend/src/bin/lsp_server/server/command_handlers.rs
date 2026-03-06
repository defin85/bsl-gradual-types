//! Command-specific handlers for BslLanguageServer
//!
//! Contains implementation of helper methods for command handling.

use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{MessageType, Url};
use tracing::{info, warn};

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::commands::{
    handle_incremental_update, handle_parse_configuration, ParseConfigurationParams,
};
use crate::handlers::{find_containing_function_in_dto, CurrentContextResponse};
use crate::types::{
    AutoReindexCommandParams, AutoReindexStateResponse, BuildIndexParams, BuildIndexResponse,
    CompletionTimelineRequest, CompletionTimelineResponse, GetCurrentContextParams,
    GetIndexStateParams, GetIndexStateResponse, IncrementalUpdateParams,
    IncrementalUpdateResponse, ObservabilityMetricsResponse, WorkspaceStatsResponse,
};

use super::{BslLanguageServer, FullIndexOperationKind, FullIndexStateKind};

const ATTACHED_MESSAGE: &str = "already running (attached)";

#[derive(Debug, Clone)]
pub(crate) enum BeginFullIndexOutcome {
    Started { operation_id: String },
    AlreadyRunning {
        active_operation: Option<FullIndexOperationKind>,
        operation_id: Option<String>,
    },
}

impl BslLanguageServer {
    pub(crate) async fn handle_get_index_state(
        &self,
        _params: GetIndexStateParams,
    ) -> JsonRpcResult<GetIndexStateResponse> {
        Ok(self.current_index_state().await)
    }

    pub(crate) async fn current_index_state(&self) -> GetIndexStateResponse {
        let state = self.full_index_state.lock().await;
        state.to_response()
    }

    pub(crate) async fn begin_full_index_operation(
        &self,
        kind: FullIndexOperationKind,
        message: impl Into<String>,
    ) -> BeginFullIndexOutcome {
        let message = message.into();
        let operation_id = format!(
            "{}-{}",
            kind.as_str(),
            self.next_full_index_operation_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );

        {
            let mut state = self.full_index_state.lock().await;
            if state.state == FullIndexStateKind::Running {
                return BeginFullIndexOutcome::AlreadyRunning {
                    active_operation: state.active_operation,
                    operation_id: state.operation_id.clone(),
                };
            }

            state.state = FullIndexStateKind::Running;
            state.active_operation = Some(kind);
            state.operation_id = Some(operation_id.clone());
            state.message = Some(message);
            state.updated_at_ms = crate::server::unix_timestamp_ms();
        }

        self.spawn_full_index_watchdog(operation_id.clone(), self.full_index_watchdog_timeout);

        BeginFullIndexOutcome::Started { operation_id }
    }

    pub(crate) fn spawn_full_index_watchdog(&self, operation_id: String, timeout: Duration) {
        let state_holder = self.full_index_state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;

            let mut state = state_holder.lock().await;
            if state.state != FullIndexStateKind::Running {
                return;
            }
            if state.operation_id.as_deref() != Some(operation_id.as_str()) {
                return;
            }

            state.state = FullIndexStateKind::Failed;
            state.active_operation = None;
            state.operation_id = None;
            state.message = Some(format!(
                "full-index timeout after {}ms",
                timeout.as_millis()
            ));
            state.updated_at_ms = crate::server::unix_timestamp_ms();
        });
    }

    pub(crate) async fn finish_full_index_operation_success(
        &self,
        operation_id: &str,
        message: impl Into<String>,
    ) {
        self.finish_full_index_operation(operation_id, FullIndexStateKind::Ready, message)
            .await;
    }

    pub(crate) async fn finish_full_index_operation_failed(
        &self,
        operation_id: &str,
        message: impl Into<String>,
    ) {
        self.finish_full_index_operation(operation_id, FullIndexStateKind::Failed, message)
            .await;
    }

    async fn finish_full_index_operation(
        &self,
        operation_id: &str,
        final_state: FullIndexStateKind,
        message: impl Into<String>,
    ) {
        let mut state = self.full_index_state.lock().await;
        if state.operation_id.as_deref() != Some(operation_id) {
            return;
        }

        state.state = final_state;
        state.active_operation = None;
        state.operation_id = None;
        state.message = Some(message.into());
        state.updated_at_ms = crate::server::unix_timestamp_ms();
    }

    fn attached_build_index_response(
        active_operation: Option<FullIndexOperationKind>,
        operation_id: Option<String>,
    ) -> BuildIndexResponse {
        let active = active_operation.map(|op| op.as_str()).unwrap_or("unknown");
        let suffix = operation_id
            .as_ref()
            .map(|id| format!(" (operation_id={id})"))
            .unwrap_or_default();
        BuildIndexResponse {
            success: true,
            types_count: 0,
            message: format!("{ATTACHED_MESSAGE}: active_operation={active}{suffix}"),
        }
    }

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
        let include_flow_sensitive = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::TypeAtPosition,
                include_flow_sensitive,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(outcome) => {
                warn!(
                    uri = %uri,
                    file_id = file_id.0,
                    outcome = outcome.as_str(),
                    "getCurrentContext: stateful operation not ready"
                );
                return Ok(CurrentContextResponse::empty());
            }
        };

        let analysis = prepared.snapshot.analysis;
        let ir_query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
            &context,
            bsl_runtime::application::ObservabilityStage::IrQuery,
            &analysis,
            Some(self.coordinator.as_ref()),
            |analysis| analysis.ir(file_id),
        );
        let ir_program = match ir_query {
            Ok(Some(ir_program)) => ir_program,
            Ok(None) => return Ok(CurrentContextResponse::empty()),
            Err(cancelled) => {
                warn!(
                    uri = %uri,
                    file_id = file_id.0,
                    error = ?cancelled,
                    "getCurrentContext: IR query cancelled"
                );
                return Ok(CurrentContextResponse::empty());
            }
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
        let operation_id = match self
            .begin_full_index_operation(
                FullIndexOperationKind::BuildIndex,
                "Building BSL index",
            )
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning {
                active_operation,
                operation_id,
            } => {
                return Ok(Self::attached_build_index_response(
                    active_operation,
                    operation_id,
                ));
            }
        };

        let cfg = self.config.read().await.clone();
        let Some(cfg) = cfg else {
            let message = "LSP config not available (initializationOptions not received)".to_string();
            self.finish_full_index_operation_failed(&operation_id, message.clone())
                .await;
            return Ok(BuildIndexResponse {
                success: false,
                types_count: 0,
                message,
            });
        };

        let platform_docs_root = cfg.platform_docs_archive.as_deref().map(PathBuf::from);

        let Some(config_path) = cfg.configuration_path else {
            let message = "configurationPath is not configured".to_string();
            self.finish_full_index_operation_failed(&operation_id, message.clone())
                .await;
            return Ok(BuildIndexResponse {
                success: false,
                types_count: 0,
                message,
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
            self.finish_full_index_operation_success(&operation_id, "Index build completed")
                .await;
        } else {
            self.finish_full_index_operation_failed(
                &operation_id,
                resp.message
                    .clone()
                    .unwrap_or_else(|| "Index build failed".to_string()),
            )
            .await;
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

    pub(crate) async fn handle_get_completion_timeline(
        &self,
        params: CompletionTimelineRequest,
    ) -> JsonRpcResult<CompletionTimelineResponse> {
        let default_limit = super::COMPLETION_TIMELINE_MAX_ENTRIES;
        let limit = params
            .limit
            .unwrap_or(default_limit)
            .clamp(1, super::COMPLETION_TIMELINE_MAX_ENTRIES);
        let request_id_filter = params.request_id.as_deref();

        let traces_guard = self.completion_timeline_traces.lock().await;
        let traces = traces_guard
            .iter()
            .rev()
            .filter(|trace| match request_id_filter {
                Some(request_id) => trace.request_id.as_deref() == Some(request_id),
                None => true,
            })
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();

        Ok(CompletionTimelineResponse {
            version: super::COMPLETION_TIMELINE_VERSION,
            traces: traces.into_iter().rev().collect(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_backend::system::SystemCoordinator;
    use futures::StreamExt;
    use std::sync::{Arc, Mutex as StdMutex};
    use tower::Service;
    use tower::ServiceExt;
    use tower_lsp::lsp_types::{ClientCapabilities, InitializeParams, InitializedParams};
    use tower_lsp::jsonrpc::Request;
    use tower_lsp::LspService;

    fn create_test_server() -> BslLanguageServer {
        let coordinator = Arc::new(SystemCoordinator::new());
        let holder: Arc<StdMutex<Option<BslLanguageServer>>> = Arc::new(StdMutex::new(None));

        let (_service, _socket) = LspService::build({
            let coordinator = coordinator.clone();
            let holder = holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *holder.lock().expect("test server holder lock") = Some(server.clone());
                server
            }
        })
        .finish();

        let server = holder
            .lock()
            .expect("test server holder lock")
            .clone()
            .expect("test server must be captured");
        server
    }

    fn create_custom_service(
    ) -> (
        LspService<BslLanguageServer>,
        tokio::task::JoinHandle<()>,
        BslLanguageServer,
    ) {
        let coordinator = Arc::new(SystemCoordinator::new());
        let holder: Arc<StdMutex<Option<BslLanguageServer>>> = Arc::new(StdMutex::new(None));
        let (service, mut socket) = LspService::build({
            let coordinator = coordinator.clone();
            let holder = holder.clone();
            move |client| {
                let server = BslLanguageServer::new(client, coordinator.clone());
                *holder.lock().expect("test server holder lock") = Some(server.clone());
                server
            }
        })
        .custom_method("bsl/buildIndex", BslLanguageServer::handle_build_index)
        .custom_method(
            "bsl/getIndexState",
            BslLanguageServer::handle_get_index_state,
        )
        .finish();

        let drain_task =
            tokio::spawn(async move { while let Some(_request) = socket.next().await {} });
        let server = holder
            .lock()
            .expect("test server holder lock")
            .clone()
            .expect("test server must be captured");
        (service, drain_task, server)
    }

    async fn initialize_custom_service(service: &mut LspService<BslLanguageServer>) {
        let initialize = Request::build("initialize")
            .id(100)
            .params(
                serde_json::to_value(InitializeParams {
                    capabilities: ClientCapabilities::default(),
                    ..Default::default()
                })
                .expect("InitializeParams"),
            )
            .finish();
        let initialize_response = service
            .ready()
            .await
            .expect("service ready")
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
            .expect("service ready")
            .call(initialized)
            .await
            .expect("initialized notification");
        assert!(
            initialized_response.is_none(),
            "initialized is a notification"
        );
    }

    #[tokio::test]
    async fn build_index_attaches_when_startup_operation_is_running() {
        let server = create_test_server();
        let startup_operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::Startup, "startup")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("startup operation unexpectedly already running")
            }
        };

        let response = server
            .handle_build_index(BuildIndexParams {
                workspace_path: String::new(),
            })
            .await
            .expect("build index response");

        assert!(response.success, "attached response must be successful");
        assert_eq!(response.types_count, 0);
        assert!(
            response.message.contains("already running (attached)"),
            "unexpected message: {}",
            response.message
        );

        let state = server.current_index_state().await;
        assert_eq!(state.state, "running");
        assert_eq!(state.active_operation.as_deref(), Some("startup"));
        assert_eq!(
            state.operation_id.as_deref(),
            Some(startup_operation_id.as_str())
        );

        server
            .finish_full_index_operation_failed(&startup_operation_id, "cleanup")
            .await;
    }

    #[tokio::test]
    async fn get_index_state_reports_ready_after_successful_finish() {
        let server = create_test_server();
        let operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::BuildIndex, "build")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("operation unexpectedly already running")
            }
        };

        server
            .finish_full_index_operation_success(&operation_id, "done")
            .await;

        let state = server
            .handle_get_index_state(GetIndexStateParams::default())
            .await
            .expect("index state response");

        assert_eq!(state.version, 1);
        assert_eq!(state.state, "ready");
        assert!(state.ready);
        assert!(state.active_operation.is_none());
        assert!(state.operation_id.is_none());
        assert!(state.updated_at_ms > 0);
    }

    #[tokio::test]
    async fn watchdog_timeout_transitions_running_operation_to_failed() {
        let server = create_test_server();
        let operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::BuildIndex, "build")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("operation unexpectedly already running")
            }
        };

        server.spawn_full_index_watchdog(operation_id, Duration::from_millis(10));
        tokio::time::sleep(Duration::from_millis(30)).await;

        let state = server.current_index_state().await;
        assert_eq!(state.state, "failed");
        assert!(!state.ready);
        assert!(state.active_operation.is_none());
        assert!(state.operation_id.is_none());
        assert!(
            state
                .message
                .as_deref()
                .is_some_and(|message| message.contains("timeout")),
            "timeout message must be present"
        );
    }

    #[tokio::test]
    async fn get_index_state_rpc_returns_nullable_fields_as_explicit_nulls() {
        let (mut service, drain_task, _server) = create_custom_service();
        initialize_custom_service(&mut service).await;
        let request = Request::build("bsl/getIndexState")
            .id(1)
            .params(serde_json::json!({}))
            .finish();

        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(request)
            .await
            .expect("bsl/getIndexState request")
            .expect("bsl/getIndexState response");

        let value = serde_json::to_value(response).expect("serialize response");
        let result = value.get("result").expect("result field");
        let object = result.as_object().expect("result object");

        for field in ["active_operation", "operation_id", "message"] {
            assert!(
                object.contains_key(field),
                "field `{field}` must be present in response"
            );
            assert!(
                object.get(field).is_some_and(|value| value.is_null()),
                "field `{field}` must be null for idle state"
            );
        }

        assert_eq!(
            object
                .get("version")
                .and_then(|value| value.as_u64())
                .expect("version"),
            1
        );
        assert_eq!(
            object
                .get("state")
                .and_then(|value| value.as_str())
                .expect("state"),
            "idle"
        );
        assert!(
            !object
                .get("ready")
                .and_then(|value| value.as_bool())
                .expect("ready")
        );

        drain_task.abort();
    }

    #[tokio::test]
    async fn build_index_rpc_attaches_to_running_startup_operation() {
        let (mut service, drain_task, server) = create_custom_service();
        initialize_custom_service(&mut service).await;
        let startup_operation_id = match server
            .begin_full_index_operation(FullIndexOperationKind::Startup, "startup")
            .await
        {
            BeginFullIndexOutcome::Started { operation_id } => operation_id,
            BeginFullIndexOutcome::AlreadyRunning { .. } => {
                panic!("startup operation unexpectedly already running")
            }
        };

        let request = Request::build("bsl/buildIndex")
            .id(2)
            .params(serde_json::json!({ "workspace_path": "/tmp/workspace" }))
            .finish();
        let response = service
            .ready()
            .await
            .expect("service ready")
            .call(request)
            .await
            .expect("bsl/buildIndex request")
            .expect("bsl/buildIndex response");
        let value = serde_json::to_value(response).expect("serialize response");
        let result = value.get("result").expect("result field");
        let object = result.as_object().expect("result object");
        assert!(
            object
                .get("success")
                .and_then(|value| value.as_bool())
                .expect("success")
        );
        let message = object
            .get("message")
            .and_then(|value| value.as_str())
            .expect("message");
        assert!(
            message.contains("already running (attached)"),
            "unexpected attached message: {}",
            message
        );

        let state = server.current_index_state().await;
        assert_eq!(state.state, "running");
        assert_eq!(state.active_operation.as_deref(), Some("startup"));
        assert_eq!(
            state.operation_id.as_deref(),
            Some(startup_operation_id.as_str())
        );

        server
            .finish_full_index_operation_failed(&startup_operation_id, "cleanup")
            .await;
        drain_task.abort();
    }

    fn sample_stage(
        name: &str,
        status: &str,
        started_offset_ms: u64,
        duration_ms: u64,
    ) -> crate::types::CompletionTimelineStageTrace {
        crate::types::CompletionTimelineStageTrace {
            name: name.to_string(),
            status: status.to_string(),
            started_offset_ms,
            duration_ms,
        }
    }

    fn sample_trace(
        trace_id: &str,
        request_id: Option<&str>,
        outcome: &str,
        total_duration_ms: u64,
        stages: Vec<crate::types::CompletionTimelineStageTrace>,
    ) -> crate::types::CompletionTimelineTrace {
        crate::types::CompletionTimelineTrace {
            trace_id: trace_id.to_string(),
            request_id: request_id.map(ToString::to_string),
            uri: "file:///timeline.bsl".to_string(),
            trigger_mode: "trigger_character".to_string(),
            outcome: outcome.to_string(),
            started_at_ms: 1_700_000_000_000,
            total_duration_ms,
            dominant_stage: stages
                .iter()
                .max_by_key(|stage| stage.duration_ms)
                .map(|stage| stage.name.clone()),
            stages,
        }
    }

    #[tokio::test]
    async fn completion_timeline_retention_evicts_oldest_first() {
        let server = create_test_server();
        for idx in 0..205_u64 {
            let trace = sample_trace(
                &format!("trace-{idx}"),
                Some(&format!("req-{idx}")),
                "ok_non_empty",
                10 + idx,
                vec![sample_stage("prepare_stateful", "completed", 0, 10 + idx)],
            );
            server.record_completion_timeline_trace(trace).await;
        }

        let response = server
            .handle_get_completion_timeline(crate::types::CompletionTimelineRequest::default())
            .await
            .expect("timeline response");
        assert_eq!(response.version, 1);
        assert_eq!(response.traces.len(), 200);
        assert_eq!(response.traces.first().map(|trace| trace.trace_id.as_str()), Some("trace-5"));
        assert_eq!(
            response.traces.last().map(|trace| trace.trace_id.as_str()),
            Some("trace-204")
        );
    }

    #[tokio::test]
    async fn completion_timeline_can_filter_by_request_id() {
        let server = create_test_server();
        server
            .record_completion_timeline_trace(sample_trace(
                "trace-a",
                Some("req-a"),
                "ok_non_empty",
                30,
                vec![sample_stage("query_bundle", "completed", 0, 30)],
            ))
            .await;
        server
            .record_completion_timeline_trace(sample_trace(
                "trace-b",
                Some("req-b"),
                "cancelled",
                5,
                vec![sample_stage("query_bundle", "cancelled", 0, 5)],
            ))
            .await;

        let response = server
            .handle_get_completion_timeline(crate::types::CompletionTimelineRequest {
                limit: Some(10),
                request_id: Some("req-b".to_string()),
            })
            .await
            .expect("timeline response");

        assert_eq!(response.traces.len(), 1);
        let trace = &response.traces[0];
        assert_eq!(trace.trace_id, "trace-b");
        assert_eq!(trace.request_id.as_deref(), Some("req-b"));
        assert_eq!(trace.outcome, "cancelled");
        assert_eq!(trace.stages.len(), 1);
        assert_eq!(trace.stages[0].status, "cancelled");
    }
}
