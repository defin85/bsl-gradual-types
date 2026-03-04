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
