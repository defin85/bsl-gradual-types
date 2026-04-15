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

use crate::ast::Statement;
use tracing::debug;
use tree_sitter::Node;

use super::span::{node_to_span_cached, LineIndex};

type LoweringObserver<'a> = dyn FnMut(usize, usize) -> Result<(), String> + 'a;

const LOWERING_PROGRESS_BATCH_UNITS: usize = 64;

pub(crate) struct LoweringProgressState {
    enabled: bool,
    processed_units: usize,
    total_units: usize,
    last_emitted_units: usize,
}

impl LoweringProgressState {
    fn disabled() -> Self {
        Self {
            enabled: false,
            processed_units: 0,
            total_units: 0,
            last_emitted_units: 0,
        }
    }

    fn with_total_hint(total_hint: usize) -> Self {
        Self {
            enabled: true,
            processed_units: 0,
            total_units: total_hint.max(1),
            last_emitted_units: 0,
        }
    }

    fn observe_unit(&mut self, observer: &mut LoweringObserver<'_>) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        self.processed_units = self.processed_units.saturating_add(1);
        self.total_units = self.total_units.max(self.processed_units);

        if self.processed_units == 1
            || self.processed_units.saturating_sub(self.last_emitted_units)
                >= LOWERING_PROGRESS_BATCH_UNITS
        {
            self.emit(observer)?;
        }

        Ok(())
    }

    fn finish(&mut self, observer: &mut LoweringObserver<'_>) -> Result<(), String> {
        if !self.enabled || self.processed_units == 0 {
            return Ok(());
        }

        self.total_units = self.total_units.max(self.processed_units);
        if self.last_emitted_units != self.processed_units {
            self.emit(observer)?;
        }

        Ok(())
    }

    fn emit(&mut self, observer: &mut LoweringObserver<'_>) -> Result<(), String> {
        observer(self.processed_units, self.total_units)?;
        self.last_emitted_units = self.processed_units;
        Ok(())
    }
}

fn is_lowering_progress_unit(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition"
            | "procedure_definition"
            | "var_definition"
            | "var_statement"
            | "if_statement"
            | "for_statement"
            | "for_each_statement"
            | "while_statement"
            | "try_statement"
            | "rise_error_statement"
            | "assignment_statement"
            | "return_statement"
            | "call_statement"
            | "break_statement"
            | "continue_statement"
            | "goto_statement"
            | "label_statement"
            | "execute_statement"
            | "add_handler_statement"
            | "remove_handler_statement"
            | "await_statement"
    )
}

// ============================================================
// Entry Points (public API)
// ============================================================

/// Конвертировать source_file (корневой узел) с использованием кеша строк (Milestone 2.19)
pub fn convert_source_file_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Vec<Statement>, String> {
    let mut progress = LoweringProgressState::disabled();
    let mut observer = |_, _| Ok(());
    convert_source_file_cached_internal(node, source, line_index, &mut progress, &mut observer)
}

/// Конвертировать source_file с progress/cancellation observer.
pub fn convert_source_file_cached_with_observer(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    mut observer: impl FnMut(usize, usize) -> Result<(), String>,
) -> Result<Vec<Statement>, String> {
    let mut progress = LoweringProgressState::with_total_hint(node.child_count() as usize);
    convert_source_file_cached_internal(node, source, line_index, &mut progress, &mut observer)
}

pub fn convert_source_file_cached_with_observer_and_reused_prefix(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    reused_prefix: &[Statement],
    mut observer: impl FnMut(usize, usize) -> Result<(), String>,
) -> Result<Vec<Statement>, String> {
    let mut progress = LoweringProgressState::with_total_hint(
        (node.child_count() as usize).saturating_sub(reused_prefix.len()),
    );
    let mut statements = Vec::with_capacity(
        reused_prefix
            .len()
            .saturating_add(node.child_count() as usize),
    );
    let mut reused_index = 0usize;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if reused_index < reused_prefix.len() && is_lowering_progress_unit(child.kind()) {
            statements.push(reused_prefix[reused_index].clone());
            reused_index = reused_index.saturating_add(1);
            continue;
        }
        if let Some(stmt) = dispatch_statement_cached_internal(
            &child,
            source,
            line_index,
            &mut progress,
            &mut observer,
        )? {
            statements.push(stmt);
        }
    }

    progress.finish(&mut observer)?;
    Ok(statements)
}

fn convert_source_file_cached_internal(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    progress: &mut LoweringProgressState,
    observer: &mut LoweringObserver<'_>,
) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(stmt) =
            dispatch_statement_cached_internal(&child, source, line_index, progress, observer)?
        {
            statements.push(stmt);
        }
    }

    progress.finish(observer)?;
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
    let mut progress = LoweringProgressState::disabled();
    let mut observer = |_, _| Ok(());
    dispatch_statement_cached_internal(node, source, line_index, &mut progress, &mut observer)
}

// ============================================================
// Dispatcher (pub(crate) для вызова из подмодулей)
// ============================================================

/// Центральный dispatcher для маршрутизации statements по типу узла
///
/// Используется всеми подмодулями для рекурсивной обработки вложенных statements.
/// Это решает проблему циклических зависимостей между модулями.
#[allow(dead_code)]
pub(crate) fn dispatch_statement_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Result<Option<Statement>, String> {
    let mut progress = LoweringProgressState::disabled();
    let mut observer = |_, _| Ok(());
    dispatch_statement_cached_internal(node, source, line_index, &mut progress, &mut observer)
}

pub(crate) fn dispatch_statement_cached_internal(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    progress: &mut LoweringProgressState,
    observer: &mut LoweringObserver<'_>,
) -> Result<Option<Statement>, String> {
    if is_lowering_progress_unit(node.kind()) {
        progress.observe_unit(observer)?;
    }

    match node.kind() {
        // Declarations
        "function_definition" | "procedure_definition" => {
            Ok(Some(declarations::convert_function_definition_cached(
                node, source, line_index, progress, observer,
            )?))
        }
        "var_definition" | "var_statement" => Ok(Some(
            declarations::convert_var_definition_cached(node, source, line_index)?,
        )),

        // Conditions
        "if_statement" => Ok(Some(conditions::convert_if_statement_cached(
            node, source, line_index, progress, observer,
        )?)),

        // Loops
        "for_statement" => Ok(Some(loops::convert_for_statement_cached(
            node, source, line_index, progress, observer,
        )?)),
        "for_each_statement" => Ok(Some(loops::convert_for_each_statement_cached(
            node, source, line_index, progress, observer,
        )?)),
        "while_statement" => Ok(Some(loops::convert_while_statement_cached(
            node, source, line_index, progress, observer,
        )?)),

        // Exceptions
        "try_statement" => Ok(Some(exceptions::convert_try_statement_cached(
            node, source, line_index, progress, observer,
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
