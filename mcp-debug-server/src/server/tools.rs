// Вспомогательные функции для MCP Tools
// Все tools определены в server/mod.rs через макросы #[tool_router] и #[tool]
//
// Этот модуль содержит только helper функции для форматирования вывода

/// Helper: Форматировать информацию о debug сессии
pub fn format_session_info(
    session_id: &str,
    state: &str,
    binary: &str,
) -> String {
    format!(
        "Session Info:\n\
         - ID: {}\n\
         - State: {}\n\
         - Binary: {}",
        session_id, state, binary
    )
}

/// Helper: Форматировать успешный результат операции
pub fn format_success(operation: &str, session_id: &str, details: &str) -> String {
    format!(
        "{} successful:\n\
         - Session: {}\n\
         {}",
        operation, session_id, details
    )
}

#[cfg(test)]
mod tests {
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
}
