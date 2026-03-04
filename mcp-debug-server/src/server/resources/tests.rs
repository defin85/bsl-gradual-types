use super::*;

#[test]
fn test_get_resource_list() {
    let resources = get_resource_list();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "debug://sessions");
    assert_eq!(resources[0].name, "Active Debug Sessions");
}

#[tokio::test]
async fn test_read_sessions_list_empty() {
    let session_manager = Arc::new(SessionManager::new());
    let result = read_sessions_list(session_manager).await;

    assert!(result.is_ok());
    let content = result.unwrap();

    // ResourceContents::text возвращает BlobResourceContents или TextResourceContents
    // Проверяем через debug representation или преобразование
    let debug_str = format!("{:?}", content);
    assert!(debug_str.contains("debug://sessions"));
}

#[tokio::test]
async fn test_read_nonexistent_session_state() {
    let session_manager = Arc::new(SessionManager::new());
    let result = read_session_state("nonexistent_id", session_manager).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Session not found"));
}

#[test]
fn test_uri_parsing() {
    // Валидный URI для state
    let uri = "debug://session/12345/state";
    let parts: Vec<&str> = uri
        .trim_start_matches("debug://session/")
        .split('/')
        .collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "12345");
    assert_eq!(parts[1], "state");

    // Валидный URI для breakpoints
    let uri = "debug://session/67890/breakpoints";
    let parts: Vec<&str> = uri
        .trim_start_matches("debug://session/")
        .split('/')
        .collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "67890");
    assert_eq!(parts[1], "breakpoints");
}
