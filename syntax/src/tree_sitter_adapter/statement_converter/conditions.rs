//! Условные конструкции: if/elseif/else
//!
//! Модуль содержит конвертеры для условных statements.
//! Использует dispatcher для рекурсивной обработки вложенных блоков.

use crate::ast::{Expression, Statement};
use bsl_shared::ir::Span;
use tree_sitter::Node;

use crate::tree_sitter_adapter::expression_converter::convert_expression;
use crate::tree_sitter_adapter::span::{node_to_span_cached, LineIndex};

fn span_from_bounds(start: u32, end: u32) -> Span {
    Span { start, end }
}

fn keyword_span_in_clause_cached(
    clause: &Node,
    source: &str,
    line_index: &LineIndex,
    keyword_kind: &str,
) -> Option<Span> {
    let mut cursor = clause.walk();
    let result = clause
        .children(&mut cursor)
        .find(|c| c.kind() == keyword_kind)
        .map(|kw| node_to_span_cached(&kw, source, line_index));
    result
}

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
    let mut if_kw_span: Option<Span> = None;
    let mut then_kw_span: Option<Span> = None;
    let mut endif_kw_span: Option<Span> = None;
    let mut else_branch_kw_span: Option<Span> = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "IF_KEYWORD" => {
                if_kw_span = Some(node_to_span_cached(&child, source, line_index));
            }
            "THEN_KEYWORD" => {
                then_kw_span = Some(node_to_span_cached(&child, source, line_index));
                in_then = true;
            }
            "ENDIF_KEYWORD" => {
                endif_kw_span = Some(node_to_span_cached(&child, source, line_index));
                break;
            }
            "expression" => {
                if !in_then {
                    if let Some(expr) = convert_expression(&child, source)? {
                        condition = expr;
                    }
                }
            }
            "else_clause" => {
                if else_branch_kw_span.is_none() {
                    else_branch_kw_span =
                        keyword_span_in_clause_cached(&child, source, line_index, "ELSE_KEYWORD");
                }
                let else_statements = convert_clause_body_cached(&child, source, line_index)?;
                else_body = Some(else_statements);
            }
            "elseif_clause" => {
                if else_branch_kw_span.is_none() {
                    else_branch_kw_span =
                        keyword_span_in_clause_cached(&child, source, line_index, "ELSIF_KEYWORD");
                }
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

    let header_span = match (if_kw_span, then_kw_span) {
        (Some(if_kw), Some(then_kw)) => Some(span_from_bounds(if_kw.start, then_kw.end)),
        _ => None,
    };

    let then_span = match (then_kw_span, else_branch_kw_span, endif_kw_span) {
        (Some(then_kw), Some(else_kw), _) => Some(span_from_bounds(then_kw.end, else_kw.start)),
        (Some(then_kw), None, Some(endif_kw)) => Some(span_from_bounds(then_kw.end, endif_kw.start)),
        _ => None,
    };

    let else_span = match (else_branch_kw_span, endif_kw_span) {
        (Some(else_kw), Some(endif_kw)) => Some(span_from_bounds(else_kw.end, endif_kw.start)),
        _ => None,
    };

    Ok(Statement::If {
        condition,
        then_body,
        else_body,
        header_span,
        then_span,
        else_span,
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
