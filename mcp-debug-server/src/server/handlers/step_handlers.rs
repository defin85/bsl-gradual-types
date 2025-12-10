//! Step execution handlers
//!
//! Handlers for stepping through code: next, step_in, step_out, continue.

use crate::server::types::StepParams;
use crate::session::{SessionManager, SessionState};
use crate::types::SessionId;
use std::sync::Arc;

/// Step next (step over) implementation
pub async fn next(session_manager: &Arc<SessionManager>, params: StepParams) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager
        .with_session(&sid, |session| {
            Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }
                .ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP next
                session
                    .dap_client
                    .next(thread_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP next error: {}", e))?;

                // Обновить состояние на Stopped (после step)
                session.set_state(SessionState::Stopped)?;

                Ok(thread_id)
            })
        })
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

/// Step into implementation
pub async fn step_in(session_manager: &Arc<SessionManager>, params: StepParams) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager
        .with_session(&sid, |session| {
            Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }
                .ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP stepIn
                session
                    .dap_client
                    .step_in(thread_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP stepIn error: {}", e))?;

                // Обновить состояние на Stopped
                session.set_state(SessionState::Stopped)?;

                Ok(thread_id)
            })
        })
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

/// Continue execution implementation
pub async fn continue_execution(
    session_manager: &Arc<SessionManager>,
    params: StepParams,
) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager
        .with_session(&sid, |session| {
            Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }
                .ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP continue
                session
                    .dap_client
                    .continue_execution(thread_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP continue error: {}", e))?;

                // Обновить состояние на Running
                session.set_state(SessionState::Running)?;

                Ok(thread_id)
            })
        })
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

/// Step out implementation
///
/// Ограничения:
/// - Может давать timeout при compiler inlining (функции оптимизированы в release mode)
/// - Для надежной отладки используйте debug build или step over вместо step out
pub async fn step_out(session_manager: &Arc<SessionManager>, params: StepParams) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager
        .with_session(&sid, |session| {
            Box::pin(async move {
                let thread_id = {
                    let guard = session.current_thread_id.lock().await;
                    *guard
                }
                .ok_or_else(|| anyhow::anyhow!("No active thread"))?;

                // Вызвать DAP stepOut
                session
                    .dap_client
                    .step_out(thread_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("DAP stepOut error: {}", e))?;

                // Обновить состояние на Stopped
                session.set_state(SessionState::Stopped)?;

                Ok(thread_id)
            })
        })
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
