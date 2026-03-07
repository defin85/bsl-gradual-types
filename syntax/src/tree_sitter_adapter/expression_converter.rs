//! Конвертация tree-sitter expression узлов в BSL Expression
//!
//! Этот модуль содержит логику преобразования различных типов выражений:
//! - Литералы (числа, строки, булевы значения, даты)
//! - Идентификаторы
//! - Бинарные и унарные операции
//! - Вызовы функций/методов
//! - Доступ к свойствам и индексам
//! - Тернарный оператор
//! - New выражения
//! - Await выражения

use crate::ast::Expression;
use tracing::debug;
use tree_sitter::Node;

use super::span::node_to_span;
use super::utils::node_text;

/// Конвертировать expression узел
pub fn convert_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    match node.kind() {
        // Промежуточные узлы - рекурсивно обрабатываем дочерние
        "expression" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(expr) = convert_expression(&child, source)? {
                    return Ok(Some(expr));
                }
            }
            Ok(None)
        }

        "const_expression" => {
            // Сначала пытаемся найти дочерний узел (number, string, boolean, date)
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(expr) = convert_expression(&child, source)? {
                    return Ok(Some(expr));
                }
            }

            // Fallback: парсим текст напрямую
            let text = node_text(node, source);
            let span = node_to_span(node, source);
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
            name: node_text(node, source),
            span: node_to_span(node, source),
        })),

        "number" => {
            let text = node_text(node, source);
            let span = node_to_span(node, source);
            if let Ok(num) = text.parse::<f64>() {
                Ok(Some(Expression::Number { value: num, span }))
            } else {
                Ok(Some(Expression::String { value: text, span }))
            }
        }

        "string" => {
            let span = node_to_span(node, source);
            // string может содержать дочерние узлы (string_content)
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "string_content" {
                    return Ok(Some(Expression::String {
                        value: node_text(&child, source),
                        span,
                    }));
                }
            }

            // Fallback: берём весь текст и убираем кавычки
            let text = node_text(node, source);
            let trimmed = text.trim_matches('"');
            Ok(Some(Expression::String {
                value: trimmed.to_string(),
                span,
            }))
        }

        "boolean" => {
            let span = node_to_span(node, source);
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
            let text = node_text(node, source);
            if text.eq_ignore_ascii_case("истина") || text.eq_ignore_ascii_case("true") {
                Ok(Some(Expression::Boolean { value: true, span }))
            } else {
                Ok(Some(Expression::Boolean { value: false, span }))
            }
        }

        "date" => {
            Ok(Some(Expression::Date {
                value: node_text(node, source),
                span: node_to_span(node, source),
            }))
        }

        "binary_expression" => convert_binary_expression(node, source),

        "call_expression" | "method_call" => convert_call_expression(node, source),

        "property_access" => convert_property_access(node, source),

        "access" => convert_access(node, source),

        // `index` is the expression inside `access[index]` (see tree-sitter-bsl grammar).
        // It is NOT the whole index access expression.
        "index" => convert_index_expression(node, source),

        "unary_expression" => convert_unary_expression(node, source),

        "ternary_expression" => convert_ternary_expression(node, source),

        "new_expression" | "new_expression_method" => convert_new_expression(node, source),

        "await_expression" => convert_await_expression(node, source),

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

