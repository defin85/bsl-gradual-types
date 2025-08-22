//! Адаптер для парсера tree-sitter-bsl
//!
//! Этот модуль обеспечивает интеграцию tree-sitter-bsl парсера
//! с нашей системой типов, конвертируя tree-sitter AST в наш формат.

use crate::parser::ast::{BinaryOp, Expression, Parameter, Program, Statement, UnaryOp};
use crate::parser::common::{Parser, TextChange};
use anyhow::{Context, Result};
use tree_sitter::{Language, Node, Parser as TSParser};

// Внешняя функция для получения языка BSL
extern "C" {
    fn tree_sitter_bsl() -> Language;
}

/// Адаптер для tree-sitter-bsl парсера
pub struct TreeSitterAdapter {
    parser: TSParser,
    source: String,
    last_tree: Option<tree_sitter::Tree>,
}

impl TreeSitterAdapter {
    /// Создать новый экземпляр адаптера
    pub fn new() -> Result<Self> {
        let mut parser = TSParser::new();
        let language = unsafe { tree_sitter_bsl() };

        // Проверяем, что язык предоставлен (не заглушка)
        if language.abi_version() == 0 {
            return Err(anyhow::anyhow!(
                "Tree-sitter BSL language not available. This is a stub implementation."
            ));
        }

        parser
            .set_language(&language)
            .context("Failed to set BSL language")?;

        Ok(Self {
            parser,
            source: String::new(),
            last_tree: None,
        })
    }

    /// Парсить BSL код
    pub fn parse_impl(&mut self, source: &str) -> Result<Program> {
        self.source = source.to_string();

        let tree = self
            .parser
            .parse(source, self.last_tree.as_ref())
            .context("Failed to parse BSL code")?;

        let root_node = tree.root_node();
        let program = self.convert_program(root_node)?;

        // Сохраняем дерево для инкрементального парсинга
        self.last_tree = Some(tree);

        Ok(program)
    }

    /// Конвертировать корневой узел в Program
    fn convert_program(&self, node: Node) -> Result<Program> {
        let mut statements = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if let Some(stmt) = self.convert_statement(child)? {
                statements.push(stmt);
            }
        }

