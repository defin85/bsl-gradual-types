//! Адаптер для конвертации tree-sitter-bsl AST в доменный Program AST
//!
//! Преобразует узлы tree-sitter в структуры из backend/src/parsing/bsl/mod.rs

use crate::parsing::bsl::ast::{Expression, ParseResult, Program, Statement};
use bsl_shared::domain::types::{ErrorType, ParseError};
use bsl_shared::ir::Span;
use tracing::debug;
use tree_sitter::{Node, Tree};

/// Адаптер tree-sitter AST → Program AST
pub struct TreeSitterAdapter;

impl TreeSitterAdapter {
    /// Конвертировать byte offset (UTF-8) в UTF-16 code units
    ///
    /// LSP использует UTF-16 code units для позиций, а tree-sitter использует byte offsets (UTF-8).
    /// Эта функция корректно преобразует byte offset в UTF-16 offset для кириллицы и других non-ASCII символов.
    ///
    /// # Milestone 2.18 Task 1: КРИТИЧНОЕ ИСПРАВЛЕНИЕ
    fn byte_offset_to_utf16(line: &str, byte_offset: usize) -> u32 {
        line[..byte_offset.min(line.len())]
            .chars()
            .map(|c| c.len_utf16() as u32)
            .sum()
    }

    /// Извлечь Span из tree-sitter Node с конвертацией в UTF-16 координаты
    ///
    /// # Milestone 2.18 Task 1: FIX UTF-16 координаты
    ///
    /// Tree-sitter возвращает позиции в byte offsets (UTF-8), но LSP требует UTF-16 code units.
    /// Без этой конвертации диагностики будут показываться на НЕПРАВИЛЬНЫХ позициях в файлах с кириллицей!
    ///
    /// Пример проблемы:
    /// ```bsl
    /// Функция Тест()  // "Функция" = 7 символов кириллицы
    ///     Перем Х;    // tree-sitter: column=14 (UTF-8 bytes), LSP нужно: 11 (UTF-16 code units)
    /// ```
    ///
    /// **ВАЖНО:** Этот метод делает O(n) итерацию по строкам для каждого узла.
    /// Для производительности используйте `node_to_span_cached()` вместо него.
    fn node_to_span(node: &Node, source: &str) -> Span {
        // Для обратной совместимости: предпросчитываем строки для этого вызова
        let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Self::node_to_span_cached(node, source, &lines)
    }

