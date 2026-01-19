use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::sync::Arc;

use crate::session::SessionManager;
use crate::types::BslAgentError;

pub mod types;

use types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, ContextExpandParams, ContextPackParams,
    WorkspaceCloseParams, WorkspaceOpenParams, WorkspaceStatusParams,
};
use types::{WorkspaceDocumentsClearParams, WorkspaceDocumentsSetParams};

#[derive(Clone)]
pub struct BslAgentHandler {
    session_manager: Arc<SessionManager>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BslAgentHandler {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Open a workspace session for semantic queries")]
    async fn workspace_open(
        &self,
        Parameters(params): Parameters<WorkspaceOpenParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.open(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Get workspace session status / progress")]
    async fn workspace_status(
        &self,
        Parameters(params): Parameters<WorkspaceStatusParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.status(&params.session_id).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Close a workspace session and release resources")]
    async fn workspace_close(
        &self,
        Parameters(params): Parameters<WorkspaceCloseParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        self.session_manager.close(&params.session_id).await?;
        Ok(Content::json(serde_json::json!({ "ok": true }))?)
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
        Ok(Content::json(response)?)
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
        Ok(Content::json(response)?)
    }

    #[tool(description = "Get semantic diagnostics for project/file/hot scope")]
    async fn bsl_diagnostics(
        &self,
        Parameters(params): Parameters<BslDiagnosticsParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.bsl_diagnostics(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Search symbols by name (deterministic)")]
    async fn bsl_symbol_search(
        &self,
        Parameters(params): Parameters<BslSymbolSearchParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.bsl_symbol_search(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Get type information at given position")]
    async fn bsl_type_at_position(
        &self,
        Parameters(params): Parameters<BslTypeAtPositionParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.bsl_type_at_position(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "List members (completion-like) at given position")]
    async fn bsl_members(
        &self,
        Parameters(params): Parameters<BslMembersParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.bsl_members(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Resolve definition for symbol_id or position")]
    async fn bsl_definition(
        &self,
        Parameters(params): Parameters<BslDefinitionParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.bsl_definition(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Find references for symbol_id")]
    async fn bsl_references(
        &self,
        Parameters(params): Parameters<BslReferencesParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.bsl_references(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Build an LLM-ready context pack within a hard char budget")]
    async fn context_pack(
        &self,
        Parameters(params): Parameters<ContextPackParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.context_pack(params).await?;
        Ok(Content::json(response)?)
    }

    #[tool(description = "Expand a specific item from a previous context_pack")]
    async fn context_expand(
        &self,
        Parameters(params): Parameters<ContextExpandParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.context_expand(params).await?;
        Ok(Content::json(response)?)
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
