use serde::{Deserialize, Serialize};

/// Debug session state machine.
///
/// Represents the current state of a debug session. States follow a strict transition model:
///
/// ```text
/// Initialized → Running → Stopped → Running → ... → Terminated
///     ↓                      ↑          ↓
///     └──────────────────────┴──────────┘
///                    (loop)
/// ```
///
/// ## Valid Transitions
///
/// | From         | To           | Description                  |
/// |--------------|--------------|------------------------------|
/// | Initialized  | Running      | Launch program               |
/// | Running      | Stopped      | Hit breakpoint, step         |
/// | Stopped      | Running      | Continue execution           |
/// | Any          | Terminated   | Program exits, user stops    |
///
/// ## Invalid Transitions
///
/// - `Initialized → Stopped` (cannot pause without running)
/// - `Terminated → *` (cannot resume terminated session)
///
/// # Examples
///
/// ```
/// use mcp_debug_server::session::SessionState;
///
/// let state = SessionState::Initialized;
/// assert!(state.can_transition_to(SessionState::Running));
/// assert!(!state.can_transition_to(SessionState::Stopped));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Ready to launch (initialized but not running).
    Initialized,

    /// Executing code (not paused).
    Running,

    /// Paused on breakpoint or after step.
    Stopped,

    /// Program finished or session terminated.
    Terminated,
}

impl SessionState {
    /// Checks if transition from current state to `next` is valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_debug_server::session::SessionState;
    ///
    /// let current = SessionState::Running;
    /// assert!(current.can_transition_to(SessionState::Stopped));
    /// assert!(!current.can_transition_to(SessionState::Initialized));
    /// ```
    pub fn can_transition_to(&self, new_state: SessionState) -> bool {
        use SessionState::*;

        match (self, new_state) {
            // Из Initialized можно в Running или Terminated
            (Initialized, Running) => true,
            (Initialized, Terminated) => true,

            // Из Running можно в Stopped или Terminated
            (Running, Stopped) => true,
            (Running, Terminated) => true,

            // Из Stopped можно в Running или Terminated
            (Stopped, Running) => true,
            (Stopped, Terminated) => true,

            // Из Terminated никуда нельзя
            (Terminated, _) => false,

            // Переход в то же состояние разрешён
            (state, new) if state == &new => true,

            // Все остальные переходы запрещены
            _ => false,
        }
    }

    /// Returns human-readable description of the state.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_debug_server::session::SessionState;
    ///
    /// let state = SessionState::Running;
    /// assert_eq!(state.description(), "Running (executing code)");
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            Self::Initialized => "Initialized (ready to launch)",
            Self::Running => "Running (executing code)",
            Self::Stopped => "Stopped (paused on breakpoint)",
            Self::Terminated => "Terminated (finished)",
        }
    }
}

#[cfg(test)]
mod tests {
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
}
