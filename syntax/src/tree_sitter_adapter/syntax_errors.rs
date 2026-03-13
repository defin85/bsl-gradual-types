//! Сбор синтаксических ошибок из tree-sitter AST
//!
//! Модуль содержит функции для:
//! - Поиска ERROR узлов в дереве
//! - Проверки отсутствующих токенов (например, точек с запятой)
//! - Формирования диагностических сообщений

use std::sync::OnceLock;

use bsl_shared::domain::types::{ErrorType, ParseError, RelatedInformation};
use bsl_shared::ir::Span;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator};

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

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(
        semicolon_scoped_definition_query(),
        *node,
        source.as_bytes(),
    );
    matches.advance();
    while let Some(query_match) = matches.get() {
        for capture in query_match.captures {
            errors.extend(check_function_body_semicolons(
                &capture.node,
                source,
                line_index,
            ));
        }
        matches.advance();
    }

    errors
}

/// Проверить, есть ли в дереве хотя бы один пропущенный `;`.
///
/// Используется как дешёвый precheck для fast-path: при первом нарушении
/// выходим, не собирая полный список diagnostics.
pub fn has_missing_semicolons(node: &Node) -> bool {
    if matches!(node.kind(), "function_definition" | "procedure_definition")
        && function_body_has_missing_semicolon(node)
    {
        return true;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_missing_semicolons(&child) {
            return true;
        }
    }

    false
}

fn semicolon_scoped_definition_query() -> &'static Query {
    static QUERY: OnceLock<Query> = OnceLock::new();
    QUERY.get_or_init(|| {
        Query::new(
            &tree_sitter_bsl::LANGUAGE.into(),
            "[(function_definition) (procedure_definition)] @definition",
        )
        .expect("semicolon definition query must compile")
    })
}

/// Проверить точки с запятой в теле функции/процедуры
fn check_function_body_semicolons(
    func_node: &Node,
    source: &str,
    line_index: &LineIndex,
) -> Vec<ParseError> {
    let mut errors = Vec::new();
    let mut cursor = func_node.walk();
    let mut previous_statement: Option<Node> = None;
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
                if let Some(statement) = previous_statement.replace(child) {
                    maybe_push_missing_semicolon_error(&mut errors, &statement, source, line_index);
                }
            }
            // Конец функции/процедуры
            "ENDFUNCTION_KEYWORD" | "ENDPROCEDURE_KEYWORD" => {
                found_end_keyword = true;
            }
            _ => {}
        }
    }

    if let Some(statement) = previous_statement.filter(|_| !found_end_keyword) {
        maybe_push_missing_semicolon_error(&mut errors, &statement, source, line_index);
    }

    errors
}

fn function_body_has_missing_semicolon(func_node: &Node) -> bool {
    let mut cursor = func_node.walk();
    let mut previous_statement: Option<Node> = None;
    let mut found_end_keyword = false;

    for child in func_node.children(&mut cursor) {
        match child.kind() {
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
                if previous_statement
                    .replace(child)
                    .is_some_and(|statement| !has_semicolon_child(&statement))
                {
                    return true;
                }
            }
            "ENDFUNCTION_KEYWORD" | "ENDPROCEDURE_KEYWORD" => {
                found_end_keyword = true;
            }
            _ => {}
        }
    }

    previous_statement
        .filter(|_| !found_end_keyword)
        .is_some_and(|statement| !has_semicolon_child(&statement))
}

fn maybe_push_missing_semicolon_error(
    errors: &mut Vec<ParseError>,
    stmt: &Node,
    source: &str,
    line_index: &LineIndex,
) {
    if has_semicolon_child(stmt) {
        return;
    }

    let span = node_to_span_cached(stmt, source, line_index);

    errors.push(ParseError {
        message: format!(
            "Отсутствует точка с запятой после оператора '{}'",
            stmt.kind().replace("_statement", "")
        ),
        span: Span::new(span.end, span.end),
        error_type: ErrorType::MissingToken,
        related: Vec::new(),
    });
}

/// Проверить незавершённые выражения `Новый` без типа/аргументов.
///
/// Tree-sitter в некоторых случаях может "вылечить" `x = Новый` за счёт
/// захвата следующего идентификатора на следующей строке (потому что newline
/// — whitespace), и тогда ERROR/missing узлы не появляются.
///
/// Для IDE это плохо: hover/completion начинают работать по сломанной структуре.
/// Поэтому добавляем явную проверку по тексту.
pub fn check_incomplete_new_expressions(source: &str, line_index: &LineIndex) -> Vec<ParseError> {
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

        // Находим абсолютный byte offset в исходном тексте.
        let byte_pos = trimmed
            .rfind(kw)
            .unwrap_or(trimmed.len().saturating_sub(kw_len));
        let col_utf16 = byte_offset_to_utf16(trimmed, byte_pos + kw_len);
        let absolute = line_index.utf16_position_to_byte_offset(source, row as u32, col_utf16);
        let abs_u32 = absolute.min(u32::MAX as usize) as u32;

        errors.push(ParseError {
            message: "Отсутствует тип после 'Новый'".to_string(),
            span: Span::new(abs_u32, abs_u32),
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
        "КОНЕЦПОПЫТКИ_KEYWORD" => find_related(
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
    let _ = (source, line_index);
    let start = node.start_byte() as u32;
    Span::new(start, start)
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
