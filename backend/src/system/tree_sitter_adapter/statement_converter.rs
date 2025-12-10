//! Конвертация tree-sitter statement узлов в BSL Statement
//!
//! Этот модуль содержит логику преобразования различных типов statements:
//! - Объявления функций и процедур
//! - Объявления переменных
//! - Управляющие конструкции (if, for, while, try)
//! - Присваивания
//! - Вызовы процедур
//! - И другие statements языка 1С

use crate::parsing::bsl::ast::{Expression, Statement};
use bsl_shared::ir::Span;
use tracing::debug;
use tree_sitter::Node;

use super::directives::find_preceding_directive;
use super::expression_converter::convert_expression;
use super::span::{node_to_span, node_to_span_cached};
use super::utils::{convert_parameters, extract_event_handler_pair, node_text};

/// Конвертировать source_file (корневой узел) с использованием кеша строк (Milestone 2.19)
pub fn convert_source_file_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
            statements.push(stmt);
        }
    }

    Ok(statements)
}

/// Конвертировать statement узел с использованием кеша строк (Milestone 2.19)
pub fn convert_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Option<Statement>, String> {
    match node.kind() {
        "function_definition" | "procedure_definition" => Ok(Some(
            convert_function_definition_cached(node, source, lines)?,
        )),
        "var_definition" | "var_statement" => Ok(Some(convert_var_definition_cached(
            node, source, lines,
        )?)),
        "if_statement" => Ok(Some(convert_if_statement_cached(
            node, source, lines,
        )?)),
        "for_statement" => Ok(Some(convert_for_statement_cached(
            node, source, lines,
        )?)),
        "for_each_statement" => Ok(Some(convert_for_each_statement_cached(
            node, source, lines,
        )?)),
        "while_statement" => Ok(Some(convert_while_statement_cached(
            node, source, lines,
        )?)),
        "try_statement" => Ok(Some(convert_try_statement_cached(
            node, source, lines,
        )?)),
        "assignment_statement" => {
            Ok(Some(convert_assignment_cached(node, source, lines)?))
        }
        "return_statement" => Ok(Some(convert_return_cached(node, source, lines)?)),
        "call_statement" => Ok(Some(convert_call_statement_cached(
            node, source, lines,
        )?)),
        "break_statement" => Ok(Some(Statement::Break {
            span: node_to_span_cached(node, source, lines),
        })),
        "continue_statement" => Ok(Some(Statement::Continue {
            span: node_to_span_cached(node, source, lines),
        })),
        "goto_statement" => Ok(Some(convert_goto_statement_cached(
            node, source, lines,
        )?)),
        "label_statement" => Ok(Some(convert_label_statement_cached(
            node, source, lines,
        )?)),
        "execute_statement" => Ok(Some(convert_execute_statement_cached(
            node, source, lines,
        )?)),
        "rise_error_statement" => Ok(Some(convert_raise_error_statement_cached(
            node, source, lines,
        )?)),
        "add_handler_statement" => Ok(Some(convert_add_handler_statement_cached(
            node, source, lines,
        )?)),
        "remove_handler_statement" => Ok(Some(convert_remove_handler_statement_cached(
            node, source, lines,
        )?)),
        "await_statement" => Ok(Some(convert_await_statement_cached(
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

/// Конвертировать function_definition с использованием кеша строк (Milestone 2.19)
fn convert_function_definition_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Vec::new();
    let is_procedure = node.kind() == "procedure_definition";

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
            _ => {
                // Собираем тело функции
                if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
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
            span,
        })
    } else {
        Ok(Statement::FunctionDecl {
            name,
            params,
            body,
            compiler_directive,
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
            name = node_text(&child, source);
            break;
        }
    }

    Ok(Statement::VarDeclaration {
        name,
        type_hint: None,
        span: node_to_span_cached(node, source, lines),
    })
}

/// Конвертировать if_statement с использованием кеша строк (Milestone 2.19)
fn convert_if_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
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
                    if let Some(expr) = convert_expression(&child, source)? {
                        condition = expr;
                    }
                }
            }
            "else_clause" => {
                let else_statements = convert_clause_body_cached(&child, source, lines)?;
                else_body = Some(else_statements);
            }
            "elseif_clause" => {
                let elseif_statements =
                    convert_clause_body_cached(&child, source, lines)?;
                else_body = Some(elseif_statements);
            }
            kind if in_then
                && (kind.ends_with("_statement") || kind.ends_with("_definition")) =>
            {
                if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
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

        if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
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
    let span = node_to_span_cached(node, source, lines);
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
                    variable = node_text(&child, source);
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
                    if let Some(expr) = convert_expression(&child, source)? {
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
                    if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
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
    let span = node_to_span_cached(node, source, lines);
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
                    variable = node_text(&child, source);
                }
            }
            "DO_KEYWORD" | "ЦИКЛ_KEYWORD" => {
                in_body = true;
            }
            "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" => {
                break;
            }
            _ if !in_body && child.kind().contains("expression") => {
                if let Some(expr) = convert_expression(&child, source)? {
                    collection = expr;
                }
            }
            _ => {
                if in_body {
                    if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
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
    let span = node_to_span_cached(node, source, lines);
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
                if let Some(expr) = convert_expression(&child, source)? {
                    condition = expr;
                }
            }
            _ => {
                if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
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
    let span = node_to_span_cached(node, source, lines);
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
                if let Some(stmt) = convert_statement_cached(&child, source, lines)? {
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
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    let mut target = None;
    let mut value = None;

    for child in node.children(&mut cursor) {
        let child_kind = child.kind();
        if child_kind == "identifier" || child_kind == "property_access" {
            if target.is_none() {
                target = convert_expression(&child, source)?;
            } else if value.is_none() {
                // BUGFIX MILESTONE 3.16: If target is already set,
                // property_access/identifier goes to value!
                // Example: Dok = Documents.OrderClient
                //   - target = "Dok" (identifier)
                //   - value = "Documents.OrderClient" (property_access)
                value = convert_expression(&child, source)?;
            }
        } else if let Some(expr) = convert_expression(&child, source)? {
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
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    let mut value = None;

    for child in node.children(&mut cursor) {
        if child.kind() == "RETURN_KEYWORD" || child.kind() == "ВОЗВРАТ_KEYWORD" {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
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
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
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
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let label = node_text(&child, source);
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
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let name = node_text(&child, source);
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
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
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
    let span = node_to_span_cached(node, source, lines);
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

/// Конвертировать add_handler_statement с использованием кеша строк (Milestone 2.19)
fn convert_add_handler_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let (event, handler) = extract_event_handler_pair(node, source, convert_expression)?;
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
    let span = node_to_span_cached(node, source, lines);
    let (event, handler) = extract_event_handler_pair(node, source, convert_expression)?;
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
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().ends_with("_KEYWORD") {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Statement::Await {
                expression: expr,
                span,
            });
        }
    }
    Err("await_statement without expression".to_string())
}

// ============================================================
// Non-cached versions for backward compatibility
// ============================================================

/// Конвертировать source_file (корневой узел)
///
/// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
/// Для производительности используйте `convert_source_file_cached()` вместо него.
#[allow(dead_code)]
pub fn convert_source_file(node: &Node, source: &str) -> Result<Vec<Statement>, String> {
    // Для обратной совместимости: предпросчитываем строки
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    convert_source_file_cached(node, source, &lines)
}

/// Конвертировать statement узел
///
/// **ВАЖНО:** Этот метод делает O(n) итерацию для каждого узла при извлечении Span.
/// Для производительности используйте `convert_statement_cached()` вместо него.
#[allow(dead_code)]
pub fn convert_statement(node: &Node, source: &str) -> Result<Option<Statement>, String> {
    // Для обратной совместимости: предпросчитываем строки
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    convert_statement_cached(node, source, &lines)
}

/// Конвертировать function_definition или procedure_definition
#[allow(dead_code)]
pub fn convert_function_definition(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut name = String::new();
    let mut params = Vec::new();
    let mut body = Vec::new();
    let is_procedure = node.kind() == "procedure_definition";

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
            _ => {
                // Собираем тело функции
                if let Some(stmt) = convert_statement(&child, source)? {
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
            span,
        })
    } else {
        Ok(Statement::FunctionDecl {
            name,
            params,
            body,
            compiler_directive,
            span,
        })
    }
}

/// Конвертировать var_definition
#[allow(dead_code)]
pub fn convert_var_definition(node: &Node, source: &str) -> Result<Statement, String> {
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
        type_hint: None, // tree-sitter-bsl не поддерживает type hints
        span: node_to_span(node, source),
    })
}

/// Конвертировать if_statement
#[allow(dead_code)]
pub fn convert_if_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
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
                    if let Some(expr) = convert_expression(&child, source)? {
                        condition = expr;
                    }
                }
            }
            "else_clause" => {
                // Обработка else блока
                let else_statements = convert_clause_body(&child, source)?;
                else_body = Some(else_statements);
            }
            "elseif_clause" => {
                // Обработка elseif как вложенный if в else
                // TODO: более корректная обработка цепочек elseif
                let elseif_statements = convert_clause_body(&child, source)?;
                else_body = Some(elseif_statements);
            }
            // Любые statement узлы в then-блоке
            kind if in_then
                && (kind.ends_with("_statement") || kind.ends_with("_definition")) =>
            {
                if let Some(stmt) = convert_statement(&child, source)? {
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
pub fn convert_clause_body(node: &Node, source: &str) -> Result<Vec<Statement>, String> {
    let mut statements = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        // Пропускаем ключевые слова
        if child.kind() == "ELSE_KEYWORD" || child.kind() == "ELSIF_KEYWORD" {
            continue;
        }

        if let Some(stmt) = convert_statement(&child, source)? {
            statements.push(stmt);
        }
    }

    Ok(statements)
}

/// Конвертировать for_statement
#[allow(dead_code)]
pub fn convert_for_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
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
                    variable = node_text(&child, source);
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
                    if let Some(expr) = convert_expression(&child, source)? {
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
                    if let Some(stmt) = convert_statement(&child, source)? {
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
pub fn convert_assignment(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut target = None;
    let mut value = None;

    for child in node.children(&mut cursor) {
        let child_kind = child.kind();
        if child_kind == "identifier" || child_kind == "property_access" {
            if target.is_none() {
                target = convert_expression(&child, source)?;
            } else if value.is_none() {
                // BUGFIX MILESTONE 3.16: If target is already set,
                // property_access/identifier goes to value!
                value = convert_expression(&child, source)?;
            }
        } else if let Some(expr) = convert_expression(&child, source)? {
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
pub fn convert_return(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut value = None;

    for child in node.children(&mut cursor) {
        // Пропускаем ключевые слова
        if child.kind() == "RETURN_KEYWORD" || child.kind() == "ВОЗВРАТ_KEYWORD" {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            value = Some(expr);
            break;
        }
    }

    Ok(Statement::Return { value, span })
}

/// Конвертировать call_statement (вызов процедуры/функции)
#[allow(dead_code)]
pub fn convert_call_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
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
pub fn convert_while_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
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
                if let Some(expr) = convert_expression(&child, source)? {
                    condition = expr;
                }
            }
            _ => {
                if let Some(stmt) = convert_statement(&child, source)? {
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
pub fn convert_try_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
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
                if let Some(stmt) = convert_statement(&child, source)? {
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
pub fn convert_for_each_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
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
                    variable = node_text(&child, source);
                }
            }
            "DO_KEYWORD" | "ЦИКЛ_KEYWORD" => {
                in_body = true;
            }
            "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" => {
                break;
            }
            _ if !in_body && child.kind().contains("expression") => {
                if let Some(expr) = convert_expression(&child, source)? {
                    collection = expr;
                }
            }
            _ => {
                if in_body {
                    if let Some(stmt) = convert_statement(&child, source)? {
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
pub fn convert_goto_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let label = node_text(&child, source);
            return Ok(Statement::Goto { label, span });
        }
    }
    Err("goto_statement without label".to_string())
}

/// Конвертировать label_statement
#[allow(dead_code)]
pub fn convert_label_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let name = node_text(&child, source);
            return Ok(Statement::Label { name, span });
        }
    }
    Err("label_statement without name".to_string())
}

/// Конвертировать execute_statement
#[allow(dead_code)]
pub fn convert_execute_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Statement::Execute { code: expr, span });
        }
    }
    Err("execute_statement without code".to_string())
}

/// Конвертировать rise_error_statement
#[allow(dead_code)]
pub fn convert_raise_error_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut message = None;

    for child in node.children(&mut cursor) {
        // Пропускаем ключевые слова
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

/// Конвертировать add_handler_statement
#[allow(dead_code)]
pub fn convert_add_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let (event, handler) = extract_event_handler_pair(node, source, convert_expression)?;
    Ok(Statement::AddHandler {
        event,
        handler,
        span,
    })
}

/// Конвертировать remove_handler_statement
#[allow(dead_code)]
pub fn convert_remove_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let (event, handler) = extract_event_handler_pair(node, source, convert_expression)?;
    Ok(Statement::RemoveHandler {
        event,
        handler,
        span,
    })
}

/// Конвертировать await_statement
#[allow(dead_code)]
pub fn convert_await_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Пропускаем ключевые слова
        if child.kind().ends_with("_KEYWORD") {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Statement::Await {
                expression: expr,
                span,
            });
        }
    }
    Err("await_statement without expression".to_string())
}
