// Вспомогательные функции для MCP Tools
// Все tools определены в server/mod.rs через макросы #[tool_router] и #[tool]
//
// Этот модуль содержит только helper функции для форматирования вывода

/// Helper: Форматировать информацию о debug сессии
pub fn format_session_info(session_id: &str, state: &str, binary: &str) -> String {
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
#[path = "tools/tests.rs"]
mod tests;
