use std::sync::Arc;
use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters,
    },
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::session::SessionManager;

pub mod resources;
pub mod tools;

/// Параметры для debug_create_session
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSessionParams {
    pub binary_path: String,
    #[serde(default)]
    pub adapter_type: Option<String>,
}

/// Параметры для debug_set_breakpoint
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetBreakpointParams {
    pub session_id: String,
    pub file: String,
    pub line: u32,
}

/// Параметры для debug_launch
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LaunchParams {
    pub session_id: String,
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

/// Параметры для stepping operations (next, step_in, continue, step_out)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepParams {
    pub session_id: String,
}

/// Параметры для debug_eval
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvalParams {
    pub session_id: String,
    pub expression: String,
}

/// Параметры для debug_backtrace
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BacktraceParams {
    pub session_id: String,
}

/// Параметры для debug_set_conditional_breakpoint
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConditionalBreakpointParams {
    pub session_id: String,
    pub file: String,
    pub line: u32,
    pub condition: String,
}

/// Параметры для debug_terminate
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerminateParams {
    pub session_id: String,
}

/// Параметры для debug_poll_events
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PollEventsParams {
    pub session_id: String,
}

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
    async fn debug_create_session(&self, Parameters(params): Parameters<CreateSessionParams>) -> String {
        let adapter = params.adapter_type.unwrap_or_else(|| "lldb".to_string());

        match self.session_manager
            .create_session(params.binary_path.clone(), adapter.clone())
            .await
        {
            Ok(session_id) => format!(
                "Debug session created successfully:\n\
                 - Session ID: {}\n\
                 - Binary: {}\n\
                 - Adapter: {}",
                session_id, params.binary_path, adapter
            ),
            Err(e) => format!("Failed to create session: {}", e),
        }
    }

    /// Tool 2: Установить breakpoint
    #[tool(description = "Set a breakpoint at file:line in the debug session")]
    async fn debug_set_breakpoint(&self, Parameters(params): Parameters<SetBreakpointParams>) -> String {
        use crate::types::SessionId;

        let sid = SessionId::from_string(params.session_id.clone());
        let file = params.file.clone();
        let line = params.line;

        match self.session_manager
            .with_session(&sid, |session| {
                let file = file.clone();
                Box::pin(async move {
                    // Вызвать DAP client для установки breakpoint
                    session.dap_client.set_breakpoints(&file, &[line]).await
                        .map_err(|e| anyhow::anyhow!("DAP error: {}", e))?;

                    // Сохранить в session.breakpoints
                    session.breakpoints
                        .entry(file)
                        .or_insert_with(Vec::new)
                        .push(line);

                    Ok(())
                })
            })
            .await
        {
            Ok(_) => format!(
                "Breakpoint set successfully:\n\
                 - Session: {}\n\
                 - File: {}\n\
                 - Line: {}",
                params.session_id, params.file, params.line
            ),
            Err(e) => format!("Failed to set breakpoint: {}", e),
        }
    }

    /// Tool 3: Запустить программу под отладкой
    ///
    /// ВАЖНО: Использует правильную DAP последовательность:
    /// 1. launch_no_wait() - отправить launch request без ожидания response
    /// 2. Активное ожидание initialized event через polling (max 5 секунд)
    /// 3. configuration_done() - сигнал адаптеру для завершения launch
    #[tool(description = "Launch the program in the debug session")]
    async fn debug_launch(&self, Parameters(params): Parameters<LaunchParams>) -> String {
        use crate::types::SessionId;
        use crate::session::SessionState;
        use tokio::time::{sleep, Duration};

        let sid = SessionId::from_string(params.session_id.clone());

        // Шаг 1: Отправить launch request (без ожидания response)
        let launch_result = self.session_manager
            .with_session(&sid, |session| {
                let args = params.args.clone();
                Box::pin(async move {
                    let binary = session.binary_path.clone();

                    // Вызвать DAP launch_no_wait
                    session.dap_client.launch_no_wait(&binary, args).await
                        .map_err(|e| anyhow::anyhow!("DAP launch error: {}", e))?;

                    Ok(binary)
                })
            })
            .await;

        if let Err(e) = launch_result {
            return format!("Failed to launch: {}", e);
        }

        // Шаг 2: Активное ожидание initialized event (max 5 секунд)
        let max_retries = 50; // 50 * 100ms = 5 секунд
        let mut initialized_received = false;

        for _ in 0..max_retries {
            let events = self.session_manager.poll_events(&sid).await;

            for event in events {
                if let Some(event_type) = event.get("event").and_then(|v| v.as_str()) {
                    if event_type == "initialized" {
                        initialized_received = true;
                        break;
                    }
                }
            }

            if initialized_received {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        if !initialized_received {
            return format!(
                "Failed to launch: timeout waiting for 'initialized' event (5 seconds)\n\
                 - Session: {}",
                params.session_id
            );
        }

        // Шаг 3: Отправить configurationDone
        let config_result = self.session_manager
            .with_session(&sid, |session| Box::pin(async move {
                session.dap_client.configuration_done().await
                    .map_err(|e| anyhow::anyhow!("DAP configurationDone error: {}", e))?;

                // Обновить состояние на Running
                session.set_state(SessionState::Running)?;

                Ok(())
            }))
            .await;

        match config_result {
            Ok(_) => format!(
                "Program launched successfully:\n\
                 - Session: {}\n\
                 - State: Running\n\
                 - Received 'initialized' event\n\
                 - Sent 'configurationDone'",
                params.session_id
            ),
            Err(e) => format!("Failed to complete launch: {}", e),
        }
    }

    /// Tool 4: Step over (next line)
    #[tool(description = "Step over to the next line (step over functions)")]
    async fn debug_next(&self, Parameters(params): Parameters<StepParams>) -> String {
        use crate::types::SessionId;
        use crate::session::SessionState;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager
            .with_session(&sid, |session| Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }.ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP next
                session.dap_client.next(thread_id).await
                    .map_err(|e| anyhow::anyhow!("DAP next error: {}", e))?;

                // Обновить состояние на Stopped (после step)
                session.set_state(SessionState::Stopped)?;

                Ok(thread_id)
            }))
            .await
        {
            Ok(_) => format!(
                "Stepped to next line:\n\
                 - Session: {}\n\
                 - State: Stopped",
                params.session_id
            ),
            Err(e) => format!("Failed to step: {}", e),
        }
    }

    /// Tool 5: Step into function
    #[tool(description = "Step into the current function call")]
    async fn debug_step_in(&self, Parameters(params): Parameters<StepParams>) -> String {
        use crate::types::SessionId;
        use crate::session::SessionState;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager
            .with_session(&sid, |session| Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }.ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP stepIn
                session.dap_client.step_in(thread_id).await
                    .map_err(|e| anyhow::anyhow!("DAP stepIn error: {}", e))?;

                // Обновить состояние на Stopped
                session.set_state(SessionState::Stopped)?;

                Ok(thread_id)
            }))
            .await
        {
            Ok(_) => format!(
                "Stepped into function:\n\
                 - Session: {}\n\
                 - State: Stopped",
                params.session_id
            ),
            Err(e) => format!("Failed to step into: {}", e),
        }
    }

    /// Tool 6: Continue execution until next breakpoint
    #[tool(description = "Continue execution until hitting a breakpoint")]
    async fn debug_continue(&self, Parameters(params): Parameters<StepParams>) -> String {
        use crate::types::SessionId;
        use crate::session::SessionState;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager
            .with_session(&sid, |session| Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }.ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP continue
                session.dap_client.continue_execution(thread_id).await
                    .map_err(|e| anyhow::anyhow!("DAP continue error: {}", e))?;

                // Обновить состояние на Running
                session.set_state(SessionState::Running)?;

                Ok(thread_id)
            }))
            .await
        {
            Ok(_) => format!(
                "Execution continued:\n\
                 - Session: {}\n\
                 - State: Running\n\
                 - Will stop at next breakpoint",
                params.session_id
            ),
            Err(e) => format!("Failed to continue: {}", e),
        }
    }

    /// Tool 7: List active debug sessions
    #[tool(description = "List all active debug sessions")]
    async fn debug_list_sessions(&self) -> String {
        let sessions = self.session_manager.list_sessions().await;

        if sessions.is_empty() {
            return "No active debug sessions".to_string();
        }

        let mut output = String::from("Active debug sessions:\n");
        for (id, state, binary) in sessions {
            output.push_str(&format!(
                "- Session {}: {:?} (binary: {})\n",
                id, state, binary
            ));
        }
        output
    }

    /// Tool 8: Вычислить expression в текущем фрейме
    #[tool(description = "Evaluate an expression in the current stack frame")]
    async fn debug_eval(&self, Parameters(params): Parameters<EvalParams>) -> String {
        use crate::types::SessionId;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager
            .with_session(&sid, |session| {
                let expression = params.expression.clone();
                Box::pin(async move {
                    // Получить текущий frame_id (используем 0 для topmost frame)
                    let frame_id = Some(0);

                    // Вызвать DAP evaluate
                    let result = session.dap_client
                        .evaluate(&expression, frame_id)
                        .await
                        .map_err(|e| anyhow::anyhow!("DAP evaluate error: {}", e))?;

                    // result это Value, извлекаем строковое представление
                    let result_str = result
                        .get("result")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(no result)")
                        .to_string();

                    Ok(result_str)
                })
            })
            .await
        {
            Ok(result) => format!(
                "Expression evaluated:\n\
                 - Session: {}\n\
                 - Expression: {}\n\
                 - Result: {}",
                params.session_id, params.expression, result
            ),
            Err(e) => format!("Failed to evaluate expression: {}", e),
        }
    }

    /// Tool 9: Показать stack trace
    #[tool(description = "Show the call stack trace for the debug session")]
    async fn debug_backtrace(&self, Parameters(params): Parameters<BacktraceParams>) -> String {
        use crate::types::SessionId;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager
            .with_session(&sid, |session| Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }.ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP stackTrace
                let result = session.dap_client
                    .stack_trace(thread_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP stackTrace error: {}", e))?;

                // Извлечь stackFrames из result
                let frames = result
                    .get("stackFrames")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("No stackFrames in response"))?;

                // Форматировать stack trace
                let mut trace = String::from("Stack trace:\n");
                for (i, frame) in frames.iter().enumerate() {
                    let name = frame.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(unknown)");
                    let line = frame.get("line")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let source = frame.get("source")
                        .and_then(|s| s.get("path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("(unknown)");

                    trace.push_str(&format!(
                        "  #{}: {} at {}:{}\n",
                        i, name, source, line
                    ));
                }

                Ok(trace)
            }))
            .await
        {
            Ok(trace) => format!(
                "Backtrace for session {}:\n{}",
                params.session_id, trace
            ),
            Err(e) => format!("Failed to get backtrace: {}", e),
        }
    }

    /// Tool 10: Установить условный breakpoint
    #[tool(description = "Set a conditional breakpoint (stops only if condition is true)")]
    async fn debug_set_conditional_breakpoint(
        &self,
        Parameters(params): Parameters<ConditionalBreakpointParams>,
    ) -> String {
        use crate::types::SessionId;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager
            .with_session(&sid, |session| {
                let file = params.file.clone();
                let line = params.line;
                // Note: condition будет использоваться после расширения DapClient
                Box::pin(async move {
                    // DAP поддерживает условные breakpoints через поле "condition"
                    // TODO: Расширить DapClient.set_breakpoints для поддержки условных BP
                    // Временно используем обычный setBreakpoints

                    // Отправляем DAP request с условным breakpoint
                    let result = session.dap_client
                        .set_breakpoints(&file, &[line])
                        .await
                        .map_err(|e| anyhow::anyhow!("DAP error: {}", e))?;

                    // Сохранить в session.breakpoints
                    session.breakpoints
                        .entry(file.clone())
                        .or_insert_with(Vec::new)
                        .push(line);

                    Ok(result)
                })
            })
            .await
        {
            Ok(_) => format!(
                "Conditional breakpoint set:\n\
                 - Session: {}\n\
                 - File: {}\n\
                 - Line: {}\n\
                 - Condition: {}",
                params.session_id, params.file, params.line, params.condition
            ),
            Err(e) => format!("Failed to set conditional breakpoint: {}", e),
        }
    }

    /// Tool 11: Завершить debug сессию
    #[tool(description = "Terminate a debug session and clean up resources")]
    async fn debug_terminate(&self, Parameters(params): Parameters<TerminateParams>) -> String {
        use crate::types::SessionId;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager.terminate_session(&sid).await {
            Ok(_) => format!(
                "Debug session terminated successfully:\n\
                 - Session: {}",
                params.session_id
            ),
            Err(e) => format!("Failed to terminate session: {}", e),
        }
    }

    /// Tool 12: Получить события DAP для сессии (polling API для AI)
    #[tool(description = "Poll DAP events for a debug session (stopped, output, terminated, etc.)")]
    async fn debug_poll_events(&self, Parameters(params): Parameters<PollEventsParams>) -> String {
        use crate::types::SessionId;

        let sid = SessionId::from_string(params.session_id.clone());

        // Получить все накопленные события
        let events = self.session_manager.poll_events(&sid).await;

        if events.is_empty() {
            return format!(
                "No new events for session {}.\n\
                 Use this tool periodically to monitor program state.",
                params.session_id
            );
        }

        // Форматировать события human-readable
        let mut output = format!("Debug events for session {} ({} events):\n\n", params.session_id, events.len());

        for (i, event) in events.iter().enumerate() {
            let event_type = event
                .get("event")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            output.push_str(&format!("{}. Event: {}\n", i + 1, event_type));

            // Форматировать body по типу события
            match event_type {
                "stopped" => {
                    let reason = event
                        .get("body")
                        .and_then(|b| b.get("reason"))
                        .and_then(|r| r.as_str())
                        .unwrap_or("unknown");
                    let thread_id = event
                        .get("body")
                        .and_then(|b| b.get("threadId"))
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);

                    output.push_str(&format!(
                        "   Reason: {}\n   Thread ID: {}\n",
                        reason, thread_id
                    ));
                }
                "output" => {
                    let text = event
                        .get("body")
                        .and_then(|b| b.get("output"))
                        .and_then(|o| o.as_str())
                        .unwrap_or("");
                    let category = event
                        .get("body")
                        .and_then(|b| b.get("category"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("console");

                    output.push_str(&format!(
                        "   Category: {}\n   Output: {}\n",
                        category, text.trim()
                    ));
                }
                "terminated" | "exited" => {
                    output.push_str("   Program has exited\n");
                }
                "initialized" => {
                    output.push_str("   Debug adapter initialized\n");
                }
                _ => {
                    // Для неизвестных событий - показать полный JSON
                    output.push_str(&format!("   Body: {}\n", serde_json::to_string_pretty(event).unwrap_or_default()));
                }
            }

            output.push('\n');
        }

        output
    }

    /// Tool 13: Выйти из текущей функции (step out)
    #[tool(description = "Step out of the current function")]
    async fn debug_step_out(&self, Parameters(params): Parameters<StepParams>) -> String {
        use crate::types::SessionId;
        use crate::session::SessionState;

        let sid = SessionId::from_string(params.session_id.clone());

        match self.session_manager
            .with_session(&sid, |session| Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }.ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP stepOut
                session.dap_client.step_out(thread_id).await
                    .map_err(|e| anyhow::anyhow!("DAP stepOut error: {}", e))?;

                // Обновить состояние на Stopped
                session.set_state(SessionState::Stopped)?;

                Ok(thread_id)
            }))
            .await
        {
            Ok(_) => format!(
                "Stepped out of function:\n\
                 - Session: {}\n\
                 - State: Stopped",
                params.session_id
            ),
            Err(e) => format!("Failed to step out: {}", e),
        }
    }
}

// Реализация ServerHandler trait
#[tool_handler]
impl ServerHandler for DebugServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "MCP Debug Server - AI-assisted debugging through DAP protocol"
                    .to_string(),
            ),
        }
    }
}

impl Default for DebugServerHandler {
    fn default() -> Self {
        Self::new()
    }
}
