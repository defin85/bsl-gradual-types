//! Условные конструкции: if/elseif/else
//!
//! Модуль содержит конвертеры для условных statements.
//! Использует dispatcher для рекурсивной обработки вложенных блоков.

use crate::ast::{Expression, Statement};
use bsl_shared::ir::Span;
use tree_sitter::Node;

use crate::tree_sitter_adapter::expression_converter::convert_expression;
use crate::tree_sitter_adapter::span::{node_to_span_cached, LineIndex};

/// Конвертировать if_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_if_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, line_index);
    let mut cursor = node.walk();
    let mut condition = Expression::Boolean {
        value: true,
        span: Span::stub(),
    };
    let mut then_body = Vec::new();
    let mut else_body = None;
    let mut in_then = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "IF_KEYWORD" => {}
            "THEN_KEYWORD" => in_then = true,
            "ENDIF_KEYWORD" => break,
            "expression" => {
                if !in_then {
                    if let Some(expr) = convert_expression(&child, source)? {
                        condition = expr;
                    }
                }
            }
            "else_clause" => {
                let else_statements = convert_clause_body_cached(&child, source, line_index)?;
                else_body = Some(else_statements);
            }
            "elseif_clause" => {
                let elseif_statements = convert_clause_body_cached(&child, source, line_index)?;
                else_body = Some(elseif_statements);
            }
            kind if in_then && (kind.ends_with("_statement") || kind.ends_with("_definition")) => {
                if let Some(stmt) = super::dispatch_statement_cached(&child, source, line_index)? {
                    then_body.push(stmt);
                }
            }
            _ => {}
        }
    }

    Ok(Statement::If {
        condition,
        then_body,
        else_body,
        span,
    })
}

/// Конвертировать clause_body с использованием кеша строк (Milestone 2.19)
///
/// Используется для обработки else_clause и elseif_clause.
pub(crate) fn convert_clause_body_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "ELSE_KEYWORD" || child.kind() == "ELSIF_KEYWORD" {
            continue;
        }

        if let Some(stmt) = super::dispatch_statement_cached(&child, source, line_index)? {
            statements.push(stmt);
        }
    }

    Ok(statements)
}
