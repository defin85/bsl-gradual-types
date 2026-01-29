use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::sync::Arc;

use crate::jobs::JobManager;
use crate::session::SessionManager;
use crate::types::{
    BslAgentError, BuildInfoResponse, JobStartResponse, McpHelpResponse, UiUrlResponse,
};

pub mod types;

use types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, BslTypeGetParams, BslTypesListParams,
    BslTypesSearchParams, BuildInfoParams, ContextExpandParams, ContextPackParams, JobCancelParams,
    JobResultParams, JobStatusParams, JobWaitParams, McpHelpParams, UiUrlParams,
    WorkspaceCloseParams, WorkspaceListParams, WorkspaceOpenParams, WorkspaceResumeParams,
    WorkspaceStatusParams,
};
use types::{WorkspaceDocumentsClearParams, WorkspaceDocumentsSetParams};

#[derive(Clone)]
pub struct BslAgentHandler {
    session_manager: Arc<SessionManager>,
    job_manager: Arc<JobManager>,
    ui_url: Option<String>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BslAgentHandler {
    pub fn new() -> Self {
        Self::with_state(Arc::new(SessionManager::new()), Arc::new(JobManager::new()))
    }

    pub fn with_state(session_manager: Arc<SessionManager>, job_manager: Arc<JobManager>) -> Self {
        Self {
            session_manager,
            job_manager,
            ui_url: None,
            tool_router: Self::tool_router(),
        }
    }

    pub fn session_manager(&self) -> Arc<SessionManager> {
        Arc::clone(&self.session_manager)
    }

    pub fn job_manager(&self) -> Arc<JobManager> {
        Arc::clone(&self.job_manager)
    }

    pub fn set_ui_url(&mut self, ui_url: String) {
        self.ui_url = Some(ui_url);
    }

    #[tool(description = "On-demand help: quickstart + per-tool examples (read-only)")]
    async fn mcp_help(
        &self,
        Parameters(params): Parameters<McpHelpParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let tool_name = params
            .tool_name
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        let mut quickstart = vec![
            "workspace_open(roots[], platform_docs_archive?, configuration_path?, platform_version?, mode?)".to_string(),
            "workspace_status(session_id) poll until ready=true".to_string(),
            "workspace_documents_set(session_id, files[], mark_hot=true)".to_string(),
            "bsl_diagnostics_start(...) / bsl_symbol_search_start(...) / context_pack_start(...)".to_string(),
            "bsl_types_list_start(...) / bsl_types_search_start(...) / bsl_type_get_start(...)".to_string(),
            "job_wait(job_id, timeout_ms) until state=succeeded".to_string(),
            "job_result(job_id)".to_string(),
        ];

        let mut notes = vec![
            "Multi-root: prefer absolute paths; server resolves via deterministic longest-prefix match against roots[].".to_string(),
            "If configuration_path is set and platform_version is omitted, bsl-agent tries to infer platform_version from config dump; otherwise INVALID_PARAMS.".to_string(),
            "Async: all semantic tools are *_start and return job_id; fetch result via job_result.".to_string(),
        ];

        let mut examples: Vec<serde_json::Value> = Vec::new();
        if let Some(name) = tool_name.as_deref() {
            match name {
                "workspace_open" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_open",
                        "arguments": { "roots": ["/abs/path/to/workspace"], "mode": "default" }
                    }));
                    examples.push(serde_json::json!({
                        "name": "workspace_open",
                        "arguments": { "roots": ["/ws/config", "/ws/ext1"], "configuration_path": "/ws/config", "platform_version": "8.3.25" }
                    }));
                    notes.push("Single-session: calling workspace_open again with different params requires workspace_close first.".to_string());
                }
                "workspace_documents_set" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_documents_set",
                        "arguments": {
                            "session_id": "<session_id>",
                            "files": ["/ws/ext1/src/CommonModules/Foo/Module.bsl"],
                            "mark_hot": true
                        }
                    }));
                    examples.push(serde_json::json!({
                        "name": "workspace_documents_set",
                        "arguments": {
                            "session_id": "<session_id>",
                            "files": [
                                { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" }, "text": "Procedure P()\nEndProcedure\n", "version": 1 }
                            ],
                            "mark_hot": true
                        }
                    }));
                    notes.push("When text is provided, version is required.".to_string());
                }
                "workspace_documents_clear" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_documents_clear",
                        "arguments": {
                            "session_id": "<session_id>",
                            "documents": [{ "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" }],
                            "clear_hot": true
                        }
                    }));
                }
                "bsl_diagnostics_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_diagnostics_start",
                        "arguments": { "session_id": "<session_id>", "scope": "hot", "limit": 200 }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_diagnostics_start",
                        "arguments": { "session_id": "<session_id>", "scope": { "kind": "project" }, "limit": 200 }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_diagnostics_start",
                        "arguments": {
                            "session_id": "<session_id>",
                            "scope": { "kind": "file", "document": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                            "limit": 200
                        }
                    }));
                    notes.push("scope string supports only: project|hot. For a single file use tagged: {kind:\"file\",document:...}.".to_string());
                }
                "job_wait" => {
                    examples.push(serde_json::json!({
                        "name": "job_wait",
                        "arguments": { "job_id": "<job_id>", "timeout_ms": 5000 }
                    }));
                }
                "bsl_types_list_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_types_list_start",
                        "arguments": { "session_id": "<session_id>", "page": 1, "limit": 50, "view": "names_only" }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_types_list_start",
                        "arguments": { "session_id": "<session_id>", "page": 1, "limit": 50, "source": "configuration", "view": "summary" }
                    }));
                }
                "bsl_types_search_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_types_search_start",
                        "arguments": { "session_id": "<session_id>", "query": "Документ", "limit": 200, "view": "summary" }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_types_search_start",
                        "arguments": { "session_id": "<session_id>", "query": "Документы.", "limit": 200, "source": "configuration", "view": "names_only" }
                    }));
                }
                "bsl_type_get_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_type_get_start",
                        "arguments": { "session_id": "<session_id>", "type_name": "Документы.ЗаказНаряд", "source": "configuration", "include_methods": false }
                    }));
                    notes.push("bsl_type_get_start returns a TypeDto with properties[] and tabularSections[] for configuration objects.".to_string());
                }
                other => {
                    return Err(rmcp::ErrorData::invalid_params(
                        format!("unknown tool_name: {other}"),
                        None,
                    ));
                }
            }
        } else {
            quickstart.insert(
                0,
                "mcp_help(tool_name?) for examples (read-only)".to_string(),
            );
            notes.push("Pass tool_name to get examples: workspace_open, workspace_documents_set, workspace_documents_clear, bsl_diagnostics_start, bsl_types_list_start, bsl_types_search_start, bsl_type_get_start, job_wait.".to_string());
        }

        Content::json(McpHelpResponse {
            summary: "bsl-agent MCP help (read-only)".to_string(),
            quickstart,
            tool_name,
            notes,
            examples,
        })
    }

    #[tool(description = "Get local HTTP UI URL (read-only). For usage examples see mcp_help.")]
    async fn ui_url(
        &self,
        Parameters(_params): Parameters<UiUrlParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let url = self.ui_url.clone().or_else(|| {
            crate::ui_discovery::read_registry_record_by_pid(std::process::id())
                .map(|record| record.ui_url)
        });
        Content::json(UiUrlResponse {
            enabled: url.is_some(),
            ui_url: url,
        })
    }

    #[tool(description = "Get build info (read-only). For usage examples see mcp_help.")]
    async fn build_info(
        &self,
        Parameters(_params): Parameters<BuildInfoParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let package = env!("CARGO_PKG_NAME").to_string();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let profile = option_env!("BSL_AGENT_PROFILE")
            .unwrap_or("unknown")
            .to_string();
        let target = option_env!("BSL_AGENT_TARGET")
            .unwrap_or("unknown")
            .to_string();

        let git_sha = option_env!("BSL_AGENT_GIT_SHA")
            .and_then(|value| (!value.trim().is_empty() && value != "unknown").then_some(value))
            .map(str::to_string);
        let git_describe = option_env!("BSL_AGENT_GIT_DESCRIBE")
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
            .map(str::to_string);
        let build_unix_secs =
            option_env!("BSL_AGENT_BUILD_UNIX_SECS").and_then(|value| value.parse::<u64>().ok());

        Content::json(BuildInfoResponse {
            package,
            version,
            profile,
            target,
            git_sha,
            git_describe,
            build_unix_secs,
            pid: std::process::id(),
        })
    }

    #[tool(
        description = "Open workspace session (single-session). Supports multi-root; config may infer platform_version. See mcp_help."
    )]
    async fn workspace_open(
        &self,
        Parameters(params): Parameters<WorkspaceOpenParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .open(params, Arc::clone(&self.job_manager))
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Get workspace status/progress. ready=true means startup finished. See mcp_help."
    )]
    async fn workspace_status(
        &self,
        Parameters(params): Parameters<WorkspaceStatusParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.status(&params.session_id).await?;
        Content::json(response)
    }

    #[tool(
        description = "Close workspace session (releases resources). Required to switch params. See mcp_help."
    )]
    async fn workspace_close(
        &self,
        Parameters(params): Parameters<WorkspaceCloseParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        self.session_manager.close(&params.session_id).await?;
        Content::json(serde_json::json!({ "ok": true }))
    }

    #[tool(
        description = "Resume a persisted session by session_id (single-session rules apply). See mcp_help."
    )]
    async fn workspace_resume(
        &self,
        Parameters(params): Parameters<WorkspaceResumeParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .resume(&params.session_id, Arc::clone(&self.job_manager))
            .await?;
        Content::json(response)
    }

    #[tool(description = "List persisted sessions available for resume. See mcp_help.")]
    async fn workspace_list(
        &self,
        Parameters(_params): Parameters<WorkspaceListParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.list().await?;
        Content::json(response)
    }

    #[tool(
        description = "Set overlays and/or mark docs hot. files accept absolute paths; version required with text. See mcp_help."
    )]
    async fn workspace_documents_set(
        &self,
        Parameters(params): Parameters<WorkspaceDocumentsSetParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .documents_set(&params.session_id, &params.files, params.mark_hot)
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Clear overlays and/or remove docs from hot set. documents accept absolute paths. See mcp_help."
    )]
    async fn workspace_documents_clear(
        &self,
        Parameters(params): Parameters<WorkspaceDocumentsClearParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .documents_clear(&params.session_id, &params.documents, params.clear_hot)
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Start diagnostics job. scope: \"project\"|\"hot\" or {kind:\"file\",document:...}. Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_diagnostics_start(
        &self,
        Parameters(params): Parameters<BslDiagnosticsParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        if let types::WorkspaceScope::Simple(value) = &params.scope {
            let trimmed = value.trim();
            if trimmed.eq_ignore_ascii_case("project") || trimmed.eq_ignore_ascii_case("hot") {
                // ok
            } else if trimmed.eq_ignore_ascii_case("file") {
                return Err(rmcp::ErrorData::invalid_params(
                    "scope=\"file\" is not supported as a string; use tagged file scope: {\"kind\":\"file\",\"document\":...}",
                    None,
                ));
            } else {
                return Err(rmcp::ErrorData::invalid_params(
                    format!("unknown scope: {trimmed}"),
                    None,
                ));
            }
        }

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_diagnostics", move |ctx| async move {
                ctx.set_progress("bsl_diagnostics/running", 0).await;
                let response = session_manager
                    .bsl_diagnostics(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start symbol search job (deterministic). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_symbol_search_start(
        &self,
        Parameters(params): Parameters<BslSymbolSearchParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_symbol_search", move |ctx| async move {
                ctx.set_progress("bsl_symbol_search/running", 0).await;
                let response = session_manager
                    .bsl_symbol_search(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start types list job (platform/config). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_types_list_start(
        &self,
        Parameters(params): Parameters<BslTypesListParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let status = self.session_manager.status(&params.session_id).await?;
        if !status.ready {
            return Err(rmcp::ErrorData::invalid_params(
                "workspace not ready (startup in progress)",
                None,
            ));
        }
        if params.page < 1 {
            return Err(rmcp::ErrorData::invalid_params("page must be >= 1", None));
        }
        if params.limit < 1 || params.limit > 1000 {
            return Err(rmcp::ErrorData::invalid_params(
                "limit must be in 1..=1000",
                None,
            ));
        }
        if params
            .certainty_level
            .is_some_and(|certainty_level| certainty_level > 100)
        {
            return Err(rmcp::ErrorData::invalid_params(
                "certainty_level must be in 0..=100",
                None,
            ));
        }

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_types_list", move |ctx| async move {
                ctx.set_progress("bsl_types_list/running", 0).await;
                let response = session_manager
                    .bsl_types_list(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                Ok(response)
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start types search job (deterministic). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_types_search_start(
        &self,
        Parameters(params): Parameters<BslTypesSearchParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let status = self.session_manager.status(&params.session_id).await?;
        if !status.ready {
            return Err(rmcp::ErrorData::invalid_params(
                "workspace not ready (startup in progress)",
                None,
            ));
        }
        let query = params.query.trim();
        if query.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "query must be non-empty",
                None,
            ));
        }
        if params.limit < 1 || params.limit > 1000 {
            return Err(rmcp::ErrorData::invalid_params(
                "limit must be in 1..=1000",
                None,
            ));
        }

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_types_search", move |ctx| async move {
                ctx.set_progress("bsl_types_search/running", 0).await;
                let response = session_manager
                    .bsl_types_search(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                Ok(response)
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start type details job by exact name. Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_type_get_start(
        &self,
        Parameters(params): Parameters<BslTypeGetParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let status = self.session_manager.status(&params.session_id).await?;
        if !status.ready {
            return Err(rmcp::ErrorData::invalid_params(
                "workspace not ready (startup in progress)",
                None,
            ));
        }
        let type_name = params.type_name.trim();
        if type_name.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "type_name must be non-empty",
                None,
            ));
        }

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_type_get", move |ctx| async move {
                ctx.set_progress("bsl_type_get/running", 0).await;
                let response = session_manager
                    .bsl_type_get(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                Ok(response)
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start type-at-position job. position is 0-based (LSP); file accepts absolute path. See mcp_help."
    )]
    async fn bsl_type_at_position_start(
        &self,
        Parameters(params): Parameters<BslTypeAtPositionParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_type_at_position", move |ctx| async move {
                ctx.set_progress("bsl_type_at_position/running", 0).await;
                let response = session_manager
                    .bsl_type_at_position(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start members job at position (completion-like). position is 0-based (LSP). See mcp_help."
    )]
    async fn bsl_members_start(
        &self,
        Parameters(params): Parameters<BslMembersParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_members", move |ctx| async move {
                ctx.set_progress("bsl_members/running", 0).await;
                let response = session_manager
                    .bsl_members(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start definition job (symbol_id or file+position). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_definition_start(
        &self,
        Parameters(params): Parameters<BslDefinitionParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_definition", move |ctx| async move {
                ctx.set_progress("bsl_definition/running", 0).await;
                let response = session_manager
                    .bsl_definition(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start references job for symbol_id. Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_references_start(
        &self,
        Parameters(params): Parameters<BslReferencesParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_references", move |ctx| async move {
                ctx.set_progress("bsl_references/running", 0).await;
                let response = session_manager
                    .bsl_references(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start context_pack job (hard budget). Use job_wait/job_result. See mcp_help."
    )]
    async fn context_pack_start(
        &self,
        Parameters(params): Parameters<ContextPackParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("context_pack", move |ctx| async move {
                ctx.set_progress("context_pack/running", 0).await;
                let response = session_manager
                    .context_pack(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start context_expand job for a context_pack item. Use job_wait/job_result. See mcp_help."
    )]
    async fn context_expand_start(
        &self,
        Parameters(params): Parameters<ContextExpandParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("context_expand", move |ctx| async move {
                ctx.set_progress("context_expand/running", 0).await;
                let response = session_manager
                    .context_expand(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Get job status/progress. progress=100 only for terminal state. See mcp_help."
    )]
    async fn job_status(
        &self,
        Parameters(params): Parameters<JobStatusParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.job_manager.status(&params.job_id).await?;
        Content::json(response)
    }

    #[tool(
        description = "Long-poll job status up to timeout_ms (no result). Use job_result after succeeded. See mcp_help."
    )]
    async fn job_wait(
        &self,
        Parameters(params): Parameters<JobWaitParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .job_manager
            .wait(&params.job_id, params.timeout_ms)
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Get job result (succeeded only). Use job_status/job_wait first. See mcp_help."
    )]
    async fn job_result(
        &self,
        Parameters(params): Parameters<JobResultParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let value = self.job_manager.result(&params.job_id).await?;
        Content::json(value)
    }

    #[tool(description = "Cancel a job (best-effort). See mcp_help.")]
    async fn job_cancel(
        &self,
        Parameters(params): Parameters<JobCancelParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.job_manager.cancel(&params.job_id).await?;
        Content::json(response)
    }
}

#[tool_handler]
impl ServerHandler for BslAgentHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "bsl-agent: local MCP server (stdio) providing semantic context for BSL projects"
                    .to_string(),
            ),
        }
    }
}

impl Default for BslAgentHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl From<BslAgentError> for rmcp::ErrorData {
    fn from(err: BslAgentError) -> Self {
        err.into_rmcp()
    }
}