        Ok(Program { statements })
    }

    /// Конвертировать узел в Statement
    fn convert_statement(&self, node: Node) -> Result<Option<Statement>> {
        match node.kind() {
            "procedure_definition" => Ok(Some(self.convert_procedure(node)?)),
            "function_definition" => Ok(Some(self.convert_function(node)?)),
            "var_definition" | "var_statement" => Ok(Some(self.convert_var_declaration(node)?)),
            "assignment_statement" => Ok(Some(self.convert_assignment(node)?)),
            "if_statement" => Ok(Some(self.convert_if_statement(node)?)),
            "while_statement" => Ok(Some(self.convert_while_statement(node)?)),
            "for_statement" | "for_each_statement" => Ok(Some(self.convert_for_statement(node)?)),
            "return_statement" => Ok(Some(self.convert_return_statement(node)?)),
            "call_statement" => Ok(Some(self.convert_call_statement(node)?)),
            "break_statement" => Ok(Some(Statement::Break)),
            "continue_statement" => Ok(Some(Statement::Continue)),
            "line_comment" | "preprocessor" => {
                // Пропускаем комментарии и препроцессорные директивы
                Ok(None)
            }
            _ => {
                // Неизвестный тип узла - пропускаем
                println!("Unknown statement type: {}", node.kind());
                Ok(None)
            }
        }
    }

    /// Конвертировать процедуру
    fn convert_procedure(&self, node: Node) -> Result<Statement> {
        let mut name = String::new();
        let mut params = Vec::new();
        let mut body = Vec::new();
        let mut export = false;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = self.get_node_text(child);
                    }
                }
                "parameters" => {
                    params = self.convert_parameters(child)?;
                }
                "EXPORT_KEYWORD" => {
                    export = true;
                }
                _ if self.is_statement_node(child) => {
                    if let Some(stmt) = self.convert_statement(child)? {
                        body.push(stmt);
                    }
                }
                _ => {}
            }
        }

        Ok(Statement::ProcedureDecl {
            name,
            params,
            body,
            export,
        })
    }

    /// Конвертировать функцию
    fn convert_function(&self, node: Node) -> Result<Statement> {
        let mut name = String::new();
        let mut params = Vec::new();
        let mut body = Vec::new();
        let mut return_value = None;
        let mut export = false;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    if name.is_empty() {
                        name = self.get_node_text(child);
                    }
                }
                "parameters" => {
                    params = self.convert_parameters(child)?;
                }
                "EXPORT_KEYWORD" => {
                    export = true;
                }
                _ if self.is_statement_node(child) => {
                    if let Some(stmt) = self.convert_statement(child)? {
                        // Проверяем, не является ли это оператором возврата
                        if let Statement::Return(expr) = stmt {
                            return_value = expr;
                        } else {
                            body.push(stmt);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Statement::FunctionDecl {
            name,
            params,
            body,
            return_value,
            export,
        })
    }

    /// Конвертировать параметры функции/процедуры
    fn convert_parameters(&self, node: Node) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "parameter" {
                params.push(self.convert_parameter(child)?);
            }
        }

        Ok(params)
    }

    /// Конвертировать один параметр
    fn convert_parameter(&self, node: Node) -> Result<Parameter> {
        let mut name = String::new();
        let mut by_value = false;
        let mut default_value = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    name = self.get_node_text(child);
                }
                "VAL_KEYWORD" => {
                    by_value = true;
                }
                _ if self.is_expression_node(child) => {
                    default_value = Some(self.convert_expression(child)?);
                }
                _ => {}
            }
        }

        Ok(Parameter {
            name,
            by_value,
            default_value,
        })
    }

    /// Конвертировать выражение
    fn convert_expression(&self, node: Node) -> Result<Expression> {
        match node.kind() {
            "expression" => {
                // Обёртка expression - смотрим на первого потомка
                let mut cursor = node.walk();
                if let Some(child) = node.children(&mut cursor).next() {
                    return self.convert_expression(child);
                }
                Ok(Expression::Undefined)
            }
            "identifier" => Ok(Expression::Identifier(self.get_node_text(node))),
            "number" => {
                let text = self.get_node_text(node);
                let value = text.parse::<f64>().unwrap_or(0.0);
                Ok(Expression::Number(value))
            }
            "string" => {
                let text = self.get_node_text(node);
                // Убираем кавычки
                let content = text.trim_start_matches('"').trim_end_matches('"');
                Ok(Expression::String(content.to_string()))
            }
            "boolean" => {
                let text = self.get_node_text(node);
                let value = text == "Истина" || text.eq_ignore_ascii_case("true");
                Ok(Expression::Boolean(value))
            }
            "UNDEFINED_KEYWORD" => Ok(Expression::Undefined),
            "NULL_KEYWORD" => Ok(Expression::Null),
            "date" => {
                let text = self.get_node_text(node);
                Ok(Expression::Date(text))
            }
            "binary_expression" => self.convert_binary_expression(node),
            "unary_expression" => self.convert_unary_expression(node),
            "call_expression" | "method_call" => self.convert_call_expression(node),
            "property_access" => self.convert_member_access(node),
            "new_expression" => self.convert_new_expression(node),
            "ternary_expression" => self.convert_ternary_expression(node),
            "const_expression" => {
                // Константное выражение - смотрим на первого потомка
                let mut cursor = node.walk();
                if let Some(child) = node.children(&mut cursor).next() {
                    return self.convert_expression(child);
                }
                Ok(Expression::Undefined)
            }
            _ => {
                // Для неизвестных типов возвращаем Undefined
                // Только если это действительно неизвестный тип
                if !matches!(
                    node.kind(),
                    "(" | ")" | ";" | "," | "=" | "THEN_KEYWORD" | "ELSE_KEYWORD"
                ) {
                    println!("Unknown expression type: {}", node.kind());
                }
                Ok(Expression::Undefined)
            }
        }
    }

    /// Конвертировать объявление переменной
    fn convert_var_declaration(&self, node: Node) -> Result<Statement> {
        let mut names = Vec::new();
        let mut export = false;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    names.push(self.get_node_text(child));
                }
                "EXPORT_KEYWORD" => {
                    export = true;
                }
                _ => {}
            }
        }

        // Возвращаем первую переменную как VarDeclaration
        // TODO: Поддержка множественных объявлений
        if let Some(name) = names.into_iter().next() {
            Ok(Statement::VarDeclaration {
                name,
                value: None,
                export,
            })
        } else {
            Ok(Statement::Return(None))
        }
    }

    /// Конвертировать присваивание
    fn convert_assignment(&self, node: Node) -> Result<Statement> {
        let mut target = None;
        let mut value = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if target.is_none() && self.is_expression_node(child) {
                target = Some(self.convert_expression(child)?);
            } else if value.is_none() && self.is_expression_node(child) {
                value = Some(self.convert_expression(child)?);
            }
        }

        if let (Some(target), Some(value)) = (target, value) {
            Ok(Statement::Assignment { target, value })
        } else {
            Ok(Statement::Return(None))
        }
    }

    /// Конвертировать условный оператор
    fn convert_if_statement(&self, node: Node) -> Result<Statement> {
        let mut condition = None;
        let mut then_branch = Vec::new();
        let mut else_if_branches = Vec::new();
        let mut else_branch = None;
        let mut cursor = node.walk();

        let mut _in_then = false;
        let mut _in_else = false;
        let mut _current_elseif_condition: Option<Expression> = None;
        let mut _current_elseif_body: Vec<Statement> = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "IF_KEYWORD" => {
                    _in_then = true;
                }
                "THEN_KEYWORD" => {
                    // После THEN идёт тело then-ветки
                }
                "elseif_clause" => {
                    // Обрабатываем ИначеЕсли
                    let (elseif_cond, elseif_body) = self.convert_elseif_clause(child)?;
                    else_if_branches.push((elseif_cond, elseif_body));
                }
                "else_clause" => {
                    // Обрабатываем Иначе
                    else_branch = Some(self.convert_else_clause(child)?);
                }
                "ENDIF_KEYWORD" => {
                    // Конец условного оператора
                    break;
                }
                _ if self.is_expression_node(child) && condition.is_none() => {
                    condition = Some(self.convert_expression(child)?);
                }
                _ if self.is_statement_node(child) && !_in_else => {
                    if let Some(stmt) = self.convert_statement(child)? {
                        then_branch.push(stmt);
                    }
                }
                _ => {}
            }
        }

        Ok(Statement::If {
            condition: condition.unwrap_or(Expression::Undefined),
            then_branch,
            else_if_branches,
            else_branch,
        })
    }

    /// Конвертировать ИначеЕсли
    fn convert_elseif_clause(&self, node: Node) -> Result<(Expression, Vec<Statement>)> {
        let mut condition = None;
        let mut body = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if self.is_expression_node(child) && condition.is_none() {
                condition = Some(self.convert_expression(child)?);
            } else if self.is_statement_node(child) {
                if let Some(stmt) = self.convert_statement(child)? {
                    body.push(stmt);
                }
            }
        }

        Ok((condition.unwrap_or(Expression::Undefined), body))
    }

    /// Конвертировать Иначе
    fn convert_else_clause(&self, node: Node) -> Result<Vec<Statement>> {
        let mut body = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if self.is_statement_node(child) {
                if let Some(stmt) = self.convert_statement(child)? {
                    body.push(stmt);
                }
            }
        }

        Ok(body)
    }

    /// Конвертировать цикл While
    fn convert_while_statement(&self, node: Node) -> Result<Statement> {
        let mut condition = None;
        let mut body = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "WHILE_KEYWORD" => {
                    // Начало цикла
                }
                "DO_KEYWORD" => {
                    // После DO идёт тело цикла
                }
                "ENDDO_KEYWORD" => {
                    // Конец цикла
                    break;
                }
                _ if self.is_expression_node(child) && condition.is_none() => {
                    condition = Some(self.convert_expression(child)?);
                }
                _ if self.is_statement_node(child) => {
                    if let Some(stmt) = self.convert_statement(child)? {
                        body.push(stmt);
                    }
                }
                _ => {}
            }
        }

        Ok(Statement::While {
            condition: condition.unwrap_or(Expression::Undefined),
            body,
        })
    }

    fn convert_for_statement(&self, node: Node) -> Result<Statement> {
        let mut variable = String::new();
        let mut from = None;
        let mut to = None;
        let mut body = Vec::new();
        let mut cursor = node.walk();

        // Определяем тип цикла
        let is_foreach = node.kind() == "for_each_statement";

        if is_foreach {
            // Для каждого ... Из ...
            let mut collection = None;

            for child in node.children(&mut cursor) {
                match child.kind() {
                    "FOR_EACH_KEYWORD" => {
                        // Начало цикла "Для Каждого"
                    }
                    "identifier" if variable.is_empty() => {
                        variable = self.get_node_text(child);
                    }
                    "FROM_KEYWORD" | "IN_KEYWORD" => {
                        // После "Из" идёт коллекция
                    }
                    "DO_KEYWORD" => {
                        // После DO идёт тело цикла
                    }
                    "ENDDO_KEYWORD" => {
                        // Конец цикла
                        break;
                    }
                    _ if self.is_expression_node(child) && collection.is_none() => {
                        collection = Some(self.convert_expression(child)?);
                    }
                    _ if self.is_statement_node(child) => {
                        if let Some(stmt) = self.convert_statement(child)? {
                            body.push(stmt);
                        }
                    }
                    _ => {}
                }
            }

            Ok(Statement::ForEach {
                variable,
                collection: collection.unwrap_or(Expression::Undefined),
                body,
            })
        } else {
            // Для ... = ... По ... Цикл
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "FOR_KEYWORD" => {
                        // Начало цикла "Для"
                    }
                    "identifier" if variable.is_empty() => {
                        variable = self.get_node_text(child);
                    }
                    "TO_KEYWORD" => {
                        // После "По" идёт конечное значение
                    }
                    "DO_KEYWORD" => {
                        // После DO идёт тело цикла
                    }
                    "ENDDO_KEYWORD" => {
                        // Конец цикла
                        break;
                    }
                    _ if self.is_expression_node(child) => {
                        if from.is_none() {
                            from = Some(self.convert_expression(child)?);
                        } else if to.is_none() {
                            to = Some(self.convert_expression(child)?);
                        }
                    }
                    _ if self.is_statement_node(child) => {
                        if let Some(stmt) = self.convert_statement(child)? {
                            body.push(stmt);
                        }
                    }
                    _ => {}
                }
            }

            Ok(Statement::For {
                variable,
                from: from.unwrap_or(Expression::Number(1.0)),
                to: to.unwrap_or(Expression::Number(10.0)),
                step: None, // TODO: Поддержка шага
                body,
            })
        }
    }

    fn convert_return_statement(&self, node: Node) -> Result<Statement> {
        let mut value = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if self.is_expression_node(child) {
                value = Some(self.convert_expression(child)?);
                break;
            }
        }

        Ok(Statement::Return(value))
    }

    fn convert_call_statement(&self, _node: Node) -> Result<Statement> {
        // Пока возвращаем Return(None), позже можно добавить Call вариант
        Ok(Statement::Return(None))
    }

    fn convert_binary_expression(&self, node: Node) -> Result<Expression> {
        let mut left = None;
        let mut right = None;
        let mut op = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "operator" {
                op = Some(self.convert_binary_op(&self.get_node_text(child)));
            } else if left.is_none() {
                left = Some(self.convert_expression(child)?);
            } else if right.is_none() {
                right = Some(self.convert_expression(child)?);
            }
        }

        if let (Some(left), Some(op), Some(right)) = (left, op, right) {
            Ok(Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(Expression::Undefined)
        }
    }

    fn convert_unary_expression(&self, node: Node) -> Result<Expression> {
        let mut operand = None;
        let mut op = None;
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "operator" {
                op = Some(self.convert_unary_op(&self.get_node_text(child)));
            } else if self.is_expression_node(child) {
                operand = Some(self.convert_expression(child)?);
            }
        }

        if let (Some(op), Some(operand)) = (op, operand) {
            Ok(Expression::Unary {
                op,
                operand: Box::new(operand),
            })
        } else {
            Ok(Expression::Undefined)
        }
    }

    fn convert_call_expression(&self, node: Node) -> Result<Expression> {
        let mut function = None;
        let mut args = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" if function.is_none() => {
                    function = Some(Box::new(Expression::Identifier(self.get_node_text(child))));
                }
                "arguments" => {
                    args = self.convert_arguments(child)?;
                }
                _ => {}
            }
        }

        if let Some(function) = function {
            Ok(Expression::Call { function, args })
        } else {
            Ok(Expression::Undefined)
        }
    }

    fn convert_arguments(&self, node: Node) -> Result<Vec<Expression>> {
        let mut args = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if self.is_expression_node(child) {
                args.push(self.convert_expression(child)?);
            }
        }

        Ok(args)
    }

    fn convert_member_access(&self, node: Node) -> Result<Expression> {
        let mut object = None;
        let mut member = String::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "property" {
                member = self.get_node_text(child);
            } else if object.is_none() {
                object = Some(self.convert_expression(child)?);
            }
        }

        if let Some(object) = object {
            Ok(Expression::MemberAccess {
                object: Box::new(object),
                member,
            })
        } else {
            Ok(Expression::Undefined)
        }
    }

    fn convert_new_expression(&self, node: Node) -> Result<Expression> {
        let mut type_name = String::new();
        let mut args = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    type_name = self.get_node_text(child);
                }
                "arguments" => {
                    args = self.convert_arguments(child)?;
                }
                _ => {}
            }
        }

        Ok(Expression::New { type_name, args })
    }

    fn convert_ternary_expression(&self, node: Node) -> Result<Expression> {
        let mut condition = None;
        let mut then_expr = None;
        let mut else_expr = None;
        let mut cursor = node.walk();
        let mut index = 0;

        for child in node.children(&mut cursor) {
            if self.is_expression_node(child) {
                match index {
                    0 => condition = Some(self.convert_expression(child)?),
                    1 => then_expr = Some(self.convert_expression(child)?),
                    2 => else_expr = Some(self.convert_expression(child)?),
                    _ => {}
                }
                index += 1;
            }
        }

        if let (Some(condition), Some(then_expr), Some(else_expr)) =
            (condition, then_expr, else_expr)
        {
            Ok(Expression::Ternary {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            })
        } else {
            Ok(Expression::Undefined)
        }
    }

    /// Конвертировать бинарный оператор
    fn convert_binary_op(&self, op_text: &str) -> BinaryOp {
        match op_text {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Subtract,
            "*" => BinaryOp::Multiply,
            "/" => BinaryOp::Divide,
            "%" => BinaryOp::Modulo,
            "=" => BinaryOp::Equal,
            "<>" => BinaryOp::NotEqual,
            "<" => BinaryOp::Less,
            "<=" => BinaryOp::LessOrEqual,
            ">" => BinaryOp::Greater,
            ">=" => BinaryOp::GreaterOrEqual,
            "И" | "AND" => BinaryOp::And,
            "ИЛИ" | "OR" => BinaryOp::Or,
            _ => BinaryOp::Add, // По умолчанию
        }
    }

    /// Конвертировать унарный оператор
    fn convert_unary_op(&self, op_text: &str) -> UnaryOp {
        match op_text {
            "-" => UnaryOp::Minus,
            "НЕ" | "NOT" => UnaryOp::Not,
            _ => UnaryOp::Not, // По умолчанию
        }
    }

    /// Получить текст узла
    fn get_node_text(&self, node: Node) -> String {
        self.source[node.byte_range()].to_string()
    }

    /// Проверить, является ли узел statement
    fn is_statement_node(&self, node: Node) -> bool {
        matches!(
            node.kind(),
            "procedure_definition"
                | "function_definition"
                | "var_definition"
                | "var_statement"
                | "assignment_statement"
                | "if_statement"
                | "while_statement"
                | "for_statement"
                | "for_each_statement"
                | "return_statement"
                | "call_statement"
                | "break_statement"
                | "continue_statement"
                | "try_statement"
        )
    }

    /// Проверить, является ли узел expression
    fn is_expression_node(&self, node: Node) -> bool {
        matches!(
            node.kind(),
            "identifier"
                | "number"
                | "string"
                | "boolean"
                | "date"
                | "UNDEFINED_KEYWORD"
                | "NULL_KEYWORD"
                | "binary_expression"
                | "unary_expression"
                | "call_expression"
                | "method_call"
                | "property_access"
                | "new_expression"
                | "ternary_expression"
                | "const_expression"
                | "expression"
                | "index_access"
                | "parenthesized_expression"
                | "array_expression"
        )
    }
}

