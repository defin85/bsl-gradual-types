use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use bsl_runtime::application::{
    cpu_work_class_for_operation, diagnostics_execution_plan, DiagnosticsDisposition,
    DiagnosticsProfile, DiagnosticsTrigger, SemanticOperation,
};

use crate::jobs::JobManager;
use crate::session::SessionManager;
use crate::types::{
    BslAgentError, BuildInfoResponse, JobStartResponse, JobStateDto, UiUrlResponse,
};

mod help;
mod tool_router_methods;
pub mod types;

use types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, BslTypeGetParams, BslTypesListParams,
    BslTypesSearchParams, BuildInfoParams, ContextExpandParams, ContextPackParams, JobCancelParams,
    JobResultParams, JobStatusParams, JobWaitParams, McpHelpParams, UiUrlParams,
    WorkspaceCloseParams, WorkspaceGetObservabilityMetricsParams, WorkspaceGetSettingsParams,
    WorkspaceListParams, WorkspaceOpenParams, WorkspaceResumeParams, WorkspaceStatusParams,
    WorkspaceUpdateSettingsParams,
};
use types::{WorkspaceDocumentsClearParams, WorkspaceDocumentsSetParams};

#[derive(Debug, Clone)]
struct TrackedBatchJob {
    job_id: String,
    analysis_revision: u64,
    profile: DiagnosticsProfile,
}

#[derive(Clone)]
pub struct BslAgentHandler {
    session_manager: Arc<SessionManager>,
    job_manager: Arc<JobManager>,
    batch_jobs_by_session: Arc<RwLock<HashMap<String, Vec<TrackedBatchJob>>>>,
    ui_url: Option<String>,
    tool_router: ToolRouter<Self>,
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

#[cfg(test)]
mod tests;
