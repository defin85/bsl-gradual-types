use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::sync::Arc;

use crate::jobs::JobManager;
use crate::session::SessionManager;
use crate::types::{BslAgentError, BuildInfoResponse, JobStartResponse, UiUrlResponse};

pub mod types;

use types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, ContextExpandParams, ContextPackParams,
    BuildInfoParams, JobCancelParams, JobResultParams, JobStatusParams, JobWaitParams, UiUrlParams,
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

    #[tool(description = "Get local HTTP UI URL for this bsl-agent instance (read-only)")]
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

    #[tool(description = "Get build info for this bsl-agent instance (read-only)")]
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
            .and_then(|value| (!value.trim().is_empty() && value != "unknown").then(|| value))
            .map(str::to_string);
        let git_describe = option_env!("BSL_AGENT_GIT_DESCRIBE")
            .and_then(|value| (!value.trim().is_empty()).then(|| value))
            .map(str::to_string);
        let build_unix_secs = option_env!("BSL_AGENT_BUILD_UNIX_SECS")
            .and_then(|value| value.parse::<u64>().ok());

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

    #[tool(description = "Open a workspace session for semantic queries")]
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

    #[tool(description = "Get workspace session status / progress")]
    async fn workspace_status(
        &self,
        Parameters(params): Parameters<WorkspaceStatusParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.status(&params.session_id).await?;
        Content::json(response)
    }

    #[tool(description = "Close a workspace session and release resources")]
    async fn workspace_close(
        &self,
        Parameters(params): Parameters<WorkspaceCloseParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        self.session_manager.close(&params.session_id).await?;
        Content::json(serde_json::json!({ "ok": true }))
    }

    #[tool(description = "Resume a persisted workspace session by session_id")]
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

    #[tool(description = "List persisted workspace sessions available for resume")]
    async fn workspace_list(
        &self,
        Parameters(_params): Parameters<WorkspaceListParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.list().await?;
        Content::json(response)
    }

    #[tool(description = "Set unsaved documents (overlay) and/or mark documents as hot")]
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

    #[tool(description = "Clear document overlays and/or remove documents from hot set")]
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

    #[tool(description = "Start semantic diagnostics job for project/file/hot scope")]
    async fn bsl_diagnostics_start(
        &self,
        Parameters(params): Parameters<BslDiagnosticsParams>,
    ) -> Result<Content, rmcp::ErrorData> {
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

    #[tool(description = "Start symbol search job by name (deterministic)")]
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

    #[tool(description = "Start type-at-position job")]
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

    #[tool(description = "Start members (completion-like) job at given position")]
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

    #[tool(description = "Start definition resolution job for symbol_id or position")]
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

    #[tool(description = "Start references search job for symbol_id")]
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

    #[tool(description = "Start building an LLM-ready context pack within a hard char budget")]
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

    #[tool(description = "Start expanding a specific item from a previous context_pack")]
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

    #[tool(description = "Get job status / progress")]
    async fn job_status(
        &self,
        Parameters(params): Parameters<JobStatusParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.job_manager.status(&params.job_id).await?;
        Content::json(response)
    }

    #[tool(description = "Wait for job status change or completion (long-poll)")]
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

    #[tool(description = "Get final job result (only for succeeded jobs)")]
    async fn job_result(
        &self,
        Parameters(params): Parameters<JobResultParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let value = self.job_manager.result(&params.job_id).await?;
        Content::json(value)
    }

    #[tool(description = "Cancel a job (best-effort)")]
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
