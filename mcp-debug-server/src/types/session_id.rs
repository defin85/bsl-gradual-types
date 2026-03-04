use uuid::Uuid;

/// Unique identifier for debug sessions.
///
/// Internally represented as a UUID v4 string, guaranteeing uniqueness across sessions.
///
/// # Examples
///
/// ```
/// use mcp_debug_server::types::SessionId;
///
/// // Create new random ID
/// let id = SessionId::new();
/// println!("Session: {}", id);  // Displays UUID
///
/// // Create from existing string (for tests)
/// let id2 = SessionId::from_string("my-test-session-123".to_string());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Creates a new random SessionId using UUID v4.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Creates SessionId from existing string (primarily for testing).
    ///
    /// **Note:** Does not validate format. Use [`new()`](Self::new) for production.
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Returns string representation of the ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
#[path = "session_id/tests.rs"]
mod tests;
