//! Сбор синтаксических ошибок из tree-sitter AST
//!
//! Модуль содержит функции для:
//! - Поиска ERROR узлов в дереве
//! - Проверки отсутствующих токенов (например, точек с запятой)
//! - Формирования диагностических сообщений

use bsl_shared::domain::types::{ErrorType, ParseError, RelatedInformation};
use bsl_shared::ir::Span;
use tree_sitter::Node;

use super::span::{byte_offset_to_utf16, node_to_span_cached, LineIndex};

/// Собрать все ERROR узлы из дерева (рекурсивный обход)
///
/// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
/// Для производительности используйте `collect_syntax_errors_cached()` вместо него.
#[allow(dead_code)]
pub fn collect_syntax_errors(node: &Node, source: &str) -> Vec<ParseError> {
    let line_index = LineIndex::new(source);
    collect_syntax_errors_cached(node, source, &line_index)
}

/// Собрать все ERROR узлы из дерева с использованием кеша строк (Milestone 2.19)
pub fn collect_syntax_errors_cached(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Vec<ParseError> {
    let mut errors = Vec::new();

    // Если текущий узел - ERROR, добавляем его
    if node.kind() == "ERROR" {
        let span = node_to_span_cached(node, source, line_index);
        let text = node
            .utf8_text(source.as_bytes())
            .unwrap_or("<неизвестно>")
            .to_string();
        let trimmed = text.trim_start();
        if trimmed.starts_with('&') {
            return errors;
        }

        errors.push(ParseError {
            message: format!("Синтаксическая ошибка: неожиданный текст '{}'", text),
            span,
            error_type: ErrorType::ParseError,
            related: Vec::new(),
        });
    }

    // Проверяем node.is_missing() для пропущенных токенов
    if node.is_missing() {
        let span = node_to_span_cached(node, source, line_index);
        let related = missing_token_related_info(node, source, line_index)
            .into_iter()
            .collect();
        errors.push(ParseError {
            message: format!("Отсутствует обязательный элемент: {}", node.kind()),
            span,
            error_type: ErrorType::MissingToken,
            related,
        });
    }

    // Рекурсивно обходим дочерние узлы
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        errors.extend(collect_syntax_errors_cached(&child, source, line_index));
    }

    errors
}

/// Проверить отсутствующие точки с запятой между statements
///
/// В BSL точка с запятой ОБЯЗАТЕЛЬНА между операторами, кроме последнего оператора
/// перед закрывающим ключевым словом (КонецФункции, КонецПроцедуры).
pub fn check_missing_semicolons(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Vec<ParseError> {
    let mut errors = Vec::new();

    // Проверяем только тела функций и процедур
    if matches!(node.kind(), "function_definition" | "procedure_definition") {
        errors.extend(check_function_body_semicolons(node, source, line_index));
    }

    // Рекурсивно проверяем вложенные узлы (для вложенных конструкций)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        errors.extend(check_missing_semicolons(&child, source, line_index));
    }

    errors
}

