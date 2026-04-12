//! Циклы: for, foreach, while
//!
//! Модуль содержит конвертеры для циклических конструкций.
//! Использует dispatcher для рекурсивной обработки тела цикла.

use crate::ast::{Expression, Statement};
use bsl_shared::ir::Span;
use tree_sitter::Node;

use crate::tree_sitter_adapter::expression_converter::convert_expression;
use crate::tree_sitter_adapter::span::{node_to_span_cached, LineIndex};
use crate::tree_sitter_adapter::utils::node_text;

/// Конвертировать for_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_for_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, line_index);
    let mut cursor = node.walk();
    let mut variable = String::new();
    let mut start = Expression::Number {
        value: 0.0,
        span: Span::stub(),
    };
    let mut end = Expression::Number {
        value: 0.0,
        span: Span::stub(),
    };
    let mut body = Vec::new();
    let mut in_body = false;
    let mut expr_count = 0;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if variable.is_empty() {
                    variable = node_text(&child, source)?;
                }
            }
            "DO_KEYWORD" | "ЦИКЛ_KEYWORD" => {
                in_body = true;
            }
            "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" => {
                break;
            }
            _ if child.kind().contains("expression")
                || child.kind() == "const_expression"
                || child.kind() == "number" =>
            {
                if !in_body {
                    if let Some(expr) = convert_expression(&child, source)? {
                        if expr_count == 0 {
                            start = expr;
                            expr_count += 1;
                        } else if expr_count == 1 {
                            end = expr;
                            expr_count += 1;
                        }
                    }
                }
            }
            _ => {
                if in_body {
                    if let Some(stmt) =
                        super::dispatch_statement_cached(&child, source, line_index)?
                    {
                        body.push(stmt);
                    }
                }
            }
        }
    }

    Ok(Statement::For {
        variable,
        start,
        end,
        body,
        span,
    })
}

/// Конвертировать for_each_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_for_each_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, line_index);
    let mut cursor = node.walk();
    let mut variable = String::new();
    let mut collection = Expression::Identifier {
        name: "unknown".to_string(),
        span: Span::stub(),
    };
    let mut body = Vec::new();
    let mut in_body = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if variable.is_empty() {
                    variable = node_text(&child, source)?;
                }
            }
            "DO_KEYWORD" | "ЦИКЛ_KEYWORD" => {
                in_body = true;
            }
            "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" => {
                break;
            }
            _ if !in_body && child.kind().contains("expression") => {
                if let Some(expr) = convert_expression(&child, source)? {
                    collection = expr;
                }
            }
            _ => {
                if in_body {
                    if let Some(stmt) =
                        super::dispatch_statement_cached(&child, source, line_index)?
                    {
                        body.push(stmt);
                    }
                }
            }
        }
    }

    Ok(Statement::ForEach {
        variable,
        collection,
        body,
        span,
    })
}

/// Конвертировать while_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_while_statement_cached(
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
    let mut body = Vec::new();
    let mut in_body = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "WHILE_KEYWORD" | "ПОКА_KEYWORD" => {}
            "DO_KEYWORD" | "ЦИКЛ_KEYWORD" => {
                in_body = true;
            }
            "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" => {
                break;
            }
            _ if !in_body => {
                if let Some(expr) = convert_expression(&child, source)? {
                    condition = expr;
                }
            }
            _ => {
                if let Some(stmt) = super::dispatch_statement_cached(&child, source, line_index)? {
                    body.push(stmt);
                }
            }
        }
    }

    Ok(Statement::While {
        condition,
        body,
        span,
    })
}
