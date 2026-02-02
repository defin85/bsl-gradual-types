//! Обработка исключений: try/except, raise
//!
//! Модуль содержит конвертеры для конструкций обработки ошибок.
//! Использует dispatcher для рекурсивной обработки блоков try/except.

use crate::ast::Statement;
use bsl_shared::ir::Span;
use tree_sitter::Node;

use crate::tree_sitter_adapter::expression_converter::convert_expression;
use crate::tree_sitter_adapter::span::{node_to_span_cached, LineIndex};

fn span_from_bounds(start: u32, end: u32) -> Span {
    Span { start, end }
}

/// Конвертировать try_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_try_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, line_index);
    let mut cursor = node.walk();
    let mut try_body = Vec::new();
    let mut except_body = Vec::new();
    let mut in_except = false;
    let mut try_kw_span: Option<Span> = None;
    let mut except_kw_span: Option<Span> = None;
    let mut endtry_kw_span: Option<Span> = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "TRY_KEYWORD" | "ПОПЫТКА_KEYWORD" => {
                try_kw_span = Some(node_to_span_cached(&child, source, line_index));
            }
            "ENDTRY_KEYWORD" | "КОНЕЦПОПЫТКИ_KEYWORD" => {
                endtry_kw_span = Some(node_to_span_cached(&child, source, line_index));
            }
            "EXCEPT_KEYWORD" | "ИСКЛЮЧЕНИЕ_KEYWORD" => {
                except_kw_span = Some(node_to_span_cached(&child, source, line_index));
                in_except = true;
            }
            _ => {
                if let Some(stmt) = super::dispatch_statement_cached(&child, source, line_index)? {
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
        header_span: try_kw_span,
        try_span: match (try_kw_span, except_kw_span) {
            (Some(try_kw), Some(except_kw)) => Some(span_from_bounds(try_kw.end, except_kw.start)),
            _ => None,
        },
        except_span: match (except_kw_span, endtry_kw_span) {
            (Some(except_kw), Some(endtry_kw)) => {
                Some(span_from_bounds(except_kw.end, endtry_kw.start))
            }
            _ => None,
        },
        span,
    })
}

/// Конвертировать raise_error_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_raise_error_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, line_index);
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
