//! Вспомогательные функции для работы с tree-sitter nodes

use crate::parsing::bsl::ast::Expression;
use tree_sitter::Node;

/// Получить текст узла из исходного кода
pub fn node_text(node: &Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

/// Конвертировать parameters
pub fn convert_parameters(node: &Node, source: &str) -> Result<Vec<String>, String> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "parameter" {
            // parameter содержит identifier как дочерний узел
            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                if param_child.kind() == "identifier" {
                    params.push(node_text(&param_child, source));
                    break; // Берём только имя параметра
                }
            }
        }
    }

    Ok(params)
}

/// Извлечь пару event-handler из узла
///
/// Используется для add_handler_statement и remove_handler_statement
pub fn extract_event_handler_pair(
    node: &Node,
    source: &str,
    convert_expression: impl Fn(&Node, &str) -> Result<Option<Expression>, String>,
) -> Result<(Expression, Expression), String> {
    let mut cursor = node.walk();
    let mut event = None;
    let mut handler = None;

    for child in node.children(&mut cursor) {
        // Пропускаем ключевые слова
        if child.kind().ends_with("_KEYWORD") {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            if event.is_none() {
                event = Some(expr);
            } else {
                handler = Some(expr);
                break;
            }
        }
    }

    match (event, handler) {
        (Some(e), Some(h)) => Ok((e, h)),
        _ => Err("handler statement requires event and handler".to_string()),
    }
}
