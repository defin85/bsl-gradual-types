use super::*;

#[test]
fn test_session_id_creation() {
    let id1 = SessionId::new();
    let id2 = SessionId::new();
    assert_ne!(id1, id2); // Должны быть разными
}

#[test]
fn test_session_id_from_string() {
    let id = SessionId::from_string("test-123".to_string());
    assert_eq!(id.as_str(), "test-123");
}

#[test]
fn test_session_id_display() {
    let id = SessionId::from_string("test-uuid".to_string());
    assert_eq!(format!("{}", id), "test-uuid");
}

#[test]
fn test_session_id_default() {
    let id = SessionId::default();
    assert!(!id.as_str().is_empty());
}
