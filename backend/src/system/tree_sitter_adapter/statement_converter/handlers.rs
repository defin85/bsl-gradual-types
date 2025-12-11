//! Обработчики событий: add/remove_handler, await
//!
//! Эти statements связаны с асинхронным программированием
//! и обработкой событий в 1С.

use crate::parsing::bsl::ast::Statement;
use tree_sitter::Node;

use crate::system::tree_sitter_adapter::expression_converter::convert_expression;
use crate::system::tree_sitter_adapter::span::node_to_span_cached;
use crate::system::tree_sitter_adapter::utils::extract_event_handler_pair;

/// Конвертировать add_handler_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_add_handler_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let (event, handler) = extract_event_handler_pair(node, source, convert_expression)?;
    Ok(Statement::AddHandler {
        event,
        handler,
        span,
    })
}

/// Конвертировать remove_handler_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_remove_handler_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let (event, handler) = extract_event_handler_pair(node, source, convert_expression)?;
    Ok(Statement::RemoveHandler {
        event,
        handler,
        span,
    })
}

/// Конвертировать await_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_await_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().ends_with("_KEYWORD") {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Statement::Await {
                expression: expr,
                span,
            });
        }
    }
    Err("await_statement without expression".to_string())
}