impl Parser for TreeSitterAdapter {
    fn parse(&mut self, source: &str) -> Result<Program> {
        self.parse_impl(source)
    }

    fn parse_incremental(&mut self, source: &str, changes: &[TextChange]) -> Result<Program> {
        // Применяем изменения к дереву
        if let Some(tree) = &mut self.last_tree {
            for change in changes {
                tree.edit(&tree_sitter::InputEdit {
                    start_byte: change.start_byte,
                    old_end_byte: change.old_end_byte,
                    new_end_byte: change.new_end_byte,
                    start_position: tree_sitter::Point {
                        row: change.start_position.row,
                        column: change.start_position.column,
                    },
                    old_end_position: tree_sitter::Point {
                        row: change.old_end_position.row,
                        column: change.old_end_position.column,
                    },
                    new_end_position: tree_sitter::Point {
                        row: change.new_end_position.row,
                        column: change.new_end_position.column,
                    },
                });
            }
        }

        // Парсим с использованием старого дерева
        self.parse_impl(source)
    }

    fn name(&self) -> &str {
        "tree-sitter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_assignment() {
        let mut adapter = TreeSitterAdapter::new().unwrap();
        let source = "А = 1;";
        let program = adapter.parse_impl(source).unwrap();

        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_parse_if_statement() {
        let mut adapter = TreeSitterAdapter::new().unwrap();
        let source = r#"
            Если А = 1 Тогда
                Б = 2;
            КонецЕсли;
        "#;
        let program = adapter.parse_impl(source).unwrap();

        assert!(!program.statements.is_empty());
    }
}
