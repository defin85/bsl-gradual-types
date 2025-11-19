use std::io;

/// Ошибки DAP Client
#[derive(Debug, thiserror::Error)]
pub enum DapError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("DAP protocol error: {0}")]
    Protocol(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Process spawn failed: {0}")]
    SpawnFailed(String),
}

pub type DapResult<T> = Result<T, DapError>;

/// Errors that can occur during debug operations.
///
/// This enum centralizes all error types from DAP protocol, session management,
/// and I/O operations. Automatically converts to MCP `ErrorData` for wire protocol.
///
/// # Examples
///
/// ```
/// use mcp_debug_server::types::McpDebugError;
/// use rmcp::ErrorData;
///
/// let err = McpDebugError::SessionNotFound("abc-123".to_string());
/// let mcp_err: ErrorData = err.into();
/// assert_eq!(mcp_err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
/// ```
#[derive(Debug, thiserror::Error)]
pub enum McpDebugError {
    /// DAP protocol error (adapter communication failure).
    #[error("DAP protocol error: {0}")]
    DapProtocol(String),

    /// Session not found in manager.
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Invalid state transition attempted.
    #[error("Invalid session state: expected {expected}, got {actual}")]
    InvalidState { expected: String, actual: String },

    /// I/O error (file read, process spawn, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// DAP request timed out (default: 5 seconds).
    #[error("Timeout waiting for DAP response")]
    Timeout,

    /// DAP adapter process crashed.
    #[error("DAP adapter process terminated unexpectedly")]
    AdapterCrashed,

    /// Invalid parameters provided to MCP tool.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Other unclassified errors.
    #[error("{0}")]
    Other(String),
}

/// Конверсия из DapError в McpDebugError
impl From<DapError> for McpDebugError {
    fn from(err: DapError) -> Self {
        match err {
            DapError::Io(e) => McpDebugError::Io(e),
            DapError::Json(e) => McpDebugError::Json(e),
            DapError::Protocol(msg) => McpDebugError::DapProtocol(msg),
            DapError::InvalidResponse(msg) => McpDebugError::DapProtocol(msg),
            DapError::Timeout => McpDebugError::Timeout,
            DapError::SpawnFailed(msg) => McpDebugError::Other(msg),
        }
    }
}

/// Конверсия из anyhow::Error в McpDebugError
impl From<anyhow::Error> for McpDebugError {
    fn from(err: anyhow::Error) -> Self {
        // Попытка downcast в известные типы
        if let Some(dap_err) = err.downcast_ref::<DapError>() {
            return match dap_err {
                DapError::Io(_) => McpDebugError::Io(std::io::Error::other(
                    err.to_string()
                )),
                DapError::Timeout => McpDebugError::Timeout,
                _ => McpDebugError::DapProtocol(err.to_string()),
            };
        }

        // Проверка на session not found
        let err_str = err.to_string();
        if err_str.contains("Session not found") {
            McpDebugError::SessionNotFound(err_str)
        } else if err_str.contains("Invalid state transition") {
            McpDebugError::InvalidState {
                expected: "valid".to_string(),
                actual: err_str,
            }
        } else {
            McpDebugError::Other(err_str)
        }
    }
}

impl From<McpDebugError> for rmcp::ErrorData {
    /// Converts to MCP ErrorData with appropriate error code.
    ///
    /// # Error Code Mapping
    ///
    /// - `SessionNotFound`, `InvalidParams` → `INVALID_PARAMS`
    /// - `DapProtocol`, `Timeout`, `AdapterCrashed` → `INTERNAL_ERROR`
    /// - Others → `INTERNAL_ERROR`
    fn from(err: McpDebugError) -> Self {
        use rmcp::ErrorData;

        match err {
            McpDebugError::SessionNotFound(_) | McpDebugError::InvalidParams(_) => {
                ErrorData::invalid_params(err.to_string(), None)
            }
            McpDebugError::DapProtocol(_)
            | McpDebugError::AdapterCrashed
            | McpDebugError::Timeout => {
                ErrorData::internal_error(err.to_string(), None)
            }
            McpDebugError::InvalidState { .. } => {
                ErrorData::invalid_params(err.to_string(), None)
            }
            McpDebugError::Io(_) | McpDebugError::Json(_) | McpDebugError::Other(_) => {
                ErrorData::internal_error(err.to_string(), None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ErrorCode;

    #[test]
    fn test_error_to_rmcp_error_data() {
        let err = McpDebugError::SessionNotFound("test-123".to_string());
        let error_data: rmcp::ErrorData = err.into();

        assert_eq!(error_data.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn test_dap_protocol_error() {
        let err = McpDebugError::DapProtocol("invalid response".to_string());
        let error_data: rmcp::ErrorData = err.into();

        assert_eq!(error_data.code, ErrorCode::INTERNAL_ERROR);
    }

    #[test]
    fn test_timeout_error() {
        let err = McpDebugError::Timeout;
        let error_data: rmcp::ErrorData = err.into();

        assert_eq!(error_data.code, ErrorCode::INTERNAL_ERROR);
    }

    #[test]
    fn test_invalid_state_error() {
        let err = McpDebugError::InvalidState {
            expected: "Running".to_string(),
            actual: "Terminated".to_string(),
        };
        let error_data: rmcp::ErrorData = err.into();

        assert_eq!(error_data.code, ErrorCode::INVALID_PARAMS);
    }

    #[test]
    fn test_dap_error_conversion() {
        let dap_err = DapError::Timeout;
        let mcp_err: McpDebugError = dap_err.into();

        assert!(matches!(mcp_err, McpDebugError::Timeout));
    }
}