/// Проверить точки с запятой в теле функции/процедуры
fn check_function_body_semicolons(
    func_node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Vec<ParseError> {
    let mut errors = Vec::new();
    let mut cursor = func_node.walk();

    // Собираем все statement узлы в теле функции
    let mut statements: Vec<Node> = Vec::new();
    let mut found_end_keyword = false;

    for child in func_node.children(&mut cursor) {
        match child.kind() {
            // Statements, которые должны заканчиваться точкой с запятой (если не последние)
            "if_statement"
            | "while_statement"
            | "for_statement"
            | "for_each_statement"
            | "assignment_statement"
            | "call_statement"
            | "return_statement"
            | "break_statement"
            | "continue_statement"
            | "var_statement"
            | "try_statement" => {
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
        if !has_semicolon_child(stmt) {
            let span = node_to_span_cached(stmt, source, line_index);

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
                related: Vec::new(),
            });
        }
    }

    errors
}

/// Проверить незавершённые выражения `Новый` без типа/аргументов.
///
/// Tree-sitter в некоторых случаях может "вылечить" `x = Новый` за счёт
/// захвата следующего идентификатора на следующей строке (потому что newline
/// — whitespace), и тогда ERROR/missing узлы не появляются.
///
/// Для IDE это плохо: hover/completion начинают работать по сломанной структуре.
/// Поэтому добавляем явную проверку по тексту.
pub fn check_incomplete_new_expressions(
    source: &str,
    line_index: &LineIndex,
) -> Vec<ParseError> {
    let mut errors = Vec::new();

    for row in 0..line_index.line_count() {
        let line = line_index.line_text(source, row);
        let trimmed = line.trim_end();

        // Игнорируем комментарии и пустые строки.
        if trimmed.is_empty() || trimmed.trim_start().starts_with("//") {
            continue;
        }

        let (kw, kw_len) = if trimmed.ends_with("Новый") {
            ("Новый", "Новый".len())
        } else if trimmed.to_ascii_lowercase().ends_with("new") {
            ("new", 3)
        } else {
            continue;
        };

        // Минимальная эвристика: обычно это assignment (`=` где-то раньше).
        if !trimmed.contains('=') {
            continue;
        }

        // Находим позицию в UTF-16.
        let byte_pos = trimmed
            .rfind(kw)
            .unwrap_or(trimmed.len().saturating_sub(kw_len));
        let col_utf16 = byte_offset_to_utf16(trimmed, byte_pos + kw_len);
        let row_u32 = row as u32;

        errors.push(ParseError {
            message: "Отсутствует тип после 'Новый'".to_string(),
            span: Span::from_positions((row_u32, col_utf16), (row_u32, col_utf16)),
            error_type: ErrorType::MissingToken,
            related: Vec::new(),
        });
    }

    // suppress unused warning for `source` if we later expand the heuristic
    let _ = source;
    errors
}

fn missing_token_related_info(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Option<RelatedInformation> {
    match node.kind() {
        "ENDIF_KEYWORD" => find_related(
            node,
            source,
            line_index,
            &[("if_statement", "Начало блока: Если")],
        ),
        "ENDDO_KEYWORD" => find_related(
            node,
            source,
            line_index,
            &[
                ("while_statement", "Начало блока: Пока"),
                ("for_statement", "Начало блока: Для"),
                ("for_each_statement", "Начало блока: Для каждого"),
            ],
        ),
        "ENDTRY_KEYWORD" => find_related(
            node,
            source,
            line_index,
            &[("try_statement", "Начало блока: Попытка")],
        ),
        "ENDFUNCTION_KEYWORD" => find_related(
            node,
            source,
            line_index,
            &[("function_definition", "Начало блока: Функция")],
        ),
        "ENDPROCEDURE_KEYWORD" => find_related(
            node,
            source,
            line_index,
            &[("procedure_definition", "Начало блока: Процедура")],
        ),
        "PREPROC_ENDIF_KEYWORD" => find_related(
            node,
            source,
            line_index,
            &[("preprocessor", "Начало директивы: #Если")],
        ),
        "PREPROC_ENDREGION_KEYWORD" => find_related(
            node,
            source,
            line_index,
            &[("preprocessor", "Начало директивы: #Область")],
        ),
        _ => None,
    }
}

fn find_related(
    node: &Node,
    source: &str,
    line_index: &LineIndex,
    ancestors: &[(&str, &str)],
) -> Option<RelatedInformation> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if let Some((_, message)) = ancestors.iter().find(|(kind, _)| *kind == parent.kind()) {
            let span = span_at_node_start(&parent, source, line_index);
            return Some(RelatedInformation {
                message: (*message).to_string(),
                span,
            });
        }
        current = parent.parent();
    }
    None
}

fn span_at_node_start(node: &Node, source: &str, line_index: &LineIndex) -> Span {
    let start = node.start_position();
    let start_column = line_index.byte_offset_to_utf16(source, start.row, start.column);
    Span::from_positions(
        (start.row as u32, start_column),
        (start.row as u32, start_column),
    )
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