    /// Извлечь Span с использованием кеша строк (O(1) доступ вместо O(n))
    ///
    /// # Performance Optimization (Milestone 2.19)
    ///
    /// Эта версия использует предпросчитанный кеш строк для избежания O(n²) итераций.
    /// Для файла в 500 строк с 300 узлами:
    /// - **Было:** ~150,000 итераций по строкам (O(n) для каждого узла)
    /// - **Стало:** 500 итераций (O(1) доступ для каждого узла через кеш)
    fn node_to_span_cached(node: &Node, _source: &str, lines: &[String]) -> Span {
        let start_pos = node.start_position();
        let end_pos = node.end_position();

        // ✅ O(1) доступ вместо O(n) итерации через source.lines().nth()!
        let start_line_text = lines
            .get(start_pos.row)
            .map(|s| s.as_str())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Tree-sitter returned invalid start line: {} (file has {} lines)",
                    start_pos.row,
                    lines.len()
                );
                ""
            });

        let end_line_text = lines
            .get(end_pos.row)
            .map(|s| s.as_str())
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Tree-sitter returned invalid end line: {} (file has {} lines)",
                    end_pos.row,
                    lines.len()
                );
                ""
            });

        // ✅ MILESTONE 2.18: Конвертируем byte offsets → UTF-16 code units
        let start_column_utf16 = Self::byte_offset_to_utf16(start_line_text, start_pos.column);
        let end_column_utf16 = Self::byte_offset_to_utf16(end_line_text, end_pos.column);

        let span = Span::from_positions(
            (start_pos.row as u32, start_column_utf16),
            (end_pos.row as u32, end_column_utf16),
        );

        // Milestone 2.11 Task B1: DEBUG логи для Span extraction
        debug!(
            "Extracted Span (UTF-16): {}:{} - {}:{} (node kind: {}) [UTF-8 was: {}:{} - {}:{}]",
            span.start_line,
            span.start_column,
            span.end_line,
            span.end_column,
            node.kind(),
            start_pos.row,
            start_pos.column,
            end_pos.row,
            end_pos.column
        );

        span
    }

    /// Конвертировать дерево tree-sitter в ParseResult с обработкой ошибок (Milestone 2.7 Task 3)
    ///
    /// # Performance Optimization (Milestone 2.19)
    ///
    /// Предпросчитывает все строки файла **один раз** для избежания O(n²) итераций
    /// при конвертации узлов AST в Span координаты.
    pub fn convert_tree(tree: &Tree, source: &str) -> Result<ParseResult, String> {
        let root = tree.root_node();

        // ✅ Milestone 2.19: Предпросчитываем все строки один раз (O(n))
        let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();

        // Собираем синтаксические ошибки из дерева с использованием кеша строк
        let mut syntax_errors = Self::collect_syntax_errors_cached(&root, source, &lines);

        // ✅ Проверяем отсутствующие точки с запятой (BSL linter)
        let semicolon_errors = Self::check_missing_semicolons(&root, source, &lines);
        syntax_errors.extend(semicolon_errors);

        // Пытаемся извлечь statements даже при наличии ошибок (partial recovery)
        let statements = Self::convert_source_file_cached(&root, source, &lines)?;
        let program = Program { statements };

        // Возвращаем ParseResult с программой и ошибками
        if syntax_errors.is_empty() {
            Ok(ParseResult::success(program))
        } else {
            Ok(ParseResult::with_errors(program, syntax_errors))
        }
    }

    /// Собрать все ERROR узлы из дерева (рекурсивный обход)
    ///
    /// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
    /// Для производительности используйте `collect_syntax_errors_cached()` вместо него.
    #[allow(dead_code)]
    fn collect_syntax_errors(node: &Node, source: &str) -> Vec<ParseError> {
        // Для обратной совместимости: предпросчитываем строки
        let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Self::collect_syntax_errors_cached(node, source, &lines)
    }

    /// Собрать все ERROR узлы из дерева с использованием кеша строк (Milestone 2.19)
    fn collect_syntax_errors_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Vec<ParseError> {
        let mut errors = Vec::new();

        // Если текущий узел — ERROR, добавляем его
        if node.kind() == "ERROR" {
            let span = Self::node_to_span_cached(node, source, lines);
            let text = node
                .utf8_text(source.as_bytes())
                .unwrap_or("<неизвестно>")
                .to_string();

            errors.push(ParseError {
                message: format!("Синтаксическая ошибка: неожиданный текст '{}'", text),
                span,
                error_type: ErrorType::ParseError,
            });
        }

        // Проверяем node.is_missing() для пропущенных токенов
        if node.is_missing() {
            let span = Self::node_to_span_cached(node, source, lines);
            errors.push(ParseError {
                message: format!("Отсутствует обязательный элемент: {}", node.kind()),
                span,
                error_type: ErrorType::MissingToken,
            });
        }

        // Рекурсивно обходим дочерние узлы
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            errors.extend(Self::collect_syntax_errors_cached(&child, source, lines));
        }

        errors
    }

    /// Проверить отсутствующие точки с запятой между statements
    ///
    /// В BSL точка с запятой ОБЯЗАТЕЛЬНА между операторами, кроме последнего оператора
    /// перед закрывающим ключевым словом (КонецФункции, КонецПроцедуры).
    fn check_missing_semicolons(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Vec<ParseError> {
        let mut errors = Vec::new();

        // Проверяем только тела функций и процедур
        if matches!(node.kind(), "function_definition" | "procedure_definition") {
            errors.extend(Self::check_function_body_semicolons(node, source, lines));
        }

        // Рекурсивно проверяем вложенные узлы (для вложенных конструкций)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            errors.extend(Self::check_missing_semicolons(&child, source, lines));
        }

        errors
    }

    /// Проверить точки с запятой в теле функции/процедуры
    fn check_function_body_semicolons(
        func_node: &Node,
        source: &str,
        lines: &[String],
    ) -> Vec<ParseError> {
        let mut errors = Vec::new();
        let mut cursor = func_node.walk();

        // Собираем все statement узлы в теле функции
        let mut statements: Vec<Node> = Vec::new();
        let mut found_end_keyword = false;

        for child in func_node.children(&mut cursor) {
            match child.kind() {
                // Statements, которые должны заканчиваться точкой с запятой (если не последние)
                "if_statement" | "while_statement" | "for_statement" | "for_each_statement"
                | "assignment_statement" | "call_statement" | "return_statement"
                | "break_statement" | "continue_statement" | "var_statement" | "try_statement" => {
                    statements.push(child);
                }
                // Конец функции/процедуры
                "ENDFUNCTION_KEYWORD" | "ENDPROCEDURE_KEYWORD" => {
                    found_end_keyword = true;
                }
                _ => {}
            }
        }

        // Проверяем каждый statement (кроме последнего)
        for (i, stmt) in statements.iter().enumerate() {
            let is_last = i == statements.len() - 1;

            // Последний statement перед КонецФункции может не иметь точку с запятой
            if is_last && found_end_keyword {
                continue;
            }

            // Проверяем наличие точки с запятой после statement
            if !Self::has_semicolon_child(stmt) {
                let span = Self::node_to_span_cached(stmt, source, lines);

                // Позиция для диагностики - конец statement
                let error_span = Span::from_positions(
                    (span.end_line, span.end_column),
                    (span.end_line, span.end_column),
                );

                errors.push(ParseError {
                    message: format!(
                        "Отсутствует точка с запятой после оператора '{}'",
                        stmt.kind().replace("_statement", "")
                    ),
                    span: error_span,
                    error_type: ErrorType::MissingToken,
                });
            }
        }

        errors
    }

    /// Проверить наличие точки с запятой как дочернего узла
    fn has_semicolon_child(node: &Node) -> bool {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == ";" {
                return true;
            }
        }

        false
    }

    /// Конвертировать source_file (корневой узел)
    ///
    /// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
    /// Для производительности используйте `convert_source_file_cached()` вместо него.
    #[allow(dead_code)]
    fn convert_source_file(node: &Node, source: &str) -> Result<Vec<Statement>, String> {
        // Для обратной совместимости: предпросчитываем строки
        let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Self::convert_source_file_cached(node, source, &lines)
    }

    /// Конвертировать source_file с использованием кеша строк (Milestone 2.19)
    fn convert_source_file_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
                statements.push(stmt);
            }
        }

        Ok(statements)
    }

    /// Конвертировать statement узел
    ///
    /// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
    /// Для производительности используйте `convert_statement_cached()` вместо него.
    #[allow(dead_code)]
    fn convert_statement(node: &Node, source: &str) -> Result<Option<Statement>, String> {
        // Для обратной совместимости: предпросчитываем строки
        let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Self::convert_statement_cached(node, source, &lines)
    }

    /// Конвертировать statement узел с использованием кеша строк (Milestone 2.19)
    fn convert_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Option<Statement>, String> {
        match node.kind() {
            "function_definition" | "procedure_definition" => Ok(Some(
                Self::convert_function_definition_cached(node, source, lines)?,
            )),
            "var_definition" | "var_statement" => Ok(Some(Self::convert_var_definition_cached(
                node, source, lines,
            )?)),
            "if_statement" => Ok(Some(Self::convert_if_statement_cached(
                node, source, lines,
            )?)),
            "for_statement" => Ok(Some(Self::convert_for_statement_cached(
                node, source, lines,
            )?)),
            "for_each_statement" => Ok(Some(Self::convert_for_each_statement_cached(
                node, source, lines,
            )?)),
            "while_statement" => Ok(Some(Self::convert_while_statement_cached(
                node, source, lines,
            )?)),
            "try_statement" => Ok(Some(Self::convert_try_statement_cached(
                node, source, lines,
            )?)),
            "assignment_statement" => {
                Ok(Some(Self::convert_assignment_cached(node, source, lines)?))
            }
            "return_statement" => Ok(Some(Self::convert_return_cached(node, source, lines)?)),
            "call_statement" => Ok(Some(Self::convert_call_statement_cached(
                node, source, lines,
            )?)),
            "break_statement" => Ok(Some(Statement::Break {
                span: Self::node_to_span_cached(node, source, lines),
            })),
            "continue_statement" => Ok(Some(Statement::Continue {
                span: Self::node_to_span_cached(node, source, lines),
            })),
            "goto_statement" => Ok(Some(Self::convert_goto_statement_cached(
                node, source, lines,
            )?)),
            "label_statement" => Ok(Some(Self::convert_label_statement_cached(
                node, source, lines,
            )?)),
            "execute_statement" => Ok(Some(Self::convert_execute_statement_cached(
                node, source, lines,
            )?)),
            "rise_error_statement" => Ok(Some(Self::convert_raise_error_statement_cached(
                node, source, lines,
            )?)),
            "add_handler_statement" => Ok(Some(Self::convert_add_handler_statement_cached(
                node, source, lines,
            )?)),
            "remove_handler_statement" => Ok(Some(Self::convert_remove_handler_statement_cached(
                node, source, lines,
            )?)),
            "await_statement" => Ok(Some(Self::convert_await_statement_cached(
                node, source, lines,
            )?)),

            // Пропускаем препроцессор и комментарии
            "preprocessor" | "comment" | "line_comment" => Ok(None),

            // Неизвестные узлы пока пропускаем
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

    /// Конвертировать function_definition или procedure_definition
    #[allow(dead_code)]
    fn convert_function_definition(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut params = Vec::new();
        let mut body = Vec::new();
        let is_procedure = node.kind() == "procedure_definition";

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = Self::node_text(&child, source);
                    }
                }
                "parameters" => {
                    params = Self::convert_parameters(&child, source)?;
                }
                _ => {
                    // Собираем тело функции
                    if let Some(stmt) = Self::convert_statement(&child, source)? {
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
                span,
            })
        } else {
            Ok(Statement::FunctionDecl {
                name,
                params,
                body,
                span,
            })
        }
    }

    /// Конвертировать parameters
    fn convert_parameters(node: &Node, source: &str) -> Result<Vec<String>, String> {
        let mut params = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "parameter" {
                // parameter содержит identifier как дочерний узел
                let mut param_cursor = child.walk();
                for param_child in child.children(&mut param_cursor) {
                    if param_child.kind() == "identifier" {
                        params.push(Self::node_text(&param_child, source));
                        break; // Берём только имя параметра
                    }
                }
            }
        }

        Ok(params)
    }

    /// Конвертировать var_definition
    #[allow(dead_code)]
    fn convert_var_definition(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut name = String::new();

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                name = Self::node_text(&child, source);
                break;
            }
        }

        Ok(Statement::VarDeclaration {
            name,
            type_hint: None, // tree-sitter-bsl не поддерживает type hints
            span: Self::node_to_span(node, source),
        })
    }

    /// Конвертировать if_statement
    #[allow(dead_code)]
    fn convert_if_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut condition = Expression::Boolean {
            value: true,
            span: Span::stub(),
        }; // default
        let mut then_body = Vec::new();
        let mut else_body = None;

        let mut in_then = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "IF_KEYWORD" => {} // Пропускаем ключевое слово
                "THEN_KEYWORD" => in_then = true,
                "ENDIF_KEYWORD" => break,
                "expression" => {
                    // Условие if-а (до THEN)
                    if !in_then {
                        if let Some(expr) = Self::convert_expression(&child, source)? {
                            condition = expr;
                        }
                    }
                }
                "else_clause" => {
                    // Обработка else блока
                    let else_statements = Self::convert_clause_body(&child, source)?;
                    else_body = Some(else_statements);
                }
                "elseif_clause" => {
                    // Обработка elseif как вложенный if в else
                    // TODO: более корректная обработка цепочек elseif
                    let elseif_statements = Self::convert_clause_body(&child, source)?;
                    else_body = Some(elseif_statements);
                }
                // Любые statement узлы в then-блоке
                kind if in_then
                    && (kind.ends_with("_statement") || kind.ends_with("_definition")) =>
                {
                    if let Some(stmt) = Self::convert_statement(&child, source)? {
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

    /// Конвертировать тело clause (else_clause, elseif_clause)
    #[allow(dead_code)]
    fn convert_clause_body(node: &Node, source: &str) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind() == "ELSE_KEYWORD" || child.kind() == "ELSIF_KEYWORD" {
                continue;
            }

            if let Some(stmt) = Self::convert_statement(&child, source)? {
                statements.push(stmt);
            }
        }

        Ok(statements)
    }

    /// Конвертировать for_statement
    #[allow(dead_code)]
    fn convert_for_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
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
                        variable = Self::node_text(&child, source);
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
                        if let Some(expr) = Self::convert_expression(&child, source)? {
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
                        if let Some(stmt) = Self::convert_statement(&child, source)? {
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

    /// Конвертировать assignment_statement
    #[allow(dead_code)]
    fn convert_assignment(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut target = None;
        let mut value = None;

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "property_access" {
                if target.is_none() {
                    target = Self::convert_expression(&child, source)?;
                }
            } else if let Some(expr) = Self::convert_expression(&child, source)? {
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

    /// Конвертировать return_statement
    #[allow(dead_code)]
    fn convert_return(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut value = None;

        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind() == "RETURN_KEYWORD" || child.kind() == "ВОЗВРАТ_KEYWORD" {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                value = Some(expr);
                break;
            }
        }

        Ok(Statement::Return { value, span })
    }

    /// Конвертировать call_statement (вызов процедуры/функции)
    #[allow(dead_code)]
    fn convert_call_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Call {
                    expression: expr,
                    span,
                });
            }
        }

        Err("call_statement without expression".to_string())
    }

    /// Конвертировать while_statement
    #[allow(dead_code)]
    fn convert_while_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
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
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        condition = expr;
                    }
                }
                _ => {
                    if let Some(stmt) = Self::convert_statement(&child, source)? {
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

    /// Конвертировать try_statement
    #[allow(dead_code)]
    fn convert_try_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut try_body = Vec::new();
        let mut except_body = Vec::new();
        let mut in_except = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "TRY_KEYWORD" | "ПОПЫТКА_KEYWORD" | "ENDTRY_KEYWORD" | "КОНЕЦПОПЫТКИ_KEYWORD" =>
                    {}
                "EXCEPT_KEYWORD" | "ИСКЛЮЧЕНИЕ_KEYWORD" => {
                    in_except = true;
                }
                _ => {
                    if let Some(stmt) = Self::convert_statement(&child, source)? {
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
            span,
        })
    }

    /// Конвертировать for_each_statement
    #[allow(dead_code)]
    fn convert_for_each_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
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
                        variable = Self::node_text(&child, source);
                    }
                }
                "DO_KEYWORD" | "ЦИКЛ_KEYWORD" => {
                    in_body = true;
                }
                "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" => {
                    break;
                }
                _ if !in_body && child.kind().contains("expression") => {
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        collection = expr;
                    }
                }
                _ => {
                    if in_body {
                        if let Some(stmt) = Self::convert_statement(&child, source)? {
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

    /// Конвертировать goto_statement
    #[allow(dead_code)]
    fn convert_goto_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let label = Self::node_text(&child, source);
                return Ok(Statement::Goto { label, span });
            }
        }
        Err("goto_statement without label".to_string())
    }

    /// Конвертировать label_statement
    #[allow(dead_code)]
    fn convert_label_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let name = Self::node_text(&child, source);
                return Ok(Statement::Label { name, span });
            }
        }
        Err("label_statement without name".to_string())
    }

    /// Конвертировать execute_statement
    #[allow(dead_code)]
    fn convert_execute_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Execute { code: expr, span });
            }
        }
        Err("execute_statement without code".to_string())
    }

    /// Конвертировать rise_error_statement
    #[allow(dead_code)]
    fn convert_raise_error_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut message = None;

        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                message = Some(expr);
                break;
            }
        }

        Ok(Statement::RaiseError { message, span })
    }

    /// Конвертировать add_handler_statement
    #[allow(dead_code)]
    fn convert_add_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let (event, handler) = Self::extract_event_handler_pair(node, source)?;
        Ok(Statement::AddHandler {
            event,
            handler,
            span,
        })
    }

    /// Конвертировать remove_handler_statement
    #[allow(dead_code)]
    fn convert_remove_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let (event, handler) = Self::extract_event_handler_pair(node, source)?;
        Ok(Statement::RemoveHandler {
            event,
            handler,
            span,
        })
    }

    /// Извлечь пару event-handler из узла
    fn extract_event_handler_pair(
        node: &Node,
        source: &str,
    ) -> Result<(Expression, Expression), String> {
        let mut cursor = node.walk();
        let mut event = None;
        let mut handler = None;

        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                if event.is_none() {
                    event = Some(expr);
                } else {
                    handler = Some(expr);
                    break;
                }
            }
        }

        match (event, handler) {
            (Some(e), Some(h)) => Ok((e, h)),
            _ => Err("handler statement requires event and handler".to_string()),
        }
    }

    /// Конвертировать await_statement
    #[allow(dead_code)]
    fn convert_await_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Await {
                    expression: expr,
                    span,
                });
            }
        }
        Err("await_statement without expression".to_string())
    }

    /// Конвертировать expression узел
    fn convert_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        match node.kind() {
            // Промежуточные узлы - рекурсивно обрабатываем дочерние
            "expression" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        return Ok(Some(expr));
                    }
                }
                Ok(None)
            }

            "const_expression" => {
                // Сначала пытаемся найти дочерний узел (number, string, boolean, date)
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        return Ok(Some(expr));
                    }
                }

                // Fallback: парсим текст напрямую
                let text = Self::node_text(node, source);
                let span = Self::node_to_span(node, source);
                if let Ok(num) = text.parse::<f64>() {
                    Ok(Some(Expression::Number { value: num, span }))
                } else if text.eq_ignore_ascii_case("истина")
                    || text.eq_ignore_ascii_case("true")
                {
                    Ok(Some(Expression::Boolean { value: true, span }))
                } else if text.eq_ignore_ascii_case("ложь")
                    || text.eq_ignore_ascii_case("false")
                {
                    Ok(Some(Expression::Boolean { value: false, span }))
                } else {
                    Ok(Some(Expression::String { value: text, span }))
                }
            }

            "identifier" => Ok(Some(Expression::Identifier {
                name: Self::node_text(node, source),
                span: Self::node_to_span(node, source),
            })),

            "number" => {
                let text = Self::node_text(node, source);
                let span = Self::node_to_span(node, source);
                if let Ok(num) = text.parse::<f64>() {
                    Ok(Some(Expression::Number { value: num, span }))
                } else {
                    Ok(Some(Expression::String { value: text, span }))
                }
            }

            "string" => {
                let span = Self::node_to_span(node, source);
                // string может содержать дочерние узлы (string_content)
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "string_content" {
                        return Ok(Some(Expression::String {
                            value: Self::node_text(&child, source),
                            span,
                        }));
                    }
                }

                // Fallback: берём весь текст и убираем кавычки
                let text = Self::node_text(node, source);
                let trimmed = text.trim_matches('"');
                Ok(Some(Expression::String {
                    value: trimmed.to_string(),
                    span,
                }))
            }

            "boolean" => {
                let span = Self::node_to_span(node, source);
                // boolean узел содержит дочерний TRUE_KEYWORD или FALSE_KEYWORD
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    let child_kind = child.kind();
                    if child_kind == "TRUE_KEYWORD" {
                        return Ok(Some(Expression::Boolean { value: true, span }));
                    } else if child_kind == "FALSE_KEYWORD" {
                        return Ok(Some(Expression::Boolean { value: false, span }));
                    }
                }

                // Fallback: парсим текст
                let text = Self::node_text(node, source);
                if text.eq_ignore_ascii_case("истина") || text.eq_ignore_ascii_case("true") {
                    Ok(Some(Expression::Boolean { value: true, span }))
                } else {
                    Ok(Some(Expression::Boolean { value: false, span }))
                }
            }

            "date" => {
                Ok(Some(Expression::Date {
                    value: Self::node_text(node, source),
                    span: Self::node_to_span(node, source),
                }))
            }

            "binary_expression" => Self::convert_binary_expression(node, source),

            "call_expression" | "method_call" => Self::convert_call_expression(node, source),

            "property_access" => Self::convert_property_access(node, source),

            "access" => {
                // access может быть частью property_access или index_access
                // Рекурсивно обрабатываем дочерние узлы
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        return Ok(Some(expr));
                    }
                }
                Ok(None)
            }

            "index" => Self::convert_index_access(node, source),

            "unary_expression" => Self::convert_unary_expression(node, source),

            "ternary_expression" => Self::convert_ternary_expression(node, source),

            "new_expression" | "new_expression_method" => Self::convert_new_expression(node, source),

            "await_expression" => Self::convert_await_expression(node, source),

            // Игнорируем ключевые слова, операторы и вспомогательные узлы
            kind if kind.ends_with("_KEYWORD")
                || kind == "="
                || kind == "("
                || kind == ")"
                || kind == ","
                || kind == ";"
                || kind == "."
                || kind == "string_content" // уже обработано в "string"
                => {
                Ok(None)
            }

            _ => {
                debug!("Skipping unknown expression: {}", node.kind());
                Ok(None)
            }
        }
    }

    /// Конвертировать binary_expression
    fn convert_binary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut left = None;
        let mut operator = String::new();
        let mut right = None;

        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                if left.is_none() {
                    left = Some(expr);
                } else {
                    right = Some(expr);
                }
            } else if child.kind() == "+" || child.kind() == "-" || child.kind() == "*" {
                operator = child.kind().to_string();
            }
        }

        if let (Some(l), Some(r)) = (left, right) {
            Ok(Some(Expression::Binary {
                left: Box::new(l),
                operator: operator.clone(),
                right: Box::new(r),
                span,
            }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать call_expression
    fn convert_call_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut function = None;
        let mut args = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" | "property_access" | "access" => {
                    if function.is_none() {
                        function = Self::convert_expression(&child, source)?;
                    }
                }
                "arguments" => {
                    // Парсим аргументы из узла arguments
                    let mut args_cursor = child.walk();
                    for arg_child in child.children(&mut args_cursor) {
                        if arg_child.kind() == "expression" {
                            if let Some(expr) = Self::convert_expression(&arg_child, source)? {
                                args.push(expr);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(func) = function {
            Ok(Some(Expression::Call {
                function: Box::new(func),
                args,
                span,
            }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать unary_expression
    fn convert_unary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut operator = String::new();
        let mut operand = None;

        for child in node.children(&mut cursor) {
            if child.kind() == "NOT_KEYWORD" || child.kind() == "-" {
                operator = child.kind().to_string();
            } else if let Some(expr) = Self::convert_expression(&child, source)? {
                operand = Some(expr);
            }
        }

        if let Some(op) = operand {
            Ok(Some(Expression::Unary {
                operator,
                operand: Box::new(op),
                span,
            }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать property_access
    fn convert_property_access(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut object = None;
        let mut property = String::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if object.is_none() {
                        let child_span = Self::node_to_span(&child, source);
                        object = Some(Expression::Identifier {
                            name: Self::node_text(&child, source),
                            span: child_span,
                        });
                    } else {
                        property = Self::node_text(&child, source);
                    }
                }
                "property_access" => {
                    // Вложенный property access (a.b.c)
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        object = Some(expr);
                    }
                }
                "property" => {
                    property = Self::node_text(&child, source);
                }
                _ => {}
            }
        }

        match object {
            Some(obj) if !property.is_empty() => Ok(Some(Expression::PropertyAccess {
                object: Box::new(obj),
                property,
                span,
            })),
            _ => {
                // Fallback: возвращаем как Identifier
                Ok(Some(Expression::Identifier {
                    name: Self::node_text(node, source),
                    span,
                }))
            }
        }
    }

    /// Конвертировать index_access (доступ по индексу: arr[0])
    fn convert_index_access(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut object = None;
        let mut index = None;

        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                if object.is_none() {
                    object = Some(expr);
                } else {
                    index = Some(expr);
                }
            }
        }

        match (object, index) {
            (Some(obj), Some(idx)) => Ok(Some(Expression::IndexAccess {
                object: Box::new(obj),
                index: Box::new(idx),
                span,
            })),
            _ => Ok(None),
        }
    }

    /// Конвертировать ternary_expression
    fn convert_ternary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut condition = None;
        let mut then_expr = None;
        let mut else_expr = None;

        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                if condition.is_none() {
                    condition = Some(expr);
                } else if then_expr.is_none() {
                    then_expr = Some(expr);
                } else {
                    else_expr = Some(expr);
                }
            }
        }

        match (condition, then_expr, else_expr) {
            (Some(c), Some(t), Some(e)) => Ok(Some(Expression::Ternary {
                condition: Box::new(c),
                then_expr: Box::new(t),
                else_expr: Box::new(e),
                span,
            })),
            _ => Ok(None),
        }
    }

    /// Конвертировать new_expression
    fn convert_new_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        let mut type_expr = None;
        let mut args = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "NEW_KEYWORD" | "НОВЫЙ_KEYWORD" => {}
                "identifier" | "property_access" => {
                    // Новый Тип или Новый Модуль.Тип - прямой identifier
                    if type_expr.is_none() {
                        type_expr = Some(Self::node_text(&child, source));
                    }
                }
                "arguments" => {
                    // Новый(Выражение) - expression внутри arguments
                    // Парсим аргументы и берём первый как type_expr
                    let mut arg_cursor = child.walk();
                    for arg_child in child.children(&mut arg_cursor) {
                        if arg_child.kind() == "expression" {
                            if let Some(expr) = Self::convert_expression(&arg_child, source)? {
                                if type_expr.is_none() {
                                    // Первое выражение - это тип для конструктора
                                    // Сохраняем как строку из исходного кода
                                    type_expr = Some(Self::node_text(&arg_child, source));
                                } else {
                                    // Остальные выражения - аргументы конструктора
                                    args.push(expr);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(type_name) = type_expr {
            Ok(Some(Expression::New {
                type_name,
                args,
                span,
            }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать await_expression
    fn convert_await_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let span = Self::node_to_span(node, source);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Some(Expression::Await {
                    expression: Box::new(expr),
                    span,
                }));
            }
        }
        Ok(None)
    }

    /// Получить текст узла из исходного кода
    fn node_text(node: &Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    // ============================================================
    // MILESTONE 2.19: Performance Optimization - Cached Versions
    // ============================================================
    //
    // Все методы ниже - cached версии для избежания O(n²) проблемы.
    // Они принимают предпросчитанный кеш строк `lines: &[String]`
    // для O(1) доступа вместо O(n) итерации через source.lines().nth()

    /// Конвертировать function_definition с использованием кеша строк (Milestone 2.19)
    fn convert_function_definition_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut params = Vec::new();
        let mut body = Vec::new();
        let is_procedure = node.kind() == "procedure_definition";

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = Self::node_text(&child, source);
                    }
                }
                "parameters" => {
                    params = Self::convert_parameters(&child, source)?;
                }
                _ => {
                    // Собираем тело функции
                    if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
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
                span,
            })
        } else {
            Ok(Statement::FunctionDecl {
                name,
                params,
                body,
                span,
            })
        }
    }

    /// Конвертировать var_definition с использованием кеша строк (Milestone 2.19)
    fn convert_var_definition_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut name = String::new();

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                name = Self::node_text(&child, source);
                break;
            }
        }

        Ok(Statement::VarDeclaration {
            name,
            type_hint: None,
            span: Self::node_to_span_cached(node, source, lines),
        })
    }

    /// Конвертировать if_statement с использованием кеша строк (Milestone 2.19)
    fn convert_if_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
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
                        if let Some(expr) = Self::convert_expression(&child, source)? {
                            condition = expr;
                        }
                    }
                }
                "else_clause" => {
                    let else_statements = Self::convert_clause_body_cached(&child, source, lines)?;
                    else_body = Some(else_statements);
                }
                "elseif_clause" => {
                    let elseif_statements =
                        Self::convert_clause_body_cached(&child, source, lines)?;
                    else_body = Some(elseif_statements);
                }
                kind if in_then
                    && (kind.ends_with("_statement") || kind.ends_with("_definition")) =>
                {
                    if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
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
    fn convert_clause_body_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "ELSE_KEYWORD" || child.kind() == "ELSIF_KEYWORD" {
                continue;
            }

            if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
                statements.push(stmt);
            }
        }

        Ok(statements)
    }

    /// Конвертировать for_statement с использованием кеша строк (Milestone 2.19)
    fn convert_for_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
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
                        variable = Self::node_text(&child, source);
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
                        if let Some(expr) = Self::convert_expression(&child, source)? {
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
                        if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
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
    fn convert_for_each_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
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
                        variable = Self::node_text(&child, source);
                    }
                }
                "DO_KEYWORD" | "ЦИКЛ_KEYWORD" => {
                    in_body = true;
                }
                "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" => {
                    break;
                }
                _ if !in_body && child.kind().contains("expression") => {
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        collection = expr;
                    }
                }
                _ => {
                    if in_body {
                        if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
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
    fn convert_while_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
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
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        condition = expr;
                    }
                }
                _ => {
                    if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
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

    /// Конвертировать try_statement с использованием кеша строк (Milestone 2.19)
    fn convert_try_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        let mut try_body = Vec::new();
        let mut except_body = Vec::new();
        let mut in_except = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "TRY_KEYWORD" | "ПОПЫТКА_KEYWORD" | "ENDTRY_KEYWORD" | "КОНЕЦПОПЫТКИ_KEYWORD" =>
                    {}
                "EXCEPT_KEYWORD" | "ИСКЛЮЧЕНИЕ_KEYWORD" => {
                    in_except = true;
                }
                _ => {
                    if let Some(stmt) = Self::convert_statement_cached(&child, source, lines)? {
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
            span,
        })
    }

    /// Конвертировать assignment_statement с использованием кеша строк (Milestone 2.19)
    fn convert_assignment_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        let mut target = None;
        let mut value = None;

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "property_access" {
                if target.is_none() {
                    target = Self::convert_expression(&child, source)?;
                }
            } else if let Some(expr) = Self::convert_expression(&child, source)? {
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
    fn convert_return_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        let mut value = None;

        for child in node.children(&mut cursor) {
            if child.kind() == "RETURN_KEYWORD" || child.kind() == "ВОЗВРАТ_KEYWORD" {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                value = Some(expr);
                break;
            }
        }

        Ok(Statement::Return { value, span })
    }

    /// Конвертировать call_statement с использованием кеша строк (Milestone 2.19)
    fn convert_call_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Call {
                    expression: expr,
                    span,
                });
            }
        }

        Err("call_statement without expression".to_string())
    }

    /// Конвертировать goto_statement с использованием кеша строк (Milestone 2.19)
    fn convert_goto_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let label = Self::node_text(&child, source);
                return Ok(Statement::Goto { label, span });
            }
        }
        Err("goto_statement without label".to_string())
    }

    /// Конвертировать label_statement с использованием кеша строк (Milestone 2.19)
    fn convert_label_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let name = Self::node_text(&child, source);
                return Ok(Statement::Label { name, span });
            }
        }
        Err("label_statement without name".to_string())
    }

    /// Конвертировать execute_statement с использованием кеша строк (Milestone 2.19)
    fn convert_execute_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Execute { code: expr, span });
            }
        }
        Err("execute_statement without code".to_string())
    }

    /// Конвертировать raise_error_statement с использованием кеша строк (Milestone 2.19)
    fn convert_raise_error_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        let mut message = None;

        for child in node.children(&mut cursor) {
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                message = Some(expr);
                break;
            }
        }

        Ok(Statement::RaiseError { message, span })
    }

    /// Конвертировать add_handler_statement с использованием кеша строк (Milestone 2.19)
    fn convert_add_handler_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let (event, handler) = Self::extract_event_handler_pair(node, source)?;
        Ok(Statement::AddHandler {
            event,
            handler,
            span,
        })
    }

    /// Конвертировать remove_handler_statement с использованием кеша строк (Milestone 2.19)
    fn convert_remove_handler_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let (event, handler) = Self::extract_event_handler_pair(node, source)?;
        Ok(Statement::RemoveHandler {
            event,
            handler,
            span,
        })
    }

    /// Конвертировать await_statement с использованием кеша строк (Milestone 2.19)
    fn convert_await_statement_cached(
        node: &Node,
        source: &str,
        lines: &[String],
    ) -> Result<Statement, String> {
        let span = Self::node_to_span_cached(node, source, lines);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Await {
                    expression: expr,
                    span,
                });
            }
        }
        Err("await_statement without expression".to_string())
    }
}
