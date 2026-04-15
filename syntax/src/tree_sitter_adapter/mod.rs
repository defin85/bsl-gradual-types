//! Адаптер для конвертации tree-sitter-bsl AST в доменный Program AST
//!
//! Преобразует узлы tree-sitter в структуры из backend/src/parsing/bsl/mod.rs
//!
//! # Архитектура модуля
//!
//! ```text
//! tree_sitter_adapter/
//! ├── mod.rs                  - Фасад (этот файл)
//! ├── utils.rs                - Вспомогательные функции
//! ├── span.rs                 - Конвертация позиций UTF-8 -> UTF-16
//! ├── directives.rs           - Парсинг директив компилятора
//! ├── expression_converter.rs - Конвертация выражений
//! ├── statement_converter/    - Конвертация statements (модуль)
//! └── syntax_errors.rs        - Сбор синтаксических ошибок
//! ```
//!
//! # Пример использования
//!
//! ```text
//! use tree_sitter::Parser;
//! use crate::system::tree_sitter_adapter::TreeSitterAdapter;
//!
//! let mut parser = Parser::new();
//! parser.set_language(tree_sitter_bsl::language()).unwrap();
//!
//! let source = "Процедура Тест() КонецПроцедуры";
//! let tree = parser.parse(source, None).unwrap();
//!
//! let result = TreeSitterAdapter::convert_tree(&tree, source).unwrap();
//! ```

pub mod directives;
mod expression_converter;
pub mod span;
mod statement_converter;
mod syntax_error_enhancers;
mod syntax_errors;
pub mod utils;

use crate::ast::{ParseResult, Program, Statement};
use bsl_shared::domain::types::ParseError;
use span::LineIndex;
use tree_sitter::Tree;

// Re-exports for external use
pub use syntax_errors::{collect_syntax_errors, collect_syntax_errors_cached};

/// Адаптер tree-sitter AST -> Program AST
pub struct TreeSitterAdapter;

impl TreeSitterAdapter {
    pub fn collect_syntax_errors_only(tree: &Tree, source: &str) -> Vec<ParseError> {
        let root = tree.root_node();
        let line_index = LineIndex::new(source);
        let (maybe_semicolon_errors, maybe_incomplete_new_errors) = if root.has_error() {
            (None, None)
        } else {
            let has_missing_semicolons = syntax_errors::has_missing_semicolons(&root);
            let incomplete_new_errors =
                syntax_errors::check_incomplete_new_expressions(source, &line_index);

            if !has_missing_semicolons && incomplete_new_errors.is_empty() {
                return Vec::new();
            }

            let semicolon_errors = if has_missing_semicolons {
                None
            } else {
                Some(Vec::new())
            };

            (semicolon_errors, Some(incomplete_new_errors))
        };

        let parser_errors = syntax_errors::collect_syntax_errors_cached(&root, source, &line_index);
        let mut heuristic_errors = maybe_semicolon_errors
            .unwrap_or_else(|| syntax_errors::check_missing_semicolons(&root, source, &line_index));
        let new_errors = maybe_incomplete_new_errors.unwrap_or_else(|| {
            syntax_errors::check_incomplete_new_expressions(source, &line_index)
        });
        heuristic_errors.extend(new_errors);

        if parser_errors.is_empty() && heuristic_errors.is_empty() {
            Vec::new()
        } else {
            syntax_error_enhancers::normalize_syntax_errors(
                source,
                &line_index,
                parser_errors,
                heuristic_errors,
            )
        }
    }

