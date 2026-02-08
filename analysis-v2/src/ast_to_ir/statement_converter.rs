//! Конвертация statements AST -> IR
//!
//! Модуль содержит методы для конвертации различных типов statements
//! из AST представления в семантические узлы IR.

use anyhow::Result;
use bsl_shared::domain::code_location::CompilerDirective;
use bsl_shared::ir::Span;
use bsl_shared::ir::{Parameter, ScopeKind, SemanticNode, SemanticNodeKind};

use bsl_syntax::ast::{Expression, Statement};

use crate::implicit_bindings::directive_disables_form_context;

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

                let node = SemanticNode {
                    kind: SemanticNodeKind::VariableDeclaration {
                        name: name.clone(),
                        type_hint,
                        is_export: false,
                        initial_value_node: None,
                    },
                    span,
                    scope_id: self.current_scope,
                };

                // Регистрируем переменную в function scope БЕЗ инициализации
                // VarDeclaration - это "Перем X;" без присваивания значения
                // В BSL переменные видны во всём теле функции, не только в текущем блоке
                self.symbol_table
                    .register_variable_declared_in_function_scope(self.current_scope, name, span);

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::Assignment {
                target,
                value,
                span: ast_span,
            } => self.convert_assignment(target, value, ast_span),

            Statement::If {
                condition,
                then_body,
                else_body,
                span: ast_span,
            } => self.convert_if_statement(condition, then_body, else_body, ast_span),

            Statement::While {
                condition,
                body,
                span: ast_span,
            } => self.convert_while_loop(condition, body, ast_span),

            Statement::For {
                variable,
                start,
                end,
                body,
                span: ast_span,
            } => self.convert_for_loop(variable, start, end, body, ast_span),

            Statement::ForEach {
                variable,
                collection,
                body,
                span: ast_span,
            } => self.convert_foreach_loop(variable, collection, body, ast_span),

            Statement::Return {
                value,
                span: ast_span,
            } => {
                let value_node = if let Some(ref expr) = value {
                    self.convert_expression_for_hover(expr)?
                } else {
                    None
                };
                let span = self.ast_span_to_ir_span(ast_span);

                let node = SemanticNode {
                    kind: SemanticNodeKind::Return { value_node },
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
            } => self.convert_try_statement(try_body, except_body, ast_span),

            Statement::Call {
                expression,
                span: _ast_span,
            } => {
                let node_idx = self.convert_expression_for_hover(&expression)?;
                Ok(node_idx)
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
                is_export: _,
                span: ast_span,
            } => {
                self.convert_function_declaration(name, params, body, compiler_directive, ast_span)
            }

            Statement::ProcedureDecl {
                name,
                params,
                body,
                compiler_directive,
                is_export: _,
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
        ast_span: Span,
    ) -> Result<Option<usize>> {
        fn expression_span(expr: &Expression) -> Span {
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

        if let Expression::Identifier { name: var_name, .. } = target {
            let value_span = self.ast_span_to_ir_span(expression_span(&value));

            // ИСПРАВЛЕНИЕ Milestone 3.5 + 3.16: Обрабатываем value expression ПЕРЕД Assignment
            // Это создаст промежуточные узлы (FunctionCall, MemberAccess) для hover и валидации
            let value_node_idx = self.convert_expression_for_hover(&value)?;

            let span = self.ast_span_to_ir_span(ast_span);

            // Помечаем переменную как инициализированную. Если переменной ещё нет — регистрируем.
            if let Some((decl_scope_id, _)) = self
                .symbol_table
                .lookup_variable_in_hierarchy(self.current_scope, &var_name)
            {
                let _ = self
                    .symbol_table
                    .mark_variable_initialized(decl_scope_id, &var_name);
            } else {
                self.symbol_table.register_variable_in_function_scope(
                    self.current_scope,
                    var_name.clone(),
                    span,
                );
            }

            let node = SemanticNode {
                kind: SemanticNodeKind::Assignment {
                    variable: var_name.clone(),
                    value_node: value_node_idx, // MILESTONE 3.5: сохраняем индекс узла value
                    value_span,
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
        ast_span: Span,
    ) -> Result<Option<usize>> {
        self.convert_expression_for_hover(&condition)?;

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
        ast_span: Span,
    ) -> Result<Option<usize>> {
        self.convert_expression_for_hover(&condition)?;

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
            kind: SemanticNodeKind::WhileLoop { body: body_indices },
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
        ast_span: Span,
    ) -> Result<Option<usize>> {
        self.convert_expression_for_hover(&start)?;
        self.convert_expression_for_hover(&end)?;

        let span = self.ast_span_to_ir_span(ast_span);

        let body_scope = self.symbol_table.create_scope(self.current_scope);
        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        // Регистрируем переменную цикла
        self.symbol_table
            .register_variable(body_scope, variable.clone(), span);

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
        ast_span: Span,
    ) -> Result<Option<usize>> {
        self.convert_expression_for_hover(&collection)?;

        let span = self.ast_span_to_ir_span(ast_span);

        let body_scope = self.symbol_table.create_scope(self.current_scope);
        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        // Переменная цикла существует внутри тела.
        self.symbol_table
            .register_variable(body_scope, variable.clone(), span);

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
        ast_span: Span,
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
        ast_span: Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);
        // Function scope для корректной регистрации переменных (видны во всём теле функции)
        let body_scope = self
            .symbol_table
            .create_scope_with_kind(self.current_scope, ScopeKind::Function);

        // Phase 3: Parameter.type_hint теперь Option<TypeResolution>
        let params_vec: Vec<Parameter> = params
            .iter()
            .map(|p| Parameter {
                name: p.clone(),
                type_hint: None,
                default_value: None,
                is_val: false,
            })
            .collect();

        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        let parent_context_enabled = self
            .form_context_enabled_stack
            .last()
            .copied()
            .unwrap_or(true);
        let context_enabled =
            parent_context_enabled && !directive_disables_form_context(compiler_directive);
        self.form_context_enabled_stack.push(context_enabled);

        // Регистрируем параметры функции в body_scope
        for param in &params_vec {
            self.symbol_table
                .register_variable(body_scope, param.name.clone(), span);
        }

        // Контекстные implicit-переменные формы доступны внутри процедур/функций
        // кроме `*БезКонтекста`.
        if !self.form_context_symbols.is_empty() && context_enabled {
            for name in &self.form_context_symbols {
                if !self.symbol_table.has_variable(body_scope, name) {
                    self.symbol_table
                        .register_variable(body_scope, name.clone(), span);
                }
            }
        }

        // Собираем только прямые дочерние индексы
        let body_result = (|| -> Result<Vec<usize>> {
            let mut body_indices = Vec::new();
            for stmt in body {
                if let Some(idx) = self.convert_statement(stmt)? {
                    body_indices.push(idx);
                }
            }
            Ok(body_indices)
        })();
        self.form_context_enabled_stack.pop();
        let body_indices = body_result?;

        self.current_scope = old_scope;

        // Phase 3: return_type теперь Option<TypeResolution>
        // Context-Aware: передаём директиву компилятора для валидации
        let node = SemanticNode {
            kind: SemanticNodeKind::FunctionDeclaration {
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

    /// Конвертация Procedure declaration
    fn convert_procedure_declaration(
        &mut self,
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        compiler_directive: Option<CompilerDirective>,
        ast_span: Span,
    ) -> Result<Option<usize>> {
        let span = self.ast_span_to_ir_span(ast_span);
        // Function scope для корректной регистрации переменных (видны во всём теле процедуры)
        let body_scope = self
            .symbol_table
            .create_scope_with_kind(self.current_scope, ScopeKind::Function);

        // Phase 3: Parameter.type_hint теперь Option<TypeResolution>
        let params_vec: Vec<Parameter> = params
            .iter()
            .map(|p| Parameter {
                name: p.clone(),
                type_hint: None,
                default_value: None,
                is_val: false,
            })
            .collect();

        let old_scope = self.current_scope;
        self.current_scope = body_scope;

        let parent_context_enabled = self
            .form_context_enabled_stack
            .last()
            .copied()
            .unwrap_or(true);
        let context_enabled =
            parent_context_enabled && !directive_disables_form_context(compiler_directive);
        self.form_context_enabled_stack.push(context_enabled);

        // Регистрируем параметры процедуры в body_scope
        for param in &params_vec {
            self.symbol_table
                .register_variable(body_scope, param.name.clone(), span);
        }

        // Контекстные implicit-переменные формы доступны внутри процедур/функций
        // кроме `*БезКонтекста`.
        if !self.form_context_symbols.is_empty() && context_enabled {
            for name in &self.form_context_symbols {
                if !self.symbol_table.has_variable(body_scope, name) {
                    self.symbol_table
                        .register_variable(body_scope, name.clone(), span);
                }
            }
        }

        // Собираем только прямые дочерние индексы
        let body_result = (|| -> Result<Vec<usize>> {
            let mut body_indices = Vec::new();
            for stmt in body {
                if let Some(idx) = self.convert_statement(stmt)? {
                    body_indices.push(idx);
                }
            }
            Ok(body_indices)
        })();
        self.form_context_enabled_stack.pop();
        let body_indices = body_result?;

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
