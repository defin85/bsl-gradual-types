//! Конвертация statements AST -> IR
//!
//! Модуль содержит методы для конвертации различных типов statements
//! из AST представления в семантические узлы IR.

use anyhow::Result;
use bsl_shared::domain::code_location::CompilerDirective;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::ir::{Parameter, ScopeKind, SemanticNode, SemanticNodeKind};

use crate::parsing::bsl::ast::{Expression, Statement};

use super::converter::AstToIrConverter;

impl AstToIrConverter {
    /// Конвертация Statement -> SemanticNode
    ///
    /// Возвращает Option<usize> - индекс добавленного главного узла (или None если узел не добавлен).
    /// Это позволяет собирать только прямые дочерние узлы, исключая вложенные.
    pub(crate) fn convert_statement(&mut self, statement: Statement) -> Result<Option<usize>> {
        match statement {
            Statement::VarDeclaration {
                name,
                type_hint,
                span: ast_span,
            } => {
                let span = self.ast_span_to_ir_span(ast_span);

                // Phase 3: type_hint теперь TypeResolution
                let type_hint_resolution = type_hint.as_ref().map(|t| TypeResolution::explicit(t));

                let node = SemanticNode {
                    kind: SemanticNodeKind::VariableDeclaration {
                        name: name.clone(),
                        type_hint: type_hint_resolution.clone(),
                        is_export: false,
                        initial_value_type: None,
                    },
                    span,
                    scope_id: self.current_scope,
                };

                // Регистрируем переменную в function scope БЕЗ инициализации
                // VarDeclaration - это "Перем X;" без присваивания значения
                // В BSL переменные видны во всём теле функции, не только в текущем блоке
                let resolution = type_hint_resolution.unwrap_or_else(TypeResolution::unknown);
                self.symbol_table
                    .register_variable_declared_in_function_scope(self.current_scope, name, resolution, span);

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::Assignment {
                target,
                value,
                span: ast_span,
            } => {
                self.convert_assignment(target, value, ast_span)
            }

            Statement::If {
                condition,
                then_body,
                else_body,
                span: ast_span,
            } => {
                self.convert_if_statement(condition, then_body, else_body, ast_span)
            }

            Statement::While {
                condition,
                body,
                span: ast_span,
            } => {
                self.convert_while_loop(condition, body, ast_span)
            }

            Statement::For {
                variable,
                start,
                end,
                body,
                span: ast_span,
            } => {
                self.convert_for_loop(variable, start, end, body, ast_span)
            }

            Statement::ForEach {
                variable,
                collection,
                body,
                span: ast_span,
            } => {
                self.convert_foreach_loop(variable, collection, body, ast_span)
            }

            Statement::Return {
                value,
                span: ast_span,
            } => {
                // Phase 3: Используем infer_type_resolution для возвращаемого значения
                let value_type = value.as_ref().map(|v| self.infer_type_resolution(v));
                let span = self.ast_span_to_ir_span(ast_span);

                let node = SemanticNode {
                    kind: SemanticNodeKind::Return { value_type },
                    span,
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::Try {
                try_body,
                except_body,
                span: ast_span,
            } => {
                self.convert_try_statement(try_body, except_body, ast_span)
            }

            Statement::Call {
                expression,
                span: ast_span,
            } => {
                let span = self.ast_span_to_ir_span(ast_span);

                // Обрабатываем как FunctionCall
                if let Expression::Call { function, args, .. } = expression {
                    return self.convert_call_expression(*function, args, span);
                } else if let Expression::PropertyAccess {
                    object, property, span: prop_span
                } = expression
                {
                    // MILESTONE 5.5: Используем convert_property_access_expression для GlobalPropertyAccess
                    return self.convert_property_access_expression(&object, &property, prop_span);
                }
                Ok(None) // Если expression не Call и не PropertyAccess
            }

            Statement::Break { span: ast_span } => {
                let span = self.ast_span_to_ir_span(ast_span);
                let node = SemanticNode {
                    kind: SemanticNodeKind::Break,
                    span,
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::Continue { span: ast_span } => {
                let span = self.ast_span_to_ir_span(ast_span);
                let node = SemanticNode {
                    kind: SemanticNodeKind::Continue,
                    span,
                    scope_id: self.current_scope,
                };
                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::FunctionDecl {
                name,
                params,
                body,
                compiler_directive,
                span: ast_span,
            } => {
                self.convert_function_declaration(name, params, body, compiler_directive, ast_span)
            }

            Statement::ProcedureDecl {
                name,
                params,
                body,
                compiler_directive,
                span: ast_span,
            } => {
                self.convert_procedure_declaration(name, params, body, compiler_directive, ast_span)
            }

            _ => {
                // Другие statement типы пока пропускаем
                // TODO: Добавить Goto, Label, Execute, RaiseError, AddHandler, RemoveHandler, Await
                Ok(None)
            }
        }
    }

    /// Конвертация Assignment statement
    fn convert_assignment(
        &mut self,
        target: Expression,
        value: Expression,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        if let Expression::Identifier { name: var_name, .. } = target {
            // ИСПРАВЛЕНИЕ Milestone 3.5 + 3.16: Обрабатываем value expression ПЕРЕД Assignment
            // Это создаст промежуточные узлы (FunctionCall, MemberAccess) для hover и валидации
            let value_node_idx = match &value {
                Expression::Call {
                    function,
                    args,
                    span: call_span,
                } => self.convert_call_expression(*function.clone(), args.clone(), *call_span)?,

                // MILESTONE 3.16 + 5.5: Обрабатываем PropertyAccess для валидации метаданных
                // Например: Док = Документы.ЗаказКлиента
                Expression::PropertyAccess {
                    object,
                    property,
                    span: prop_span,
                } => {
                    self.convert_property_access_expression(object, property, *prop_span)?
                }

                _ => None,
            };

            let span = self.ast_span_to_ir_span(ast_span);

            // FIX: Используем infer_type_resolution для получения полного TypeResolution
            // Это обеспечивает согласованность между Symbol Table и SemanticNode
            let value_type_resolution = self.infer_type_resolution(&value);

            // КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Определяем тип для переменной
            // Phase 3: Используем ref для избежания partial move
            let type_resolution = if let Expression::New { ref type_name, .. } = value {
                // ОЧИСТКА: Убираем скобки если tree-sitter включил их в type_name
                let clean_type_name = type_name.trim().trim_end_matches("()").trim();

                // Для Generic коллекций (Массив, Соответствие, Список)
                use bsl_shared::domain::types::Certainty;
                match clean_type_name {
                    "Массив" => TypeResolution::generic("Массив", &["?"], Certainty::InferredWeak),
                    "Соответствие" => TypeResolution::generic("Соответствие", &["?", "?"], Certainty::InferredWeak),
                    "Список" => TypeResolution::generic("Список", &["?"], Certainty::InferredWeak),
                    _ => {
                        // ИСПРАВЛЕНИЕ: ДЛЯ ВСЕХ ОСТАЛЬНЫХ ТИПОВ создаём Explicit!
                        TypeResolution::explicit(clean_type_name)
                    }
                }
            } else {
                // FIX: Используем уже вычисленный value_type_resolution
                value_type_resolution.clone()
            };

            // КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Проверяем, существует ли переменная
            let variable_exists = self
                .symbol_table
                .get_variable_type(self.current_scope, &var_name)
                .is_some();

            if !variable_exists {
                // Переменная не объявлена через VarDeclaration -> регистрируем её
                // В BSL переменные видны во всём теле функции (function scope)
                use tracing::debug;
                debug!(
                    "Assignment declares new variable: {} with type {:?}",
                    var_name, type_resolution
                );
                self.symbol_table.register_variable_in_function_scope(
                    self.current_scope,
                    var_name.clone(),
                    type_resolution,
                    span,
                );
            } else {
                // Переменная уже существует -> обновляем тип (flow-sensitive)
                // Используем публичный API вместо прямого доступа к scopes
                if !self.symbol_table.update_variable_type(
                    self.current_scope,
                    var_name.clone(),
                    type_resolution,
                ) {
                    tracing::warn!(
                        "Failed to update variable type for '{}' in scope {:?}",
                        var_name,
                        self.current_scope
                    );
                }
            }

            // Phase 3: value_type теперь TypeResolution
            // FIX: Используем уже вычисленный value_type_resolution
            let node = SemanticNode {
                kind: SemanticNodeKind::Assignment {
                    variable: var_name.clone(),
                    // Phase 3: value_type теперь TypeResolution
                    value_type: value_type_resolution,
                    value_node: value_node_idx, // MILESTONE 3.5: сохраняем индекс узла value
                },
                span,
                scope_id: self.current_scope,
            };

            self.nodes.push(node);
            return Ok(Some(self.nodes.len() - 1));
        }
        Ok(None) // Если target не Identifier
    }

    /// Конвертация If statement
    fn convert_if_statement(
        &mut self,
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        // Phase 3: Используем infer_type_resolution для условия
        let condition_type = self.infer_type_resolution(&condition);
        let span = self.ast_span_to_ir_span(ast_span);

        // Создаём scope для then ветки
        let then_scope = self.symbol_table.create_scope(self.current_scope);
        let old_scope = self.current_scope;
        self.current_scope = then_scope;

        // Собираем только прямые дочерние индексы
        let mut then_indices = Vec::new();
        for stmt in then_body {
            if let Some(idx) = self.convert_statement(stmt)? {
                then_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        // Создаём scope для else ветки
        let else_indices = if let Some(else_stmts) = else_body {
            let else_scope = self.symbol_table.create_scope(self.current_scope);
            self.current_scope = else_scope;

            // Собираем только прямые дочерние индексы
            let mut indices = Vec::new();
            for stmt in else_stmts {
                if let Some(idx) = self.convert_statement(stmt)? {
                    indices.push(idx);
                }
            }

            self.current_scope = old_scope;
            Some(indices)
        } else {
            None
        };

        let node = SemanticNode {
            kind: SemanticNodeKind::IfStatement {
                condition_type,
                then_branch: then_indices,
                else_branch: else_indices,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }

    /// Конвертация While loop
    fn convert_while_loop(
        &mut self,
        condition: Expression,
        body: Vec<Statement>,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        // Phase 3: Используем infer_type_resolution для условия
        let condition_type = self.infer_type_resolution(&condition);
        let span = self.ast_span_to_ir_span(ast_span);

        let body_scope = self.symbol_table.create_scope(self.current_scope);
        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        // Собираем только прямые дочерние индексы
        let mut body_indices = Vec::new();
        for stmt in body {
            if let Some(idx) = self.convert_statement(stmt)? {
                body_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        let node = SemanticNode {
            kind: SemanticNodeKind::WhileLoop {
                condition_type,
                body: body_indices,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }

    /// Конвертация For loop
    fn convert_for_loop(
        &mut self,
        variable: String,
        start: Expression,
        end: Expression,
        body: Vec<Statement>,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        // Phase 3: For loop range всегда числовой
        let range_type = TypeResolution::primitive("Число");
        let span = self.ast_span_to_ir_span(ast_span);

        // Debug info о start/end типах (для будущей валидации)
        let _start_type = self.infer_expression_type(&start);
        let _end_type = self.infer_expression_type(&end);

        let body_scope = self.symbol_table.create_scope(self.current_scope);
        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        // Регистрируем переменную цикла
        self.symbol_table.register_variable(
            self.current_scope,
            variable.clone(),
            TypeResolution::primitive("Число"),
            span,
        );

        // Собираем только прямые дочерние индексы
        let mut body_indices = Vec::new();
        for stmt in body {
            if let Some(idx) = self.convert_statement(stmt)? {
                body_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        let node = SemanticNode {
            kind: SemanticNodeKind::ForLoop {
                variable,
                range_type,
                body: body_indices,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }

    /// Конвертация ForEach loop
    fn convert_foreach_loop(
        &mut self,
        variable: String,
        collection: Expression,
        body: Vec<Statement>,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        // Phase 3: Используем infer_type_resolution для коллекции
        let collection_type = self.infer_type_resolution(&collection);
        let span = self.ast_span_to_ir_span(ast_span);

        let body_scope = self.symbol_table.create_scope(self.current_scope);
        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        // Собираем только прямые дочерние индексы
        let mut body_indices = Vec::new();
        for stmt in body {
            if let Some(idx) = self.convert_statement(stmt)? {
                body_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        let node = SemanticNode {
            kind: SemanticNodeKind::ForEachLoop {
                variable,
                collection_type,
                body: body_indices,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }

    /// Конвертация Try statement
    fn convert_try_statement(
        &mut self,
        try_body: Vec<Statement>,
        except_body: Vec<Statement>,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);

        // Try scope
        let try_scope = self.symbol_table.create_scope(self.current_scope);
        let old_scope = self.current_scope;
        self.current_scope = try_scope;

        // Собираем только прямые дочерние индексы
        let mut try_indices = Vec::new();
        for stmt in try_body {
            if let Some(idx) = self.convert_statement(stmt)? {
                try_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        // Except scope
        let except_scope = self.symbol_table.create_scope(self.current_scope);
        self.current_scope = except_scope;

        // Собираем только прямые дочерние индексы
        let mut except_indices = Vec::new();
        for stmt in except_body {
            if let Some(idx) = self.convert_statement(stmt)? {
                except_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        let node = SemanticNode {
            kind: SemanticNodeKind::TryExcept {
                try_body: try_indices,
                except_body: except_indices,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }

    /// Конвертация Function declaration
    fn convert_function_declaration(
        &mut self,
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        compiler_directive: Option<CompilerDirective>,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);
        // Function scope для корректной регистрации переменных (видны во всём теле функции)
        let body_scope = self.symbol_table.create_scope_with_kind(self.current_scope, ScopeKind::Function);

        // Phase 3: Parameter.type_hint теперь Option<TypeResolution>
        let params_vec: Vec<Parameter> = params
            .iter()
            .map(|p| Parameter {
                name: p.clone(),
                type_hint: None, // Phase 3: TypeResolution
                default_value: None,
                is_val: false,
            })
            .collect();

        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        // Регистрируем параметры функции в body_scope
        for param in &params_vec {
            self.symbol_table.register_variable(
                body_scope,
                param.name.clone(),
                TypeResolution::unknown(), // Градуальный тип (пока Unknown)
                span,
            );
        }

        // Собираем только прямые дочерние индексы
        let mut body_indices = Vec::new();
        for stmt in body {
            if let Some(idx) = self.convert_statement(stmt)? {
                body_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        // Phase 3: return_type теперь Option<TypeResolution>
        // Context-Aware: передаём директиву компилятора для валидации
        let node = SemanticNode {
            kind: SemanticNodeKind::FunctionDeclaration {
                name,
                params: params_vec,
                return_type: None, // Phase 3: TypeResolution, будет выведен из return
                body_scope,
                body: body_indices,
                compiler_directive, // Context-Aware валидация
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }

    /// Конвертация Procedure declaration
    fn convert_procedure_declaration(
        &mut self,
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        compiler_directive: Option<CompilerDirective>,
        ast_span: crate::parsing::bsl::ast::Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);
        // Function scope для корректной регистрации переменных (видны во всём теле процедуры)
        let body_scope = self.symbol_table.create_scope_with_kind(self.current_scope, ScopeKind::Function);

        // Phase 3: Parameter.type_hint теперь Option<TypeResolution>
        let params_vec: Vec<Parameter> = params
            .iter()
            .map(|p| Parameter {
                name: p.clone(),
                type_hint: None, // Phase 3: TypeResolution
                default_value: None,
                is_val: false,
            })
            .collect();

        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        // Регистрируем параметры процедуры в body_scope
        for param in &params_vec {
            self.symbol_table.register_variable(
                body_scope,
                param.name.clone(),
                TypeResolution::unknown(), // Градуальный тип (пока Unknown)
                span,
            );
        }

        // Собираем только прямые дочерние индексы
        let mut body_indices = Vec::new();
        for stmt in body {
            if let Some(idx) = self.convert_statement(stmt)? {
                body_indices.push(idx);
            }
        }

        self.current_scope = old_scope;

        // Context-Aware: передаём директиву компилятора для валидации
        let node = SemanticNode {
            kind: SemanticNodeKind::ProcedureDeclaration {
                name,
                params: params_vec,
                body_scope,
                body: body_indices,
                compiler_directive, // Context-Aware валидация
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1))
    }
}
