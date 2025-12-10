//! Session management handlers
//!
//! Handlers for creating, listing, and terminating debug sessions.

use crate::server::types::{CreateSessionParams, TerminateParams};
use crate::session::SessionManager;
use crate::types::SessionId;
use std::sync::Arc;

/// Create session implementation
pub async fn create_session(
    session_manager: &Arc<SessionManager>,
    params: CreateSessionParams,
) -> String {
    let adapter = params.adapter_type.unwrap_or_else(|| "lldb".to_string());

    match session_manager
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

/// List sessions implementation
pub async fn list_sessions(session_manager: &Arc<SessionManager>) -> String {
    let sessions = session_manager.list_sessions().await;

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

/// Terminate session implementation
pub async fn terminate(session_manager: &Arc<SessionManager>, params: TerminateParams) -> String {
    let sid = SessionId::from_string(params.session_id.clone());

    match session_manager.terminate_session(&sid).await {
        Ok(_) => format!(
            "Debug session terminated successfully:\n\
             - Session: {}",
            params.session_id
        ),
        Err(e) => format!("Failed to terminate session: {}", e),
    }
}
