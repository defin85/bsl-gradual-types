//! Конвертация tree-sitter statement узлов в BSL Statement
//!
//! Этот модуль содержит логику преобразования различных типов statements:
//! - Объявления функций и процедур
//! - Объявления переменных
//! - Управляющие конструкции (if, for, while, try)
//! - Присваивания
//! - Вызовы процедур
//! - И другие statements языка 1С
//!
//! # Архитектура модуля
//!
//! ```text
//! statement_converter/
//! ├── mod.rs           - Entry points + dispatcher (этот файл)
//! ├── declarations.rs  - function_definition, var_definition
//! ├── loops.rs         - for, foreach, while
//! ├── conditions.rs    - if/elseif/else
//! ├── exceptions.rs    - try/except, raise
//! ├── simple.rs        - assignment, return, call
//! ├── special.rs       - goto, label, execute
//! └── handlers.rs      - add/remove_handler, await
//! ```
//!
//! # Паттерн Dispatcher
//!
//! Dispatcher (`dispatch_statement_cached`) централизует маршрутизацию по типу узла
//! и позволяет подмодулям вызывать его для рекурсивной обработки вложенных statements.

mod conditions;
mod declarations;
mod exceptions;
mod handlers;
mod loops;
mod simple;
mod special;

use crate::parsing::bsl::ast::Statement;
use tracing::debug;
use tree_sitter::Node;

use super::span::{node_to_span_cached, LineIndex};

// ============================================================
// Entry Points (public API)
// ============================================================

/// Конвертировать source_file (корневой узел) с использованием кеша строк (Milestone 2.19)
pub fn convert_source_file_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(stmt) = dispatch_statement_cached(&child, source, line_index)? {
            statements.push(stmt);
        }
    }

    Ok(statements)
}

/// Конвертировать source_file с прогрессом по количеству детей корневого узла.
pub fn convert_source_file_cached_with_progress(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    mut progress: impl FnMut(usize, usize),
) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut cursor = node.walk();
    let total = node.child_count();
    let mut processed = 0usize;

    for child in node.children(&mut cursor) {
        processed = processed.saturating_add(1);
        if processed == 1 || processed.is_multiple_of(1000) || processed == total {
            progress(processed, total);
        }
        if let Some(stmt) = dispatch_statement_cached(&child, source, line_index)? {
            statements.push(stmt);
        }
    }

    Ok(statements)
}

/// Конвертировать statement узел с использованием кеша строк (Milestone 2.19)
///
/// Публичная функция для обратной совместимости.
/// Делегирует работу dispatcher'у.
pub fn convert_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Option<Statement>, String> {
    dispatch_statement_cached(node, source, line_index)
}

// ============================================================
// Dispatcher (pub(crate) для вызова из подмодулей)
// ============================================================

/// Центральный dispatcher для маршрутизации statements по типу узла
///
/// Используется всеми подмодулями для рекурсивной обработки вложенных statements.
/// Это решает проблему циклических зависимостей между модулями.
pub(crate) fn dispatch_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Option<Statement>, String> {
    match node.kind() {
        // Declarations
        "function_definition" | "procedure_definition" => Ok(Some(
            declarations::convert_function_definition_cached(node, source, line_index)?,
        )),
        "var_definition" | "var_statement" => Ok(Some(
            declarations::convert_var_definition_cached(node, source, line_index)?,
        )),

        // Conditions
        "if_statement" => Ok(Some(conditions::convert_if_statement_cached(
            node, source, line_index,
        )?)),

        // Loops
        "for_statement" => Ok(Some(loops::convert_for_statement_cached(
            node, source, line_index,
        )?)),
        "for_each_statement" => Ok(Some(loops::convert_for_each_statement_cached(
            node, source, line_index,
        )?)),
        "while_statement" => Ok(Some(loops::convert_while_statement_cached(
            node, source, line_index,
        )?)),

        // Exceptions
        "try_statement" => Ok(Some(exceptions::convert_try_statement_cached(
            node, source, line_index,
        )?)),
        "rise_error_statement" => Ok(Some(exceptions::convert_raise_error_statement_cached(
            node, source, line_index,
        )?)),

        // Simple statements
        "assignment_statement" => Ok(Some(simple::convert_assignment_cached(
            node, source, line_index,
        )?)),
        "return_statement" => Ok(Some(simple::convert_return_cached(
            node, source, line_index,
        )?)),
        "call_statement" => Ok(Some(simple::convert_call_statement_cached(
            node, source, line_index,
        )?)),

        // Break/Continue (inline - too simple for separate module)
        "break_statement" => Ok(Some(Statement::Break {
            span: node_to_span_cached(node, source, line_index),
        })),
        "continue_statement" => Ok(Some(Statement::Continue {
            span: node_to_span_cached(node, source, line_index),
        })),

        // Special statements
        "goto_statement" => Ok(Some(special::convert_goto_statement_cached(
            node, source, line_index,
        )?)),
        "label_statement" => Ok(Some(special::convert_label_statement_cached(
            node, source, line_index,
        )?)),
        "execute_statement" => Ok(Some(special::convert_execute_statement_cached(
            node, source, line_index,
        )?)),

        // Event handlers
        "add_handler_statement" => Ok(Some(handlers::convert_add_handler_statement_cached(
            node, source, line_index,
        )?)),
        "remove_handler_statement" => Ok(Some(handlers::convert_remove_handler_statement_cached(
            node, source, line_index,
        )?)),
        "await_statement" => Ok(Some(handlers::convert_await_statement_cached(
            node, source, line_index,
        )?)),

        // Skip preprocessor and comments
        "preprocessor" | "comment" | "line_comment" => Ok(None),

        // Unknown nodes - skip with debug log
        _ => {
            debug!(
                "Skipping unknown statement type: {} at {}",
                node.kind(),
                node.start_position().row
            );
            Ok(None)
        }
    }
}

// ============================================================
// Backward Compatibility (non-cached versions)
// ============================================================

/// Конвертировать source_file (корневой узел)
///
/// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
/// Для производительности используйте `convert_source_file_cached()` вместо него.
#[allow(dead_code)]
pub fn convert_source_file(node: &Node, source: &str) -> Result<Vec<Statement>, String> {
    let line_index = LineIndex::new(source);
    convert_source_file_cached(node, source, &line_index)
}

/// Конвертировать statement узел
///
/// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
/// Для производительности используйте `convert_statement_cached()` вместо него.
#[allow(dead_code)]
pub fn convert_statement(node: &Node, source: &str) -> Result<Option<Statement>, String> {
    let line_index = LineIndex::new(source);
    convert_statement_cached(node, source, &line_index)
}
