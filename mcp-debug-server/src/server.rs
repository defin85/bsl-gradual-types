use anyhow::Result;
use rmcp::{
    ErrorData as McpError,
    RoleServer, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters,
    },
    model::{
        AnnotateAble, CallToolResult, Content, Implementation, ListResourcesResult, ProtocolVersion,
        RawResource, ReadResourceRequestParam, ReadResourceResult, ResourceContents, ServerCapabilities,
    },
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::session::{
    Breakpoint, CreateSessionRequest, LaunchParameters, SessionId, SessionManager,
};

#[derive(Clone)]
pub struct DebugServer {
    session_manager: SessionManager,
    tool_router: ToolRouter<Self>,
}

impl DebugServer {
    pub fn new(session_manager: SessionManager) -> Self {
        Self {
            session_manager,
            tool_router: Self::tool_router(),
        }
    }

    fn text_result(message: impl Into<String>) -> CallToolResult {
        CallToolResult::success(vec![Content::text(message)])
    }

    fn err(err: impl ToString) -> McpError {
        McpError::internal_error(err.to_string(), None)
    }
}

#[tool_router]
impl DebugServer {
    #[tool(description = "Создать новую DAP сессию (по умолчанию через CodeLLDB)")]
    async fn create_debug_session(
        &self,
        Parameters(params): Parameters<CreateSessionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = self
            .session_manager
            .create_session(params)
            .await
            .map_err(Self::err)?;
        Ok(Self::text_result(format!(
            "Создана debug-сессия: {session_id}"
        )))
    }

    #[tool(description = "Установить breakpoints для файла")]
    async fn set_breakpoints(
        &self,
        Parameters(params): Parameters<BreakpointRequest>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = params.session_id.clone();
        let file = params.file.clone();
        let lines = params.lines.clone();

        let breakpoints = self
            .session_manager
            .with_session(&session_id, move |session| {
                let file = file.clone();
                let lines = lines.clone();
                async move { session.set_breakpoints(&file, lines).await }
            })
            .await
            .map_err(Self::err)?;

        Ok(Self::text_result(format_breakpoints(&breakpoints)))
    }

    #[tool(description = "Запустить отладку (launch request)")]
    async fn launch_target(
        &self,
        Parameters(params): Parameters<LaunchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let launch_params = params.launch.clone();
        self.session_manager
            .with_session(&params.session_id, move |session| {
                let launch_params = launch_params.clone();
                async move { session.launch(launch_params).await }
            })
            .await
            .map_err(Self::err)?;
        Ok(Self::text_result("Запущен debug target"))
    }

    #[tool(description = "Продолжить выполнение (continue)")]
    async fn debug_continue(
        &self,
        Parameters(params): Parameters<SessionCommandArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.session_manager
            .with_session(&params.session_id, |session| async move {
                session.continue_execution().await
            })
            .await
            .map_err(Self::err)?;
        Ok(Self::text_result("Продолжаю выполнение"))
    }

    #[tool(description = "Шаг внутрь (stepIn)")]
    async fn debug_step_in(
        &self,
        Parameters(params): Parameters<SessionCommandArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.session_manager
            .with_session(&params.session_id, |session| async move {
                session.step_in().await
            })
            .await
            .map_err(Self::err)?;
        Ok(Self::text_result("Step in выполнен"))
    }

    #[tool(description = "Шаг через (next)")]
    async fn debug_step_over(
        &self,
        Parameters(params): Parameters<SessionCommandArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.session_manager
            .with_session(&params.session_id, |session| async move {
                session.step_over().await
            })
            .await
            .map_err(Self::err)?;
        Ok(Self::text_result("Step over выполнен"))
    }

    #[tool(description = "Шаг из функции (stepOut)")]
    async fn debug_step_out(
        &self,
        Parameters(params): Parameters<SessionCommandArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.session_manager
            .with_session(&params.session_id, |session| async move {
                session.step_out().await
            })
            .await
            .map_err(Self::err)?;
        Ok(Self::text_result("Step out выполнен"))
    }

    #[tool(description = "Показать stack trace активного потока")]
    async fn debug_stack(
        &self,
        Parameters(params): Parameters<StackTraceArgs>,
    ) -> Result<CallToolResult, McpError> {
        let depth = params.depth;
        let body = self
            .session_manager
            .with_session(&params.session_id, move |session| {
                async move { session.stack_trace(depth).await }
            })
            .await
            .map_err(Self::err)?;

        Ok(Self::text_result(format_stack_trace(&body)))
    }

