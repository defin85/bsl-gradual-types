//! Адаптер для конвертации tree-sitter-bsl AST в доменный Program AST
//!
//! Преобразует узлы tree-sitter в структуры из backend/src/parsing/bsl/mod.rs

use crate::parsing::bsl::ast::{Expression, Program, Statement};
use tree_sitter::{Node, Tree};
use tracing::debug;

/// Адаптер tree-sitter AST → Program AST
pub struct TreeSitterAdapter;

impl TreeSitterAdapter {
    /// Конвертировать дерево tree-sitter в Program
    pub fn convert_tree(tree: &Tree, source: &str) -> Result<Program, String> {
        let root = tree.root_node();
        let statements = Self::convert_source_file(&root, source)?;

        Ok(Program { statements })
    }

    /// Конвертировать source_file (корневой узел)
    fn convert_source_file(node: &Node, source: &str) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(stmt) = Self::convert_statement(&child, source)? {
                statements.push(stmt);
            }
        }

        Ok(statements)
    }

    /// Конвертировать statement узел
    fn convert_statement(node: &Node, source: &str) -> Result<Option<Statement>, String> {
        match node.kind() {
            "function_definition" | "procedure_definition" => {
                Ok(Some(Self::convert_function_definition(node, source)?))
            }
            "var_definition" | "var_statement" => {
                Ok(Some(Self::convert_var_definition(node, source)?))
            }
            "if_statement" => Ok(Some(Self::convert_if_statement(node, source)?)),
            "for_statement" => Ok(Some(Self::convert_for_statement(node, source)?)),
            "for_each_statement" => Ok(Some(Self::convert_for_each_statement(node, source)?)),
            "while_statement" => Ok(Some(Self::convert_while_statement(node, source)?)),
            "try_statement" => Ok(Some(Self::convert_try_statement(node, source)?)),
            "assignment_statement" => Ok(Some(Self::convert_assignment(node, source)?)),
            "return_statement" => Ok(Some(Self::convert_return(node, source)?)),
            "call_statement" => Ok(Some(Self::convert_call_statement(node, source)?)),
            "break_statement" => Ok(Some(Statement::Break)),
            "continue_statement" => Ok(Some(Statement::Continue)),
            "goto_statement" => Ok(Some(Self::convert_goto_statement(node, source)?)),
            "label_statement" => Ok(Some(Self::convert_label_statement(node, source)?)),
            "execute_statement" => Ok(Some(Self::convert_execute_statement(node, source)?)),
            "rise_error_statement" => Ok(Some(Self::convert_raise_error_statement(node, source)?)),
            "add_handler_statement" => Ok(Some(Self::convert_add_handler_statement(node, source)?)),
            "remove_handler_statement" => Ok(Some(Self::convert_remove_handler_statement(node, source)?)),
            "await_statement" => Ok(Some(Self::convert_await_statement(node, source)?)),

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
    fn convert_function_definition(node: &Node, source: &str) -> Result<Statement, String> {
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
            Ok(Statement::ProcedureDecl { name, params, body })
        } else {
            Ok(Statement::FunctionDecl { name, params, body })
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
        })
    }

    /// Конвертировать if_statement
    fn convert_if_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut condition = Expression::Boolean(true); // default
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
        })
    }

    /// Конвертировать тело clause (else_clause, elseif_clause)
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
    fn convert_for_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut variable = String::new();
        let mut start = Expression::Number(0.0);
        let mut end = Expression::Number(0.0);
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
                _ if child.kind().contains("expression") || child.kind() == "const_expression" || child.kind() == "number" => {
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
        })
    }

    /// Конвертировать assignment_statement
    fn convert_assignment(node: &Node, source: &str) -> Result<Statement, String> {
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
            target: target.unwrap_or(Expression::Identifier("unknown".to_string())),
            value: value.unwrap_or(Expression::Identifier("unknown".to_string())),
        })
    }

    /// Конвертировать return_statement
    fn convert_return(node: &Node, source: &str) -> Result<Statement, String> {
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

        Ok(Statement::Return { value })
    }

    /// Конвертировать call_statement (вызов процедуры/функции)
    fn convert_call_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Call { expression: expr });
            }
        }

        Err("call_statement without expression".to_string())
    }

    /// Конвертировать while_statement
    fn convert_while_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut condition = Expression::Boolean(true);
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

        Ok(Statement::While { condition, body })
    }

    /// Конвертировать try_statement
    fn convert_try_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut try_body = Vec::new();
        let mut except_body = Vec::new();
        let mut in_except = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "TRY_KEYWORD" | "ПОПЫТКА_KEYWORD" | "ENDTRY_KEYWORD" | "КОНЕЦПОПЫТКИ_KEYWORD" => {}
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
        })
    }

    /// Конвертировать for_each_statement
    fn convert_for_each_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut variable = String::new();
        let mut collection = Expression::Identifier("unknown".to_string());
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
        })
    }

    /// Конвертировать goto_statement
    fn convert_goto_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let label = Self::node_text(&child, source);
                return Ok(Statement::Goto { label });
            }
        }
        Err("goto_statement without label".to_string())
    }

    /// Конвертировать label_statement
    fn convert_label_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let name = Self::node_text(&child, source);
                return Ok(Statement::Label { name });
            }
        }
        Err("label_statement without name".to_string())
    }

    /// Конвертировать execute_statement
    fn convert_execute_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Execute { code: expr });
            }
        }
        Err("execute_statement without code".to_string())
    }

    /// Конвертировать rise_error_statement
    fn convert_raise_error_statement(node: &Node, source: &str) -> Result<Statement, String> {
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

        Ok(Statement::RaiseError { message })
    }

    /// Конвертировать add_handler_statement
    fn convert_add_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let (event, handler) = Self::extract_event_handler_pair(node, source)?;
        Ok(Statement::AddHandler { event, handler })
    }

    /// Конвертировать remove_handler_statement
    fn convert_remove_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let (event, handler) = Self::extract_event_handler_pair(node, source)?;
        Ok(Statement::RemoveHandler { event, handler })
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
    fn convert_await_statement(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Await { expression: expr });
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
                if let Ok(num) = text.parse::<f64>() {
                    Ok(Some(Expression::Number(num)))
                } else if text.eq_ignore_ascii_case("истина")
                    || text.eq_ignore_ascii_case("true")
                {
                    Ok(Some(Expression::Boolean(true)))
                } else if text.eq_ignore_ascii_case("ложь")
                    || text.eq_ignore_ascii_case("false")
                {
                    Ok(Some(Expression::Boolean(false)))
                } else {
                    Ok(Some(Expression::String(text)))
                }
            }

            "identifier" => Ok(Some(Expression::Identifier(Self::node_text(node, source)))),

            "number" => {
                let text = Self::node_text(node, source);
                if let Ok(num) = text.parse::<f64>() {
                    Ok(Some(Expression::Number(num)))
                } else {
                    Ok(Some(Expression::String(text)))
                }
            }

            "string" => {
                // string может содержать дочерние узлы (string_content)
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "string_content" {
                        return Ok(Some(Expression::String(Self::node_text(&child, source))));
                    }
                }

                // Fallback: берём весь текст и убираем кавычки
                let text = Self::node_text(node, source);
                let trimmed = text.trim_matches('"');
                Ok(Some(Expression::String(trimmed.to_string())))
            }

            "boolean" => {
                // boolean узел содержит дочерний TRUE_KEYWORD или FALSE_KEYWORD
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    let child_kind = child.kind();
                    if child_kind == "TRUE_KEYWORD" {
                        return Ok(Some(Expression::Boolean(true)));
                    } else if child_kind == "FALSE_KEYWORD" {
                        return Ok(Some(Expression::Boolean(false)));
                    }
                }

                // Fallback: парсим текст
                let text = Self::node_text(node, source);
                if text.eq_ignore_ascii_case("истина") || text.eq_ignore_ascii_case("true") {
                    Ok(Some(Expression::Boolean(true)))
                } else {
                    Ok(Some(Expression::Boolean(false)))
                }
            }

            "date" => {
                Ok(Some(Expression::Date(Self::node_text(node, source))))
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
            }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать call_expression
    fn convert_call_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
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
            }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать unary_expression
    fn convert_unary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
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
            }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать property_access
    fn convert_property_access(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let mut cursor = node.walk();
        let mut object = None;
        let mut property = String::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if object.is_none() {
                        object = Some(Expression::Identifier(Self::node_text(&child, source)));
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
            })),
            _ => {
                // Fallback: возвращаем как Identifier
                Ok(Some(Expression::Identifier(Self::node_text(node, source))))
            }
        }
    }

    /// Конвертировать index_access (доступ по индексу: arr[0])
    fn convert_index_access(node: &Node, source: &str) -> Result<Option<Expression>, String> {
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
            })),
            _ => Ok(None),
        }
    }

    /// Конвертировать ternary_expression
    fn convert_ternary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
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
            })),
            _ => Ok(None),
        }
    }

    /// Конвертировать new_expression
    fn convert_new_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let mut cursor = node.walk();
        let mut type_name = String::new();
        let mut args = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "NEW_KEYWORD" | "НОВЫЙ_KEYWORD" => {}
                "identifier" | "property_access" => {
                    if type_name.is_empty() {
                        type_name = Self::node_text(&child, source);
                    }
                }
                _ => {
                    if let Some(expr) = Self::convert_expression(&child, source)? {
                        args.push(expr);
                    }
                }
            }
        }

        if !type_name.is_empty() {
            Ok(Some(Expression::New { type_name, args }))
        } else {
            Ok(None)
        }
    }

    /// Конвертировать await_expression
    fn convert_await_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Пропускаем ключевые слова
            if child.kind().ends_with("_KEYWORD") {
                continue;
            }

            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Some(Expression::Await {
                    expression: Box::new(expr),
                }));
            }
        }
        Ok(None)
    }

    /// Получить текст узла из исходного кода
    fn node_text(node: &Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }
}
