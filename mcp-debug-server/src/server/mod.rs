//! MCP Debug Server
//!
//! Main server module with tool routing and handler delegation.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::sync::Arc;

use crate::session::SessionManager;

// Sub-modules
pub mod handlers;
pub mod resources;
pub mod tools;
pub mod types;

// Re-export types for backward compatibility
pub use types::{
    BacktraceParams, ConditionalBreakpointParams, CreateSessionParams, EvalParams, LaunchParams,
    PollEventsParams, SetBreakpointParams, StepParams, TerminateParams,
};

/// Главная структура MCP Debug Server
#[derive(Clone)]
pub struct DebugServerHandler {
    /// Менеджер debug сессий (thread-safe)
    session_manager: Arc<SessionManager>,
    /// Router для обработки tool вызовов
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl DebugServerHandler {
    /// Создать новый server handler
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
            tool_router: Self::tool_router(),
        }
    }

    /// Tool 1: Создать новую debug сессию
    #[tool(description = "Create a new debug session for a binary program")]
    async fn debug_create_session(
        &self,
        Parameters(params): Parameters<CreateSessionParams>,
    ) -> String {
        handlers::session_handlers::create_session(&self.session_manager, params).await
    }

    /// Tool 2: Установить breakpoint
    #[tool(description = "Set a breakpoint at file:line in the debug session")]
    async fn debug_set_breakpoint(
        &self,
        Parameters(params): Parameters<SetBreakpointParams>,
    ) -> String {
        handlers::debug_handlers::set_breakpoint(&self.session_manager, params).await
    }

    /// Tool 3: Запустить программу под отладкой
    #[tool(description = "Launch the program in the debug session")]
    async fn debug_launch(&self, Parameters(params): Parameters<LaunchParams>) -> String {
        handlers::debug_handlers::launch(&self.session_manager, params).await
    }

    /// Tool 4: Step over (next line)
    #[tool(description = "Step over to the next line (step over functions)")]
    async fn debug_next(&self, Parameters(params): Parameters<StepParams>) -> String {
        handlers::step_handlers::next(&self.session_manager, params).await
    }

    /// Tool 5: Step into function
    #[tool(description = "Step into the current function call")]
    async fn debug_step_in(&self, Parameters(params): Parameters<StepParams>) -> String {
        handlers::step_handlers::step_in(&self.session_manager, params).await
    }

    /// Tool 6: Continue execution until next breakpoint
    #[tool(description = "Continue execution until hitting a breakpoint")]
    async fn debug_continue(&self, Parameters(params): Parameters<StepParams>) -> String {
        handlers::step_handlers::continue_execution(&self.session_manager, params).await
    }

    /// Tool 7: List active debug sessions
    #[tool(description = "List all active debug sessions")]
    async fn debug_list_sessions(&self) -> String {
        handlers::session_handlers::list_sessions(&self.session_manager).await
    }

    /// Tool 8: Вычислить expression в текущем фрейме
    #[tool(description = "Evaluate an expression in the current stack frame. Note: Variables may be unavailable after their lifetime ends (Rust DWARF behavior)")]
    async fn debug_eval(&self, Parameters(params): Parameters<EvalParams>) -> String {
        handlers::debug_handlers::eval(&self.session_manager, params).await
    }

    /// Tool 9: Показать stack trace
    #[tool(description = "Show the call stack trace for the debug session")]
    async fn debug_backtrace(&self, Parameters(params): Parameters<BacktraceParams>) -> String {
        handlers::debug_handlers::backtrace(&self.session_manager, params).await
    }

    /// Tool 10: Установить условный breakpoint
    #[tool(description = "Set a conditional breakpoint (stops only if condition is true). WARNING: May not work for Rust programs in CodeLLDB. Use regular breakpoints + debug_eval as workaround")]
    async fn debug_set_conditional_breakpoint(
        &self,
        Parameters(params): Parameters<ConditionalBreakpointParams>,
    ) -> String {
        handlers::debug_handlers::set_conditional_breakpoint(&self.session_manager, params).await
    }

    /// Tool 11: Завершить debug сессию
    #[tool(description = "Terminate a debug session and clean up resources")]
    async fn debug_terminate(&self, Parameters(params): Parameters<TerminateParams>) -> String {
        handlers::session_handlers::terminate(&self.session_manager, params).await
    }

    /// Tool 12: Получить события DAP для сессии (polling API для AI)
    #[tool(description = "Poll DAP events for a debug session (stopped, output, terminated, etc.)")]
    async fn debug_poll_events(&self, Parameters(params): Parameters<PollEventsParams>) -> String {
        handlers::debug_handlers::poll_events(&self.session_manager, params).await
    }

    /// Tool 13: Выйти из текущей функции (step out)
    #[tool(description = "Step out of the current function. Note: May timeout with compiler-inlined functions (release builds). Use debug builds for reliable stepping")]
    async fn debug_step_out(&self, Parameters(params): Parameters<StepParams>) -> String {
        handlers::step_handlers::step_out(&self.session_manager, params).await
    }
}

// Реализация ServerHandler trait
#[tool_handler]
impl ServerHandler for DebugServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "MCP Debug Server - AI-assisted debugging through DAP protocol".to_string(),
            ),
        }
    }
}

impl Default for DebugServerHandler {
    fn default() -> Self {
        Self::new()
    }
}