    #[tool(description = "Оценить выражение в текущем frame")]
    async fn debug_evaluate(
        &self,
        Parameters(params): Parameters<EvaluateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let expr = params.expression.clone();
        let ctx = params.context.clone();
        let body = self
            .session_manager
            .with_session(&params.session_id, move |session| {
                async move { session.evaluate(&expr, ctx.as_deref()).await }
            })
            .await
            .map_err(Self::err)?;

        let result = body
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("<нет значения>");
        let ty = body
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        Ok(Self::text_result(format!("{} = {} ({ty})", params.expression, result)))
    }

    #[tool(description = "Ждать события остановки (breakpoint, исключение)")]
    async fn wait_for_stop(
        &self,
        Parameters(params): Parameters<SessionCommandArgs>,
    ) -> Result<CallToolResult, McpError> {
        let stop = self
            .session_manager
            .with_session(&params.session_id, |session| async move {
                session.wait_for_stop().await
            })
            .await
            .map_err(Self::err)?;

        let mut parts = vec![format!("Reason: {}", stop.reason)];
        if let Some(thread_id) = stop.thread_id {
            parts.push(format!("thread #{thread_id}"));
        }
        if let Some(desc) = stop.description {
            parts.push(desc);
        }
        Ok(Self::text_result(parts.join(" | ")))
    }
}

#[tool_handler]
impl ServerHandler for DebugServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        let mut implementation = Implementation::from_build_env();
        implementation.name = "mcp-debug-server".to_string();

        rmcp::model::ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: implementation,
            instructions: Some(
                "Используй инструменты create_debug_session → set_breakpoints → launch_target → wait_for_stop/step/continue → debug_stack/debug_evaluate."
                    .to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = vec![
            RawResource::new("debug://sessions", "Активные сессии")
                .with_description("Перечень доступных debug-сессий")
                .with_mime("application/json")
                .no_annotation(),
            RawResource::new("debug://instructions", "Инструкции")
                .with_description("Краткое описание команд")
                .with_mime("text/plain")
                .no_annotation(),
        ];

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "debug://sessions" => {
                let sessions = self.session_manager.list_sessions().await;
                let json = serde_json::to_string_pretty(&sessions).unwrap_or_else(|_| "[]".into());
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(json, uri)],
                })
            }
            "debug://instructions" => {
                let text = "Алгоритм: create_debug_session -> set_breakpoints -> launch_target -> wait_for_stop -> stack/evaluate -> continue.";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(text, uri)],
                })
            }
            _ => Err(McpError::resource_not_found(
                format!("Ресурс {uri} не найден"),
                None,
            )),
        }
    }
}

fn format_breakpoints(breakpoints: &[Breakpoint]) -> String {
    if breakpoints.is_empty() {
        return "Адаптер не вернул breakpoints".to_string();
    }
    let mut lines = vec!["Breakpoints:".to_string()];
    for bp in breakpoints {
        let status = if bp.verified { "✓" } else { "✗" };
        let mut entry = format!("{status} line {}", bp.line);
        if let Some(msg) = &bp.message {
            entry.push_str(&format!(" — {}", msg));
        }
        lines.push(entry);
    }
    lines.join("\n")
}

fn format_stack_trace(body: &serde_json::Value) -> String {
    let frames = body
        .get("stackFrames")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if frames.is_empty() {
        return "Stack trace пуст".to_string();
    }

    let mut lines = vec!["Stack trace:".to_string()];
    for (idx, frame) in frames.iter().enumerate() {
        let name = frame
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        let file = frame
            .get("source")
            .and_then(|s| s.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let line = frame
            .get("line")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        lines.push(format!("#{} {name} ({file}:{line})", idx));
    }
    lines.join("\n")
}

trait ResourceExt {
    fn with_description(self, description: impl Into<String>) -> Self;
    fn with_mime(self, mime: impl Into<String>) -> Self;
}

impl ResourceExt for RawResource {
    fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn with_mime(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BreakpointRequest {
    pub session_id: SessionId,
    pub file: String,
    pub lines: Vec<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionCommandArgs {
    pub session_id: SessionId,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StackTraceArgs {
    pub session_id: SessionId,
    #[serde(default)]
    pub depth: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EvaluateArgs {
    pub session_id: SessionId,
    pub expression: String,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LaunchRequest {
    pub session_id: SessionId,
    #[serde(default)]
    pub launch: LaunchParameters,
}
