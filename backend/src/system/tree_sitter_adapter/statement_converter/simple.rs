//! Простые statements: assignment, return, call
//!
//! Эти statements не содержат вложенных блоков кода,
//! поэтому не требуют рекурсивного вызова dispatcher.

use crate::parsing::bsl::ast::{Expression, Statement};
use bsl_shared::ir::Span;
use tree_sitter::Node;

use crate::system::tree_sitter_adapter::expression_converter::convert_expression;
use crate::system::tree_sitter_adapter::span::node_to_span_cached;

/// Конвертировать assignment_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_assignment_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    let mut target = None;
    let mut value = None;

    for child in node.children(&mut cursor) {
        let child_kind = child.kind();
        if child_kind == "identifier" || child_kind == "property_access" {
            if target.is_none() {
                target = convert_expression(&child, source)?;
            } else if value.is_none() {
                // BUGFIX MILESTONE 3.16: If target is already set,
                // property_access/identifier goes to value!
                // Example: Dok = Documents.OrderClient
                //   - target = "Dok" (identifier)
                //   - value = "Documents.OrderClient" (property_access)
                value = convert_expression(&child, source)?;
            }
        } else if let Some(expr) = convert_expression(&child, source)? {
            value = Some(expr);
        }
    }

    Ok(Statement::Assignment {
        target: target.unwrap_or(Expression::Identifier {
            name: "unknown".to_string(),
            span: Span::stub(),
        }),
        value: value.unwrap_or(Expression::Identifier {
            name: "unknown".to_string(),
            span: Span::stub(),
        }),
        span,
    })
}

/// Конвертировать return_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_return_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    let mut value = None;

    for child in node.children(&mut cursor) {
        if child.kind() == "RETURN_KEYWORD" || child.kind() == "ВОЗВРАТ_KEYWORD" {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            value = Some(expr);
            break;
        }
    }

    Ok(Statement::Return { value, span })
}

/// Конвертировать call_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_call_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Statement::Call {
                expression: expr,
                span,
            });
        }
    }

    Err("call_statement without expression".to_string())
}
