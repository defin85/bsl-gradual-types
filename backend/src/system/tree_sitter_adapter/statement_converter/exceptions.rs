//! Обработка исключений: try/except, raise
//!
//! Модуль содержит конвертеры для конструкций обработки ошибок.
//! Использует dispatcher для рекурсивной обработки блоков try/except.

use crate::parsing::bsl::ast::Statement;
use tree_sitter::Node;

use crate::system::tree_sitter_adapter::expression_converter::convert_expression;
use crate::system::tree_sitter_adapter::span::node_to_span_cached;

/// Конвертировать try_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_try_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    let mut try_body = Vec::new();
    let mut except_body = Vec::new();
    let mut in_except = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "TRY_KEYWORD" | "ПОПЫТКА_KEYWORD" | "ENDTRY_KEYWORD" | "КОНЕЦПОПЫТКИ_KEYWORD" =>
                {}
            "EXCEPT_KEYWORD" | "ИСКЛЮЧЕНИЕ_KEYWORD" => {
                in_except = true;
            }
            _ => {
                if let Some(stmt) = super::dispatch_statement_cached(&child, source, lines)? {
                    if in_except {
                        except_body.push(stmt);
                    } else {
                        try_body.push(stmt);
                    }
                }
            }
        }
    }

    Ok(Statement::Try {
        try_body,
        except_body,
        span,
    })
}

/// Конвертировать raise_error_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_raise_error_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    let mut message = None;

    for child in node.children(&mut cursor) {
        if child.kind().ends_with("_KEYWORD") {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            message = Some(expr);
            break;
        }
    }

    Ok(Statement::RaiseError { message, span })
}
