use super::*;

#[test]
fn test_format_session_info() {
    let output = format_session_info("test-123", "Running", "test.exe");

    assert!(output.contains("Session Info:"));
    assert!(output.contains("ID: test-123"));
    assert!(output.contains("State: Running"));
    assert!(output.contains("Binary: test.exe"));
}

#[test]
fn test_format_success() {
    let output = format_success("Launch", "test-123", "- Thread: 1");

    assert!(output.contains("Launch successful:"));
    assert!(output.contains("Session: test-123"));
    assert!(output.contains("Thread: 1"));
}
