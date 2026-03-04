use rmcp::model::{Annotated, RawResource, ResourceContents};
use serde_json::json;
use std::sync::Arc;

use crate::session::SessionManager;
use crate::types::SessionId;

/// Получить список доступных MCP Resources
pub fn get_resource_list() -> Vec<Annotated<RawResource>> {
    vec![
        Annotated {
            raw: RawResource {
                uri: "debug://sessions".to_string(),
                name: "Active Debug Sessions".to_string(),
                title: None,
                description: Some(
                    "List of all active debug sessions with their states".to_string(),
                ),
                mime_type: Some("application/json".to_string()),
                size: None,
                icons: None,
            },
            annotations: None,
        },
        // Динамические resources (session/{id}/...) не включаются в list
        // AI может их запросить напрямую, зная session_id
    ]
}

/// Прочитать содержимое resource по URI
pub async fn read_resource(
    uri: &str,
    session_manager: Arc<SessionManager>,
) -> Result<ResourceContents, String> {
    // Парсинг URI
    if uri == "debug://sessions" {
        return read_sessions_list(session_manager).await;
    }

    // Парсинг динамических URIs: debug://session/{id}/...
    if uri.starts_with("debug://session/") {
        let parts: Vec<&str> = uri
            .trim_start_matches("debug://session/")
            .split('/')
            .collect();
        if parts.len() < 2 {
            return Err("Invalid session URI format".to_string());
        }

        let session_id = parts[0];
        let resource_type = parts[1];

        match resource_type {
            "state" => read_session_state(session_id, session_manager).await,
            "breakpoints" => read_session_breakpoints(session_id, session_manager).await,
            _ => Err(format!("Unknown resource type: {}", resource_type)),
        }
    } else {
        Err(format!("Unknown resource URI: {}", uri))
    }
}

/// Resource: debug://sessions
async fn read_sessions_list(
    session_manager: Arc<SessionManager>,
) -> Result<ResourceContents, String> {
    let sessions = session_manager.list_sessions().await;

    let sessions_json: Vec<_> = sessions
        .into_iter()
        .map(|(id, state, binary)| {
            json!({
                "session_id": id.to_string(),
                "state": format!("{:?}", state),
                "binary_path": binary,
            })
        })
        .collect();

    let content = json!({
        "sessions": sessions_json,
        "count": sessions_json.len(),
    });

    Ok(ResourceContents::text(
        serde_json::to_string_pretty(&content)
            .map_err(|e| format!("JSON serialization error: {}", e))?,
        "debug://sessions",
    ))
}

/// Resource: debug://session/{id}/state
async fn read_session_state(
    session_id: &str,
    session_manager: Arc<SessionManager>,
) -> Result<ResourceContents, String> {
    let sid = SessionId::from_string(session_id.to_string());

    if !session_manager.session_exists(&sid).await {
        return Err(format!("Session not found: {}", session_id));
    }

    // Получить информацию о сессии через list_sessions
    let sessions = session_manager.list_sessions().await;
    let session_info = sessions
        .into_iter()
        .find(|(id, _, _)| id.as_str() == session_id)
        .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let content = json!({
        "session_id": session_info.0.to_string(),
        "state": format!("{:?}", session_info.1),
        "binary_path": session_info.2,
        "uri": format!("debug://session/{}/state", session_id),
    });

    Ok(ResourceContents::text(
        serde_json::to_string_pretty(&content)
            .map_err(|e| format!("JSON serialization error: {}", e))?,
        format!("debug://session/{}/state", session_id),
    ))
}

/// Resource: debug://session/{id}/breakpoints
async fn read_session_breakpoints(
    session_id: &str,
    session_manager: Arc<SessionManager>,
) -> Result<ResourceContents, String> {
    let sid = SessionId::from_string(session_id.to_string());

    // Получить breakpoints через with_session
    let breakpoints = session_manager
        .with_session(&sid, |session| {
            Box::pin(async move {
                // Преобразовать HashMap в Vec для сериализации
                let bps: Vec<_> = session
                    .breakpoints
                    .iter()
                    .flat_map(|(file, lines)| {
                        lines.iter().map(move |line| {
                            json!({
                                "file": file,
                                "line": line,
                            })
                        })
                    })
                    .collect();

                Ok(bps)
            })
        })
        .await
        .map_err(|e| format!("Failed to read breakpoints: {}", e))?;

    let content = json!({
        "session_id": session_id,
        "breakpoints": breakpoints,
        "count": breakpoints.len(),
    });

    Ok(ResourceContents::text(
        serde_json::to_string_pretty(&content)
            .map_err(|e| format!("JSON serialization error: {}", e))?,
        format!("debug://session/{}/breakpoints", session_id),
    ))
}

#[cfg(test)]
#[path = "resources/tests.rs"]
mod tests;
