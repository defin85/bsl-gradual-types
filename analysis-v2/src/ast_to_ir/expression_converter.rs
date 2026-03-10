//! Конвертация выражений AST -> IR
//!
//! Модуль содержит методы для конвертации различных типов выражений
//! из AST представления в семантические узлы IR.

use anyhow::Result;
use bsl_shared::ir::{MemberAccessKind, SemanticNode, SemanticNodeKind, Span};

use bsl_syntax::ast::Expression;

use super::converter::AstToIrConverter;
use super::global_collections::is_global_collection;

fn expression_ast_span(expr: &Expression) -> Span {
    match expr {
        Expression::Identifier { span, .. }
        | Expression::String { span, .. }
        | Expression::Number { span, .. }
        | Expression::Boolean { span, .. }
        | Expression::Date { span, .. }
        | Expression::Call { span, .. }
        | Expression::Binary { span, .. }
        | Expression::Unary { span, .. }
        | Expression::Ternary { span, .. }
        | Expression::New { span, .. }
        | Expression::PropertyAccess { span, .. }
        | Expression::IndexAccess { span, .. }
        | Expression::Await { span, .. } => *span,
    }
}

impl AstToIrConverter {
    /// Создаёт IR-узлы для hover внутри выражений
    ///
    /// Обходит выражение рекурсивно и конвертирует Call/PropertyAccess,
    /// чтобы hover работал на вложенных вызовах и доступах.
    pub(crate) fn convert_expression_for_hover(
        &mut self,
        expr: &Expression,
    ) -> Result<Option<usize>> {
        match expr {
            Expression::Call {
                function,
                args,
                span,
            } => self.convert_call_expression(*function.clone(), args.clone(), *span),
            Expression::PropertyAccess {
                object,
                property,
                span,
            } => {
                let node_idx = self.convert_property_access_expression(object, property, *span)?;

                Ok(node_idx)
            }
            Expression::Identifier { name, span } => {
                let name_lower = name.to_lowercase();
                if matches!(name_lower.as_str(), "истина" | "ложь" | "true" | "false") {
                    return Ok(None);
                }

                if name_lower == "null" {
                    let node = SemanticNode {
                        kind: SemanticNodeKind::NullLiteral,
                        span: self.ast_span_to_ir_span(*span),
                        scope_id: self.current_scope,
                    };
                    self.nodes.push(node);
                    return Ok(Some(self.nodes.len() - 1));
                }

                if matches!(name_lower.as_str(), "неопределено" | "undefined") {
                    let node = SemanticNode {
                        kind: SemanticNodeKind::UndefinedLiteral,
                        span: self.ast_span_to_ir_span(*span),
                        scope_id: self.current_scope,
                    };
                    self.nodes.push(node);
                    return Ok(Some(self.nodes.len() - 1));
                }

                let node = SemanticNode {
                    kind: SemanticNodeKind::VariableAccess { name: name.clone() },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::String { value, span } => {
                let node = SemanticNode {
                    kind: SemanticNodeKind::StringLiteral {
                        value: value.clone(),
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Number { value, span } => {
                let node = SemanticNode {
                    kind: SemanticNodeKind::NumberLiteral { value: *value },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Boolean { value, span } => {
                let node = SemanticNode {
                    kind: SemanticNodeKind::BooleanLiteral { value: *value },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Date { value, span } => {
                let node = SemanticNode {
                    kind: SemanticNodeKind::DateLiteral {
                        value: value.clone(),
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Binary { .. } => {
                let Expression::Binary {
                    left,
                    operator,
                    right,
                    span,
                } = expr
                else {
                    return Ok(None);
                };

                let left_node = self.convert_expression_for_hover(left)?;
                let right_node = self.convert_expression_for_hover(right)?;

                let node = SemanticNode {
                    kind: SemanticNodeKind::BinaryExpression {
                        operator: operator.clone(),
                        left_node,
                        right_node,
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Unary { .. } => {
                let Expression::Unary {
                    operator,
                    operand,
                    span,
                } = expr
                else {
                    return Ok(None);
                };

                let operand_node = self.convert_expression_for_hover(operand)?;
                let node = SemanticNode {
                    kind: SemanticNodeKind::UnaryExpression {
                        operator: operator.clone(),
                        operand_node,
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                span,
            } => {
                let condition_node = self.convert_expression_for_hover(condition)?;
                let then_node = self.convert_expression_for_hover(then_expr)?;
                let else_node = self.convert_expression_for_hover(else_expr)?;
                let node = SemanticNode {
                    kind: SemanticNodeKind::TernaryExpression {
                        condition_node,
                        then_node,
                        else_node,
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::New { .. } => {
                let Expression::New {
                    type_name,
                    args,
                    span,
                } = expr
                else {
                    return Ok(None);
                };

                let mut type_name = type_name.trim().to_string();
                let is_dynamic = type_name.starts_with('"') && type_name.ends_with('"');
                if is_dynamic && type_name.len() >= 2 {
                    type_name = type_name[1..type_name.len() - 1].to_string();
                }

                let arg_nodes = args
                    .iter()
                    .map(|arg| self.convert_expression_for_hover(arg))
                    .collect::<Result<Vec<_>>>()?;

                let node = SemanticNode {
                    kind: SemanticNodeKind::NewExpression {
                        type_name,
                        generic_params: None,
                        is_dynamic,
                        arg_nodes,
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::IndexAccess { object, index, .. } => {
                let object_span = self.ast_span_to_ir_span(expression_ast_span(object));
                let index_span = self.ast_span_to_ir_span(expression_ast_span(index));
                let object_name = if let Expression::Identifier { name, .. } = &**object {
                    Some(name.clone())
                } else {
                    None
                };
                let object_node = match &**object {
                    Expression::PropertyAccess {
                        object: inner_obj,
                        property,
                        span,
                    } => self.convert_property_access_expression(inner_obj, property, *span)?,
                    Expression::IndexAccess { .. } => self.convert_expression_for_hover(object)?,
                    _ if object_name.is_none() => self.convert_expression_for_hover(object)?,
                    _ => None,
                };
                let index_node = self.convert_expression_for_hover(index)?;

                let node = SemanticNode {
                    kind: SemanticNodeKind::IndexAccess {
                        object_node,
                        object_name,
                        object_span: Some(object_span),
                        index_node,
                        index_span: Some(index_span),
                    },
                    span: self.ast_span_to_ir_span(expression_ast_span(expr)),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Await { .. } => {
                let Expression::Await { expression, span } = expr else {
                    return Ok(None);
                };

                let expression_node = self.convert_expression_for_hover(expression)?;
                let node = SemanticNode {
                    kind: SemanticNodeKind::AwaitExpression { expression_node },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
        }
    }

    /// Конвертация вызова функции
    pub(crate) fn convert_call_expression(
        &mut self,
        function: Expression,
        args: Vec<Expression>,
        ast_span: Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);
        let arg_nodes = args
            .iter()
            .map(|arg| self.convert_expression_for_hover(arg))
            .collect::<Result<Vec<_>>>()?;
        let arg_spans = args
            .iter()
            .map(expression_ast_span)
            .map(|span| self.ast_span_to_ir_span(span))
            .collect();

        match function {
            Expression::Identifier { name, .. } => {
                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionCall {
                        function_name: name,
                        object_name: None,
                        object_node: None,
                        object_span: None,
                        arg_nodes,
                        arg_spans,
                    },
                    span,
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let object_span = self.ast_span_to_ir_span(expression_ast_span(&object));
                let object_name = if let Expression::Identifier { name, .. } = &*object {
                    Some(name.clone())
                } else {
                    None
                };

                // Для цепочек вызовов сохраняем ссылку на вложенный объектный узел.
                let object_node = if object_name.is_some() {
                    let _ = self.convert_expression_for_hover(&object)?;
                    None
                } else {
                    self.convert_expression_for_hover(&object)?
                };

                // Расширяем span вызова, чтобы он включал объект (для hover).
                let expanded_span = if let Expression::Identifier { span: obj_span, .. } = &*object
                {
                    Span::new(obj_span.start, span.end)
                } else {
                    span
                };

                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionCall {
                        function_name: property,
                        object_name,
                        object_node,
                        object_span: Some(object_span),
                        arg_nodes,
                        arg_spans,
                    },
                    span: expanded_span,
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            _ => Ok(None),
        }
    }

    /// Конвертация PropertyAccess выражения с поддержкой GlobalPropertyAccess
    ///
    /// MILESTONE 5.5: Создаёт GlobalPropertyAccess для глобальных коллекций (Справочники, Документы и т.д.)
    /// и MemberAccess с object_node для построения иерархии.
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// Справочники.Контрагенты
    /// // Создаёт:
    /// // 1. GlobalPropertyAccess { name: "Справочники", result_type: СправочникиМенеджер }
    /// // 2. MemberAccess { object_node: Some(1), member_name: "Контрагенты", result_type: СправочникМенеджер.Контрагенты }
    /// ```
    pub(crate) fn convert_property_access_expression(
        &mut self,
        object: &Expression,
        property: &str,
        ast_span: Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);

        // Проверяем, является ли object глобальной коллекцией (Справочники, Документы и т.д.)
        if let Expression::Identifier {
            name,
            span: obj_span,
        } = object
        {
            if is_global_collection(name).is_some() {
                // MILESTONE 5.5: Создаём GlobalPropertyAccess узел
                let global_node = SemanticNode {
                    kind: SemanticNodeKind::GlobalPropertyAccess { name: name.clone() },
                    span: self.ast_span_to_ir_span(*obj_span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(global_node);
                let global_node_idx = self.nodes.len() - 1;

                // Создаём MemberAccess с object_node указывающим на GlobalPropertyAccess
                let member_node = SemanticNode {
                    kind: SemanticNodeKind::MemberAccess {
                        object_node: Some(global_node_idx),
                        object_name: None, // Не нужен - используем object_node
                        object_span: Some(self.ast_span_to_ir_span(*obj_span)),
                        member_name: property.to_string(),
                        access_kind: MemberAccessKind::Property,
                    },
                    span,
                    scope_id: self.current_scope,
                };
                self.nodes.push(member_node);
                return Ok(Some(self.nodes.len() - 1));
            }
        }

        // Обычный PropertyAccess (не глобальная коллекция)
        let object_name = match object {
            Expression::Identifier { name, .. } => Some(name.clone()),
            _ => None,
        };

        // Для вложенных PropertyAccess рекурсивно обрабатываем object
        let object_node_idx = match object {
            Expression::PropertyAccess {
                object: inner_obj,
                property: inner_prop,
                span: inner_span,
            } => self.convert_property_access_expression(inner_obj, inner_prop, *inner_span)?,
            _ if object_name.is_none() => self.convert_expression_for_hover(object)?,
            _ => None,
        };

        let node = SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_node: object_node_idx,
                object_name,
                object_span: Some(self.ast_span_to_ir_span(expression_ast_span(object))),
                member_name: property.to_string(),
                access_kind: MemberAccessKind::Property,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }
}