/// Конвертировать access узел
///
/// # MILESTONE 5.3 FIX: Правильная обработка access узлов для цепочек вызовов
///
/// access может содержать:
/// 1. Простой property access: access { access { identifier }, ".", property }
/// 2. Method call: access { access { ... }, ".", method_call { ... } }
/// 3. Вложенные chains: access { access { access {...}, ".", method_call }, ".", property }
///
/// Нужно собрать структуру: объект + (property ИЛИ method_call)
fn convert_access(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut object = None;
    let mut property_name = None;
    let mut method_call_node = None;
    let mut index_expr = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "access" => {
                // Внутренний access - рекурсивно конвертируем как объект
                object = convert_expression(&child, source)?;
            }
            "identifier" => {
                // Простой идентификатор (leaf node)
                if object.is_none() {
                    object = Some(Expression::Identifier {
                        name: node_text(&child, source),
                        span: node_to_span(&child, source),
                    });
                }
            }
            "property" => {
                // Свойство после точки
                property_name = Some(node_text(&child, source));
            }
            "method_call" => {
                // Вызов метода после точки
                method_call_node = Some(child);
            }
            "index" => {
                // Доступ по индексу: access { access { ... }, "[", index(expression), "]" }
                index_expr = convert_expression(&child, source)?;
            }
            "." => {} // Игнорируем точку
            _ => {
                // MILESTONE 5.3 FIX (дополнение): объект цепочки может быть НЕ access/identifier,
                // а выражение (например `Новый Структура().Вставить(...)`).
                // Пробуем сконвертировать любой "expression-like" child как object, если object ещё не найден.
                if object.is_none() {
                    if let Some(expr) = convert_expression(&child, source)? {
                        object = Some(expr);
                    } else {
                        debug!("access: skipping child {}", child.kind());
                    }
                } else {
                    debug!("access: skipping child {}", child.kind());
                }
            }
        }
    }

    // Index access: `obj[expr]`
    if let Some(idx) = index_expr {
        return match object {
            Some(obj) => Ok(Some(Expression::IndexAccess {
                object: Box::new(obj),
                index: Box::new(idx),
                span,
            })),
            None => Ok(None),
        };
    }

    // Строим результат в зависимости от структуры
    match (method_call_node, object, property_name) {
        // Случай 1: access содержит method_call -> это Call expression
        (Some(method), Some(obj), _) => {
            // Извлекаем имя метода и аргументы
            let mut method_cursor = method.walk();
            let mut method_name = String::new();
            let mut method_args = Vec::new();
            let mut method_identifier_span = span;

            for child in method.children(&mut method_cursor) {
                match child.kind() {
                    "identifier" => {
                        method_name = node_text(&child, source);
                        method_identifier_span = node_to_span(&child, source);
                    }
                    "arguments" => {
                        let mut args_cursor = child.walk();
                        for arg_child in child.children(&mut args_cursor) {
                            if arg_child.kind() == "expression" {
                                if let Some(expr) = convert_expression(&arg_child, source)? {
                                    method_args.push(expr);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Создаём PropertyAccess + Call
            let property_access = Expression::PropertyAccess {
                object: Box::new(obj),
                property: method_name,
                span: method_identifier_span,
            };

            Ok(Some(Expression::Call {
                function: Box::new(property_access),
                args: method_args,
                span,
            }))
        }
        // Случай 1b: method_call без объекта.
        //
        // Tree-sitter иногда представляет вызов "просто функции" как access{ method_call(...) }.
        // Например, `Структура().Вставить(...)` парсится как call_expression{ access(method_call Структура()), method_call Вставить() }.
        // В этом случае трактуем method_call как вызов глобальной функции.
        (Some(method), None, _) => {
            let mut method_cursor = method.walk();
            let mut method_name = String::new();
            let mut method_args = Vec::new();
            let mut method_identifier_span = span;

            for child in method.children(&mut method_cursor) {
                match child.kind() {
                    "identifier" => {
                        method_name = node_text(&child, source);
                        method_identifier_span = node_to_span(&child, source);
                    }
                    "arguments" => {
                        let mut args_cursor = child.walk();
                        for arg_child in child.children(&mut args_cursor) {
                            if arg_child.kind() == "expression" {
                                if let Some(expr) = convert_expression(&arg_child, source)? {
                                    method_args.push(expr);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if method_name.is_empty() {
                debug!("access: method_call without identifier");
                return Ok(None);
            }

            let function = Expression::Identifier {
                name: method_name,
                span: method_identifier_span,
            };

            Ok(Some(Expression::Call {
                function: Box::new(function),
                args: method_args,
                span: method_identifier_span,
            }))
        }
        // Случай 2: access содержит property -> это PropertyAccess
        (None, Some(obj), Some(prop)) => Ok(Some(Expression::PropertyAccess {
            object: Box::new(obj),
            property: prop,
            span,
        })),
        // Случай 3: только объект (leaf node)
        (None, Some(obj), None) => Ok(Some(obj)),
        // Ничего не распознано
        (None, None, _) => {
            debug!("access: couldn't parse structure, returning None");
            Ok(None)
        }
    }
}

/// Конвертировать binary_expression
fn convert_binary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut left = None;
    let mut operator = String::new();
    let mut right = None;

    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
            if left.is_none() {
                left = Some(expr);
            } else {
                right = Some(expr);
            }
        } else if child.kind() == "operator" {
            operator = node_text(&child, source);
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
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut function = None;
    let mut args = Vec::new();

    // MILESTONE 3.5 FIX: Обрабатываем паттерн вызова метода
    // Tree-sitter создаёт: call_expression { access, ".", method_call }
    // Нужно собрать PropertyAccess из access + method_call
    let mut access_node = None;
    let mut method_call_node = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "access" => {
                access_node = Some(child);
            }
            "method_call" => {
                method_call_node = Some(child);
            }
            "identifier" | "property_access" => {
                if function.is_none() {
                    function = convert_expression(&child, source)?;
                }
            }
            "arguments" => {
                // Парсим аргументы из узла arguments
                let mut args_cursor = child.walk();
                for arg_child in child.children(&mut args_cursor) {
                    if arg_child.kind() == "expression" {
                        if let Some(expr) = convert_expression(&arg_child, source)? {
                            args.push(expr);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // MILESTONE 3.5: Если есть access + method_call -> создаём PropertyAccess
    if let (Some(access), Some(method)) = (access_node, method_call_node) {
        // Получаем object из access
        let Some(object) = convert_expression(&access, source)? else {
            // В некоторых случаях tree-sitter может дать access без корректно извлекаемого объекта
            // (например, при наличии ERROR-узлов). Не валим весь парсинг — просто падаем обратно
            // на стандартный разбор (который в худшем случае вернёт None).
            return Ok(None);
        };

        // Получаем имя метода из method_call И его span
        let mut method_cursor = method.walk();
        let mut method_name = String::new();
        let mut method_args = Vec::new();
        let mut method_identifier_span = span; // По умолчанию весь span

        for child in method.children(&mut method_cursor) {
            match child.kind() {
                "identifier" => {
                    method_name = node_text(&child, source);
                    method_identifier_span = node_to_span(&child, source);
                    // Span только имени метода!
                }
                "arguments" => {
                    // Парсим аргументы метода
                    let mut args_cursor = child.walk();
                    for arg_child in child.children(&mut args_cursor) {
                        if arg_child.kind() == "expression" {
                            if let Some(expr) = convert_expression(&arg_child, source)? {
                                method_args.push(expr);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Создаём PropertyAccess для объекта.метод с span ТОЛЬКО метода
        let property_access = Expression::PropertyAccess {
            object: Box::new(object),
            property: method_name,
            span: method_identifier_span, // ИСПРАВЛЕНИЕ: span только имени метода
        };

        return Ok(Some(Expression::Call {
            function: Box::new(property_access),
            args: method_args,
            span: method_identifier_span, // ИСПРАВЛЕНИЕ: span только имени метода для диагностики
        }));
    }

    // Стандартный случай: обычная функция
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
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut operator = String::new();
    let mut operand = None;

    for child in node.children(&mut cursor) {
        if child.kind() == "NOT_KEYWORD" || child.kind() == "-" {
            operator = child.kind().to_string();
        } else if let Some(expr) = convert_expression(&child, source)? {
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
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut object = None;
    let mut property = String::new();
    let mut index_expr = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            // Прямой identifier (старая грамматика или fallback)
            "identifier" => {
                if object.is_none() {
                    let child_span = node_to_span(&child, source);
                    object = Some(Expression::Identifier {
                        name: node_text(&child, source),
                        span: child_span,
                    });
                } else {
                    property = node_text(&child, source);
                }
            }
            // BUGFIX: tree-sitter-bsl использует "access" узел для объекта
            // Структура: property_access -> access -> identifier
            "access" => {
                if object.is_none() {
                    // Рекурсивно конвертируем содержимое access
                    if let Some(expr) = convert_expression(&child, source)? {
                        object = Some(expr);
                    }
                }
            }
            "property_access" => {
                // Вложенный property access (a.b.c)
                if let Some(expr) = convert_expression(&child, source)? {
                    object = Some(expr);
                }
            }
            "property" => {
                property = node_text(&child, source);
            }
            "index" => {
                // index access: property_access { access, "[", index(expression), "]" }
                index_expr = convert_expression(&child, source)?;
            }
            _ => {}
        }
    }

    // Index access: `obj[expr]`
    if let Some(idx) = index_expr {
        return match object {
            Some(obj) => Ok(Some(Expression::IndexAccess {
                object: Box::new(obj),
                index: Box::new(idx),
                span,
            })),
            None => Ok(None),
        };
    }

    match object {
        Some(obj) if !property.is_empty() => Ok(Some(Expression::PropertyAccess {
            object: Box::new(obj),
            property,
            span,
        })),
        Some(obj) => {
            // Preserve the stable receiver for incomplete member access like `obj.`
            // or `obj[index].` instead of degrading to an undeclared identifier.
            Ok(Some(obj))
        }
        None => {
            // Fallback: возвращаем как Identifier
            Ok(Some(Expression::Identifier {
                name: node_text(node, source),
                span,
            }))
        }
    }
}

/// Конвертировать индекс-выражение в `access[index]`.
///
/// В tree-sitter-bsl `index` — это alias для `expression` внутри квадратных скобок,
/// а не отдельный узел "index access". Сам "index access" — это `access`/`property_access`
/// c дочерними узлами `access` + `index`.
fn convert_index_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Some(expr));
        }
    }
    Ok(None)
}

/// Конвертировать ternary_expression
fn convert_ternary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut condition = None;
    let mut then_expr = None;
    let mut else_expr = None;

    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
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
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    let mut type_expr = None;
    let mut args = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "NEW_KEYWORD" | "НОВЫЙ_KEYWORD" => {}
            "identifier" | "property_access" => {
                // Новый Тип или Новый Модуль.Тип - прямой identifier
                if type_expr.is_none() {
                    type_expr = Some(node_text(&child, source));
                }
            }
            "arguments" => {
                // Новый(Выражение) - expression внутри arguments
                // Парсим аргументы и берём первый как type_expr
                let mut arg_cursor = child.walk();
                for arg_child in child.children(&mut arg_cursor) {
                    if arg_child.kind() == "expression" {
                        if let Some(expr) = convert_expression(&arg_child, source)? {
                            if type_expr.is_none() {
                                // Первое выражение - это тип для конструктора
                                // Сохраняем как строку из исходного кода
                                type_expr = Some(node_text(&arg_child, source));
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
    let span = node_to_span(node, source);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Пропускаем ключевые слова
        if child.kind().ends_with("_KEYWORD") {
            continue;
        }

        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Some(Expression::Await {
                expression: Box::new(expr),
                span,
            }));
        }
    }
    Ok(None)
}
