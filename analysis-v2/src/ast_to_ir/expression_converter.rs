//! Конвертация выражений AST -> IR
//!
//! Модуль содержит методы для конвертации различных типов выражений
//! из AST представления в семантические узлы IR.

use anyhow::Result;
use bsl_shared::ir::{MemberAccessKind, SemanticNode, SemanticNodeKind, Span};

use bsl_syntax::ast::Expression;

use super::converter::AstToIrConverter;
use super::global_collections::is_global_collection;

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
            } => {
                let node_idx =
                    self.convert_call_expression(*function.clone(), args.clone(), *span)?;

                for arg in args {
                    self.convert_expression_for_hover(arg)?;
                }

                Ok(node_idx)
            }
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
                if matches!(
                    name_lower.as_str(),
                    "неопределено" | "null" | "истина" | "ложь" | "true" | "false"
                ) {
                    return Ok(None);
                }

                let node = SemanticNode {
                    kind: SemanticNodeKind::VariableAccess { name: name.clone() },
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

                self.convert_expression_for_hover(left)?;
                self.convert_expression_for_hover(right)?;

                let node = SemanticNode {
                    kind: SemanticNodeKind::BinaryExpression {
                        operator: operator.clone(),
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }
            Expression::Unary { operand, .. } => {
                self.convert_expression_for_hover(operand)?;
                Ok(None)
            }
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.convert_expression_for_hover(condition)?;
                self.convert_expression_for_hover(then_expr)?;
                self.convert_expression_for_hover(else_expr)?;
                Ok(None)
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

                let node = SemanticNode {
                    kind: SemanticNodeKind::NewExpression {
                        type_name,
                        generic_params: None,
                        is_dynamic,
                    },
                    span: self.ast_span_to_ir_span(*span),
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                let node_idx = self.nodes.len() - 1;

                for arg in args {
                    self.convert_expression_for_hover(arg)?;
                }

                Ok(Some(node_idx))
            }
            Expression::IndexAccess { object, index, .. } => {
                self.convert_expression_for_hover(object)?;
                self.convert_expression_for_hover(index)?;
                Ok(None)
            }
            Expression::Await { expression, .. } => {
                self.convert_expression_for_hover(expression)?;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Конвертация вызова функции
    pub(crate) fn convert_call_expression(
        &mut self,
        function: Expression,
        _args: Vec<Expression>,
        ast_span: Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);

        match function {
            Expression::Identifier { name, .. } => {
                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionCall {
                        function_name: name,
                        object_name: None,
                        object_node: None,
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
