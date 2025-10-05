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
            "for_statement" | "for_each_statement" => {
                Ok(Some(Self::convert_for_statement(node, source)?))
            }
            "assignment_statement" => Ok(Some(Self::convert_assignment(node, source)?)),
            "return_statement" => Ok(Some(Self::convert_return(node, source)?)),
            "call_statement" => Ok(Some(Self::convert_call_statement(node, source)?)),

            // Пропускаем препроцессор и комментарии
            "preprocessor" | "comment" => Ok(None),

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

    /// Конвертировать function_definition
    fn convert_function_definition(node: &Node, source: &str) -> Result<Statement, String> {
        let mut cursor = node.walk();
        let mut name = String::new();
        let mut params = Vec::new();
        let mut body = Vec::new();

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

        Ok(Statement::FunctionDecl { name, params, body })
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

    /// Конвертировать for_statement (упрощённая версия)
    fn convert_for_statement(_node: &Node, _source: &str) -> Result<Statement, String> {
        // Пока используем If как placeholder для циклов
        // TODO: Добавить Statement::For в ast.rs
        Ok(Statement::If {
            condition: Expression::Boolean(true),
            then_body: vec![],
            else_body: None,
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

    /// Конвертировать return_statement (упрощённо)
    fn convert_return(_node: &Node, _source: &str) -> Result<Statement, String> {
        // Пока используем Assignment как placeholder
        // TODO: Добавить Statement::Return в ast.rs
        Ok(Statement::Assignment {
            target: Expression::Identifier("__return".to_string()),
            value: Expression::Boolean(true),
        })
    }

    /// Конвертировать call_statement (вызов процедуры/функции)
    fn convert_call_statement(node: &Node, source: &str) -> Result<Statement, String> {
        // call_statement содержит вызов метода/функции
        // Преобразуем в Assignment для совместимости с текущим AST
        // TODO: Добавить Statement::Call в ast.rs
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(expr) = Self::convert_expression(&child, source)? {
                return Ok(Statement::Assignment {
                    target: Expression::Identifier("__call".to_string()),
                    value: expr,
                });
            }
        }

        Ok(Statement::Assignment {
            target: Expression::Identifier("__call".to_string()),
            value: Expression::Identifier("unknown".to_string()),
        })
    }

    /// Конвертировать expression узел
    fn convert_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
        match node.kind() {
            "identifier" => Ok(Some(Expression::Identifier(Self::node_text(node, source)))),

            "const_expression" => {
                let text = Self::node_text(node, source);
                // Попытка парсинга числа
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

            "binary_expression" => Self::convert_binary_expression(node, source),

            "call_expression" | "method_call" => Self::convert_call_expression(node, source),

            "property_access" => {
                // a.b.c → преобразуем в Identifier("a.b.c")
                Ok(Some(Expression::Identifier(Self::node_text(node, source))))
            }

            "unary_expression" => Self::convert_unary_expression(node, source),

            // Игнорируем ключевые слова и операторы
            kind if kind.ends_with("_KEYWORD") || kind == "=" || kind == "(" || kind == ")" => {
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
            if child.kind() == "identifier" || child.kind() == "property_access" {
                if function.is_none() {
                    function = Self::convert_expression(&child, source)?;
                }
            } else if let Some(expr) = Self::convert_expression(&child, source)? {
                args.push(expr);
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

    /// Получить текст узла из исходного кода
    fn node_text(node: &Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }
}
