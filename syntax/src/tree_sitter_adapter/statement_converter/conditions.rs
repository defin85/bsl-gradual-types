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

struct ElseIfClause {
    condition: Expression,
    body: Vec<Statement>,
    elsif_kw_span: Option<Span>,
    then_kw_span: Option<Span>,
    span: Span,
}

fn convert_elseif_clause_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<ElseIfClause, String> {
    let span = node_to_span_cached(node, source, line_index);
    let mut cursor = node.walk();

    let mut condition = Expression::Boolean {
        value: true,
        span: Span::stub(),
    };
    let mut body = Vec::new();
    let mut in_body = false;
    let mut elsif_kw_span: Option<Span> = None;
    let mut then_kw_span: Option<Span> = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "ELSIF_KEYWORD" => {
                elsif_kw_span = Some(node_to_span_cached(&child, source, line_index));
            }
            "THEN_KEYWORD" => {
                then_kw_span = Some(node_to_span_cached(&child, source, line_index));
                in_body = true;
            }
            "expression" => {
                if !in_body {
                    if let Some(expr) = convert_expression(&child, source)? {
                        condition = expr;
                    }
                }
            }
            _ => {
                if in_body {
                    if let Some(stmt) = super::dispatch_statement_cached(&child, source, line_index)?
                    {
                        body.push(stmt);
                    }
                }
            }
        }
    }

    Ok(ElseIfClause {
        condition,
        body,
        elsif_kw_span,
        then_kw_span,
        span,
    })
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
    let mut else_clause_body: Option<Vec<Statement>> = None;
    let mut elseifs: Vec<ElseIfClause> = Vec::new();
    let mut in_then = false;
    let mut if_kw_span: Option<Span> = None;
    let mut then_kw_span: Option<Span> = None;
    let mut endif_kw_span: Option<Span> = None;
    let mut else_branch_kw_span: Option<Span> = None;
    let mut else_kw_span: Option<Span> = None;

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
                let kw = keyword_span_in_clause_cached(&child, source, line_index, "ELSE_KEYWORD");
                else_kw_span = kw;
                if else_branch_kw_span.is_none() {
                    else_branch_kw_span = kw;
                }
                let else_statements = convert_clause_body_cached(&child, source, line_index)?;
                else_clause_body = Some(else_statements);
            }
            "elseif_clause" => {
                let clause = convert_elseif_clause_cached(&child, source, line_index)?;
                if else_branch_kw_span.is_none() {
                    else_branch_kw_span = clause.elsif_kw_span;
                }
                elseifs.push(clause);
            }
            kind if in_then && (kind.ends_with("_statement") || kind.ends_with("_definition")) => {
                if let Some(stmt) = super::dispatch_statement_cached(&child, source, line_index)? {
                    then_body.push(stmt);
                }
            }
            _ => {}
        }
    }

    let else_body = if elseifs.is_empty() {
        else_clause_body
    } else {
        let mut tail = else_clause_body;
        let mut next_clause_kw_span = else_kw_span.or(endif_kw_span);

        while let Some(clause) = elseifs.pop() {
            let header_span = match (clause.elsif_kw_span, clause.then_kw_span) {
                (Some(elsif_kw), Some(then_kw)) => Some(span_from_bounds(elsif_kw.end, then_kw.end)),
                _ => None,
            };

            let then_span = match (clause.then_kw_span, next_clause_kw_span, endif_kw_span) {
                (Some(then_kw), Some(next_kw), _) => Some(span_from_bounds(then_kw.end, next_kw.start)),
                (Some(then_kw), None, Some(endif_kw)) => Some(span_from_bounds(then_kw.end, endif_kw.start)),
                _ => None,
            };

            let else_span = if tail.is_some() {
                match (next_clause_kw_span, endif_kw_span) {
                    (Some(next_kw), Some(endif_kw)) => Some(span_from_bounds(next_kw.end, endif_kw.start)),
                    _ => None,
                }
            } else {
                None
            };

            let span = match (clause.elsif_kw_span, endif_kw_span) {
                (Some(elsif_kw), Some(endif_kw)) => span_from_bounds(elsif_kw.start, endif_kw.end),
                _ => clause.span,
            };

            let nested = Statement::If {
                condition: clause.condition,
                then_body: clause.body,
                else_body: tail,
                header_span,
                then_span,
                else_span,
                span,
            };

            tail = Some(vec![nested]);
            next_clause_kw_span = clause.elsif_kw_span;
        }

        tail
    };

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
/// Используется для обработки else_clause.
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
