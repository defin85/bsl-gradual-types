use super::*;

#[test]
fn test_valid_transitions() {
    use SessionState::*;

    // Initialized → Running
    assert!(Initialized.can_transition_to(Running));

    // Running → Stopped
    assert!(Running.can_transition_to(Stopped));

    // Stopped → Running
    assert!(Stopped.can_transition_to(Running));

    // Any → Terminated
    assert!(Initialized.can_transition_to(Terminated));
    assert!(Running.can_transition_to(Terminated));
    assert!(Stopped.can_transition_to(Terminated));
}

#[test]
fn test_invalid_transitions() {
    use SessionState::*;

    // Initialized → Stopped (нельзя)
    assert!(!Initialized.can_transition_to(Stopped));

    // Terminated → * (нельзя никуда)
    assert!(!Terminated.can_transition_to(Initialized));
    assert!(!Terminated.can_transition_to(Running));
    assert!(!Terminated.can_transition_to(Stopped));
}

#[test]
fn test_same_state_transition() {
    use SessionState::*;

    // Переход в то же состояние разрешён
    assert!(Initialized.can_transition_to(Initialized));
    assert!(Running.can_transition_to(Running));
}
