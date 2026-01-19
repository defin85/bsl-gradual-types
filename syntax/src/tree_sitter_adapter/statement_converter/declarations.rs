//! Объявления: function_definition, var_definition
//!
//! Модуль содержит конвертеры для объявлений функций, процедур и переменных.
//! Использует dispatcher для рекурсивной обработки тела функций.

use crate::ast::Statement;
use tree_sitter::Node;

use crate::tree_sitter_adapter::directives::find_preceding_directive;
use crate::tree_sitter_adapter::span::{node_to_span_cached, LineIndex};
use crate::tree_sitter_adapter::utils::{convert_parameters, node_text};

/// Конвертировать function_definition с использованием кеша строк (Milestone 2.19)
///
/// Вызывает dispatcher для рекурсивной обработки тела функции/процедуры.
pub(crate) fn convert_function_definition_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, line_index);
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Vec::new();
    let is_procedure = node.kind() == "procedure_definition";
    let mut is_export = false;

    // Ищем директиву компилятора перед функцией/процедурой
    let compiler_directive = find_preceding_directive(node, source);

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if name.is_empty() {
                    name = node_text(&child, source);
                }
            }
            "parameters" => {
                params = convert_parameters(&child, source)?;
            }
            _ if child.kind().ends_with("_KEYWORD") => {
                let kw = node_text(&child, source).trim().to_lowercase();
                if kw == "экспорт" || kw == "export" {
                    is_export = true;
                }
            }
            _ => {
                // Собираем тело функции через dispatcher
                if let Some(stmt) = super::dispatch_statement_cached(&child, source, line_index)? {
                    body.push(stmt);
                }
            }
        }
    }

    if is_procedure {
        Ok(Statement::ProcedureDecl {
            name,
            params,
            body,
            compiler_directive,
            is_export,
            span,
        })
    } else {
        Ok(Statement::FunctionDecl {
            name,
            params,
            body,
            compiler_directive,
            is_export,
            span,
        })
    }
}

/// Конвертировать var_definition с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_var_definition_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut name = String::new();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            name = node_text(&child, source);
            break;
        }
    }

    Ok(Statement::VarDeclaration {
        name,
        type_hint: None,
        span: node_to_span_cached(node, source, line_index),
    })
}