    /// Конвертировать дерево tree-sitter в ParseResult с обработкой ошибок (Milestone 2.7 Task 3)
    ///
    /// # Performance Optimization (Milestone 2.19)
    ///
    /// Предпросчитывает все строки файла **один раз** для избежания O(n^2) итераций
    /// при конвертации узлов AST в Span координаты.
    ///
    /// # Arguments
    ///
    /// * `tree` - Дерево tree-sitter для конвертации
    /// * `source` - Исходный код BSL файла
    ///
    /// # Returns
    ///
    /// * `Ok(ParseResult)` - Результат парсинга с программой и возможными ошибками
    /// * `Err(String)` - Критическая ошибка конвертации
    ///
    /// # Example
    ///
    /// ```text
    /// let result = TreeSitterAdapter::convert_tree(&tree, source)?;
    /// if result.has_errors() {
    ///     for error in result.errors() {
    ///         eprintln!("Error: {}", error.message);
    ///     }
    /// }
    /// ```
    pub fn convert_tree(tree: &Tree, source: &str) -> Result<ParseResult, String> {
        let root = tree.root_node();

        // Milestone 2.19+: Предпросчитываем индекс строк один раз (O(n))
        let line_index = LineIndex::new(source);
        let (maybe_semicolon_errors, maybe_incomplete_new_errors) = if root.has_error() {
            (None, None)
        } else {
            let has_missing_semicolons = syntax_errors::has_missing_semicolons(&root);
            let incomplete_new_errors =
                syntax_errors::check_incomplete_new_expressions(source, &line_index);

            if !has_missing_semicolons && incomplete_new_errors.is_empty() {
                let statements =
                    statement_converter::convert_source_file_cached(&root, source, &line_index)?;
                let program = Program { statements };
                return Ok(ParseResult::success(program));
            }

            let semicolon_errors = if has_missing_semicolons {
                None
            } else {
                Some(Vec::new())
            };

            (semicolon_errors, Some(incomplete_new_errors))
        };

        let parser_errors = syntax_errors::collect_syntax_errors_cached(&root, source, &line_index);
        let mut heuristic_errors = maybe_semicolon_errors
            .unwrap_or_else(|| syntax_errors::check_missing_semicolons(&root, source, &line_index));
        let new_errors = maybe_incomplete_new_errors.unwrap_or_else(|| {
            syntax_errors::check_incomplete_new_expressions(source, &line_index)
        });
        heuristic_errors.extend(new_errors);

        let syntax_errors = if parser_errors.is_empty() && heuristic_errors.is_empty() {
            Vec::new()
        } else {
            syntax_error_enhancers::normalize_syntax_errors(
                source,
                &line_index,
                parser_errors,
                heuristic_errors,
            )
        };

        // Пытаемся извлечь statements даже при наличии ошибок (partial recovery)
        let statements =
            statement_converter::convert_source_file_cached(&root, source, &line_index)?;
        let program = Program { statements };

        // Возвращаем ParseResult с программой и ошибками
        if syntax_errors.is_empty() {
            Ok(ParseResult::success(program))
        } else {
            Ok(ParseResult::with_errors(program, syntax_errors))
        }
    }

    /// Быстрый путь для индексации (без диагностики/линтера).
    ///
    /// Используется в индексации BSL модулей, где нужны только statements.
    pub fn convert_tree_fast(tree: &Tree, source: &str) -> Result<ParseResult, String> {
        let root = tree.root_node();
        let line_index = LineIndex::new(source);
        let statements =
            statement_converter::convert_source_file_cached(&root, source, &line_index)?;
        let program = Program { statements };
        Ok(ParseResult::success(program))
    }

    /// Быстрый путь для индексации с прогрессом по реальным lowering units.
    pub fn convert_tree_fast_with_progress(
        tree: &Tree,
        source: &str,
        mut progress: impl FnMut(usize, usize),
    ) -> Result<ParseResult, String> {
        Self::convert_tree_fast_with_observer(tree, source, |processed, total| {
            progress(processed, total);
            Ok(())
        })
    }

    /// Быстрый путь для индексации с progress/cancellation observer.
    pub fn convert_tree_fast_with_observer(
        tree: &Tree,
        source: &str,
        mut observer: impl FnMut(usize, usize) -> Result<(), String>,
    ) -> Result<ParseResult, String> {
        let root = tree.root_node();
        let line_index = LineIndex::new(source);
        let statements = statement_converter::convert_source_file_cached_with_observer(
            &root,
            source,
            &line_index,
            &mut observer,
        )?;
        let program = Program { statements };
        Ok(ParseResult::success(program))
    }

    pub fn convert_tree_fast_with_observer_and_reused_prefix(
        tree: &Tree,
        source: &str,
        reused_prefix: &[Statement],
        mut observer: impl FnMut(usize, usize) -> Result<(), String>,
    ) -> Result<ParseResult, String> {
        let root = tree.root_node();
        let line_index = LineIndex::new(source);
        let statements =
            statement_converter::convert_source_file_cached_with_observer_and_reused_prefix(
                &root,
                source,
                &line_index,
                reused_prefix,
                &mut observer,
            )?;
        let program = Program { statements };
        Ok(ParseResult::success(program))
    }
}
