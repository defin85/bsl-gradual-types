use super::*;

#[tokio::test]
async fn test_session_manager_creation() {
    let manager = SessionManager::new();
    let sessions = manager.list_sessions().await;
    assert_eq!(sessions.len(), 0);
}

#[test]
fn test_session_state_validation() {
    // Тестируем валидацию state transitions через SessionState API
    // (без создания DebugSession, так как требуется реальный DapClient)

    use SessionState::*;

    // Тест 1: Valid transition Initialized → Running
    let current_state = Initialized;
    assert!(current_state.can_transition_to(Running));

    // Тест 2: Invalid transition Running → Initialized
    let current_state = Running;
    assert!(!current_state.can_transition_to(Initialized));

    // Тест 3: Valid transition to Terminated
    assert!(Running.can_transition_to(Terminated));
    assert!(Stopped.can_transition_to(Terminated));
}
