//! Debug handlers for breakpoints, launch, eval, backtrace, and events
//!
//! Contains the core debugging functionality implementations.

use crate::server::types::{
    BacktraceParams, ConditionalBreakpointParams, EvalParams, LaunchParams, PollEventsParams,
    SetBreakpointParams,
};
use crate::session::SessionManager;
use crate::types::SessionId;
use std::sync::Arc;

/// Set breakpoint implementation
pub async fn set_breakpoint(
    session_manager: &Arc<SessionManager>,
    params: SetBreakpointParams,
) -> String {
    let sid = SessionId::from_string(params.session_id.clone());
    let file = params.file.clone();
    let line = params.line;

    match session_manager
        .with_session(&sid, |session| {
            let file = file.clone();
            Box::pin(async move {
                // Вызвать DAP client для установки breakpoint
                session
                    .dap_client
                    .set_breakpoints(&file, &[line])
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP error: {}", e))?;

                // Сохранить в session.breakpoints
                session
                    .breakpoints
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

/// Launch program implementation
///
/// ВАЖНО: Использует правильную DAP последовательность:
/// 1. launch_no_wait() - отправить launch request без ожидания response
/// 2. Активное ожидание initialized event через polling (max 5 секунд)
/// 3. configuration_done() - сигнал адаптеру для завершения launch
pub async fn launch(session_manager: &Arc<SessionManager>, params: LaunchParams) -> String {
    use crate::session::SessionState;
    use tokio::time::{sleep, Duration};

    let sid = SessionId::from_string(params.session_id.clone());

    // Шаг 1: Отправить launch request (без ожидания response)
    let launch_result = session_manager
        .with_session(&sid, |session| {
            let args = params.args.clone();
            Box::pin(async move {
                let binary = session.binary_path.clone();

                // Вызвать DAP launch_no_wait
                session
                    .dap_client
                    .launch_no_wait(&binary, args)
                    .await
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
        let events = session_manager.poll_events(&sid).await;

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
    let config_result = session_manager
        .with_session(&sid, |session| {
            Box::pin(async move {
                session
                    .dap_client
                    .configuration_done()
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP configurationDone error: {}", e))?;

                // Обновить состояние на Running
                session.set_state(SessionState::Running)?;

                Ok(())
            })
        })
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

/// Evaluate expression implementation
///
/// Ограничения:
/// - Переменные могут быть недоступны после их lifetime (Rust DWARF behavior)
/// - Используйте переменные до того, как они выйдут из области видимости
pub async fn eval(session_manager: &Arc<SessionManager>, params: EvalParams) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager
        .with_session(&sid, |session| {
            let expression = params.expression.clone();
            Box::pin(async move {
                // Получить реальный frameId через helper метод
                // (включает проверку state и thread_id)
                let frame_id = session.get_current_frame_id().await?;

                // Вызвать DAP evaluate с реальным frameId
                let result = session
                    .dap_client
                    .evaluate(&expression, Some(frame_id))
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

/// Backtrace implementation
pub async fn backtrace(session_manager: &Arc<SessionManager>, params: BacktraceParams) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager
        .with_session(&sid, |session| {
            Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }
                .ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP stackTrace
                let result = session
                    .dap_client
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
                    let name = frame
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(unknown)");
                    let line = frame.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
                    let source = frame
                        .get("source")
                        .and_then(|s| s.get("path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("(unknown)");

                    trace.push_str(&format!("  #{}: {} at {}:{}\n", i, name, source, line));
                }

                Ok(trace)
            })
        })
        .await
    {
        Ok(trace) => format!("Backtrace for session {}:\n{}", params.session_id, trace),
        Err(e) => format!("Failed to get backtrace: {}", e),
    }
}

/// Set conditional breakpoint implementation
///
/// Ограничения:
/// - Условные breakpoints могут НЕ работать для Rust в CodeLLDB (известная проблема)
/// - Для отладки Rust рекомендуется использовать обычные breakpoints + debug_eval
/// - См. https://github.com/vadimcn/codelldb/issues/253
pub async fn set_conditional_breakpoint(
    session_manager: &Arc<SessionManager>,
    params: ConditionalBreakpointParams,
) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager
        .with_session(&sid, |session| {
            let file = params.file.clone();
            let line = params.line;
            let condition = params.condition.clone();
            Box::pin(async move {
                // Отправляем DAP request с условным breakpoint
                let result = session
                    .dap_client
                    .set_conditional_breakpoint(&file, line, &condition)
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP error: {}", e))?;

                // Сохранить в session.breakpoints
                session
                    .breakpoints
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

/// Poll events implementation
pub async fn poll_events(
    session_manager: &Arc<SessionManager>,
    params: PollEventsParams,
) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    // Получить все накопленные события
    let events = session_manager.poll_events(&sid).await;

    if events.is_empty() {
        return format!(
            "No new events for session {}.\n\
             Use this tool periodically to monitor program state.",
            params.session_id
        );
    }

    // Форматировать события human-readable
    let mut output = format!(
        "Debug events for session {} ({} events):\n\n",
        params.session_id,
        events.len()
    );

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
                    category,
                    text.trim()
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
                output.push_str(&format!(
                    "   Body: {}\n",
                    serde_json::to_string_pretty(event).unwrap_or_default()
                ));
            }
        }

        output.push('\n');
    }

    output
}
