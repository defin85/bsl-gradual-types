//! Конвертация выражений AST -> IR
//!
//! Модуль содержит методы для конвертации различных типов выражений
//! из AST представления в семантические узлы IR.

use anyhow::Result;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::ir::{MemberAccessKind, SemanticNode, SemanticNodeKind, Span};

use crate::parsing::bsl::ast::Expression;

use super::converter::AstToIrConverter;
use super::global_collections::{get_manager_type_for_metadata, is_global_collection};

impl AstToIrConverter {
    /// Конвертация вызова функции
    pub(crate) fn convert_call_expression(
        &mut self,
        function: Expression,
        args: Vec<Expression>,
        span: Span,
    ) -> Result<Option<usize>> {
        // Возвращаем индекс созданного узла
        let function_name = match function {
            Expression::Identifier { name, .. } => name,
            Expression::PropertyAccess {
                object, property, ..
            } => {
                // MILESTONE 5.4: Создаём дерево для цепочек методов
                // Например для цепочки: Справочники.Контрагенты.НайтиПоКоду("001").ПолучитьОбъект()
                // Создаётся иерархия:
                //   FunctionCall ПолучитьОбъект
                //     |_ FunctionCall НайтиПоКоду
                //          |_ MemberAccess Справочники.Контрагенты
                let object_node: Option<usize> = match object.as_ref() {
                    // Вложенный вызов метода: .НайтиПоКоду().ПолучитьОбъект()
                    Expression::Call {
                        function: inner_function,
                        args: inner_args,
                        span: inner_span,
                    } => {
                        self.convert_call_expression(
                            *inner_function.clone(),
                            inner_args.clone(),
                            *inner_span,
                        )?
                    }
                    // PropertyAccess: Справочники.Контрагенты.НайтиПоКоду()
                    // MILESTONE 5.5: Используем convert_property_access_expression для GlobalPropertyAccess
                    Expression::PropertyAccess {
                        object: inner_object,
                        property: inner_property,
                        span: inner_span,
                    } => {
                        self.convert_property_access_expression(inner_object, inner_property, *inner_span)?
                    }
                    _ => None,
                };

                // Метод объекта: объект.Метод()
                // Phase 3: Используем infer_type_resolution для полной информации
                // MILESTONE 5.6: Для цепочек вызовов берём object_type из result_type дочернего узла
                // Это критично для: Ссылка.Работы.Выгрузить() где .Работы имеет result_type "ТабличнаяЧасть<Работы>"
                let object_type = if let Some(child_idx) = object_node {
                    if let Some(node) = self.nodes.get(child_idx) {
                        match &node.kind {
                            SemanticNodeKind::MemberAccess { result_type, .. } => result_type.clone(),
                            SemanticNodeKind::FunctionCall { result_type, .. } => result_type.clone(),
                            _ => self.infer_type_resolution(&object)
                        }
                    } else {
                        self.infer_type_resolution(&object)
                    }
                } else {
                    self.infer_type_resolution(&object)
                };

                // Для Generic inference всё ещё нужны String типы
                let arg_types_str: Vec<String> = args
                    .iter()
                    .map(|arg| self.infer_expression_type(arg))
                    .collect();

                // Phase 3: arg_types как Vec<TypeResolution>
                let arg_types: Vec<TypeResolution> = args
                    .iter()
                    .map(|arg| self.infer_type_resolution(arg))
                    .collect();

                // НОВОЕ: Generic inference из вызова метода
                // Если это вызов метода переменной (а не выражения),
                // пытаемся вывести Generic тип (используем arg_types_str для совместимости)
                let object_name = if let Expression::Identifier { name, .. } = &*object {
                    self.try_infer_generic_from_method_call(name, &property, &arg_types_str);
                    Some(name.clone())
                } else {
                    None
                };

                // MILESTONE 2.11: Расширяем span FunctionCall узла, чтобы включить объект
                // Это позволит hover правильно работать на объекте в вызове метода
                let expanded_span = if let Expression::Identifier { span: obj_span, .. } = &*object {
                    // Объединяем span объекта (ТаблицаТип) и span вызова (Количество())
                    Span {
                        start_line: obj_span.start_line,
                        start_column: obj_span.start_column,
                        end_line: span.end_line,
                        end_column: span.end_column,
                    }
                } else {
                    span // Для сложных выражений используем оригинальный span
                };

                // MILESTONE 5.6: Вычисляем result_type для FunctionCall через SignatureIndex
                // Это критично для цепочек: Ссылка.Работы.Выгрузить() -> ТаблицаЗначений
                let result_type = self.resolve_method_return_type(&object_type, &property);

                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionCall {
                        function_name: property.clone(),
                        object_name, // Имя объекта для методов
                        // Phase 3: object_type теперь TypeResolution
                        object_type: Some(object_type),
                        // Phase 3: arg_types теперь Vec<TypeResolution>
                        arg_types,
                        // MILESTONE 5.4: ссылка на вложенный узел для иерархии
                        object_node,
                        // MILESTONE 5.6: тип возвращаемого значения из SignatureIndex
                        result_type,
                    },
                    span: expanded_span, // Используем расширенный span
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                return Ok(Some(self.nodes.len() - 1)); // Возвращаем индекс
            }
            _ => "Unknown".to_string(),
        };

        // Phase 3: arg_types как Vec<TypeResolution>
        let arg_types: Vec<TypeResolution> = args
            .iter()
            .map(|arg| self.infer_type_resolution(arg))
            .collect();

        let node = SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name,
                object_name: None, // Обычная функция, не метод
                object_type: None,
                // Phase 3: arg_types теперь Vec<TypeResolution>
                arg_types,
                object_node: None, // Нет вложенного узла для обычных функций
                // НОВОЕ: тип возвращаемого значения (TODO: resolve from function signature)
                result_type: TypeResolution::unknown(),
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1)) // Возвращаем индекс
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
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);

        // Проверяем, является ли object глобальной коллекцией (Справочники, Документы и т.д.)
        if let Expression::Identifier { name, span: obj_span } = object {
            if let Some(collection_manager_type) = is_global_collection(name) {
                // MILESTONE 5.5: Создаём GlobalPropertyAccess узел
                let global_node = SemanticNode {
                    kind: SemanticNodeKind::GlobalPropertyAccess {
                        name: name.clone(),
                        result_type: TypeResolution::explicit(collection_manager_type),
                    },
                    span: self.ast_span_to_ir_span(*obj_span),
                    scope_id: self.current_scope,
                };
                self.nodes.push(global_node);
                let global_node_idx = self.nodes.len() - 1;

                // Вычисляем result_type для MemberAccess
                // Для Справочники.Контрагенты -> СправочникМенеджер.Контрагенты
                let result_type = TypeResolution::explicit(&get_manager_type_for_metadata(name, property));

                // Создаём MemberAccess с object_node указывающим на GlobalPropertyAccess
                let member_node = SemanticNode {
                    kind: SemanticNodeKind::MemberAccess {
                        object_node: Some(global_node_idx),
                        object_name: None, // Не нужен - используем object_node
                        object_type: TypeResolution::explicit(collection_manager_type),
                        member_name: property.to_string(),
                        access_kind: MemberAccessKind::Property,
                        result_type,
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

        // Phase 3: Инферим тип объекта с TypeResolution
        let object_type = self.infer_type_resolution(object);

        // Для вложенных PropertyAccess рекурсивно обрабатываем object
        let object_node_idx = match object {
            Expression::PropertyAccess {
                object: inner_obj,
                property: inner_prop,
                span: inner_span,
            } => self.convert_property_access_expression(inner_obj, inner_prop, *inner_span)?,
            _ => None,
        };

        // Резолвим тип свойства
        let result_type = self.resolve_member_type(&object_type, property);

        let node = SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_node: object_node_idx,
                object_name,
                object_type: object_type.clone(),
                member_name: property.to_string(),
                access_kind: MemberAccessKind::Property,
                result_type,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }
}
