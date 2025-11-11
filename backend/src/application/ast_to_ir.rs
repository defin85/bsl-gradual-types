//! Конвертер AST → IR
//!
//! Преобразует синтаксическое представление (AST из tree-sitter)
//! в семантическое представление (IR в shared).
//!
//! # Архитектура
//!
//! ```text
//! AST (backend) → AstToIrConverter → SemanticProgram (shared)
//! ```

use crate::parsing::bsl::ast::{Expression, Program, Statement};
use anyhow::Result;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::ir::*;
use bsl_shared::utils::hash::hash_content;
use std::sync::Arc;

/// Конвертер AST → IR
///
/// Выполняет два прохода:
/// 1. Сбор глобальных символов (функции, процедуры)
/// 2. Конвертация statements → SemanticNode с построением scope hierarchy
pub struct AstToIrConverter {
    /// Таблица символов в процессе построения
    symbol_table: SymbolTable,

    /// Текущий scope
    current_scope: ScopeId,

    /// Семантические узлы
    nodes: Vec<SemanticNode>,

    /// Исходный код (для дополнительной информации в диагностике)
    #[allow(dead_code)]
    source: String,

    /// TypeRepository для доступа к Generic метаданным коллекций
    repository: Arc<dyn TypeRepository>,
}

impl AstToIrConverter {
    /// Создать новый конвертер
    fn new(source: String, repository: Arc<dyn TypeRepository>) -> Self {
        let symbol_table = SymbolTable::new();
        let current_scope = symbol_table.root_scope;

        Self {
            symbol_table,
            current_scope,
            nodes: Vec::new(),
            source,
            repository,
        }
    }

    /// Главный entry point: AST → SemanticProgram
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::application::ast_to_ir::AstToIrConverter;
    /// use bsl_backend::parsing::bsl::ast::Program;
    /// use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// use std::sync::Arc;
    ///
    /// let ast = Program { statements: vec![] };
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let ir = AstToIrConverter::convert(ast, "source code".to_string(), "test.bsl".to_string(), repo)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn convert(
        ast: Program,
        source: String,
        file_path: String,
        repository: Arc<dyn TypeRepository>,
    ) -> Result<SemanticProgram> {
        let mut converter = Self::new(source.clone(), repository);

        // Проход 1: Сбор глобальных функций/процедур
        for statement in &ast.statements {
            converter.collect_global_symbols(statement)?;
        }

        // Проход 2: Конвертация statements → SemanticNode
        // Игнорируем индексы для root level - они нам не нужны
        for statement in ast.statements {
            let _ = converter.convert_statement(statement)?;
        }

        // Построение CFG (опционально, для flow-sensitive)
        let cfg = converter.build_cfg();

        Ok(SemanticProgram {
            symbols: converter.symbol_table,
            nodes: converter.nodes,
            source_info: SourceInfo {
                path: file_path,
                content_hash: hash_content(&source),
            },
            cfg,
        })
    }

    /// Сбор глобальных символов (функции, процедуры)
    fn collect_global_symbols(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::FunctionDecl { name, params, .. } => {
                let params_vec: Vec<Parameter> = params
                    .iter()
                    .map(|p| Parameter {
                        name: p.clone(),
                        type_hint: None,
                        default_value: None,
                        is_val: false,
                    })
                    .collect();

                self.symbol_table.register_function(FunctionSignature {
                    name: name.clone(),
                    params: params_vec,
                    return_type: None,
                    is_export: false,
                });
            }
            Statement::ProcedureDecl { name, params, .. } => {
                let params_vec: Vec<Parameter> = params
                    .iter()
                    .map(|p| Parameter {
                        name: p.clone(),
                        type_hint: None,
                        default_value: None,
                        is_val: false,
                    })
                    .collect();

                self.symbol_table.register_procedure(FunctionSignature {
                    name: name.clone(),
                    params: params_vec,
                    return_type: None,
                    is_export: false,
                });
            }
            _ => {}
        }
        Ok(())
    }

    /// Конвертация Statement → SemanticNode
    ///
    /// Возвращает Option<usize> - индекс добавленного главного узла (или None если узел не добавлен).
    /// Это позволяет собирать только прямые дочерние узлы, исключая вложенные.
    fn convert_statement(&mut self, statement: Statement) -> Result<Option<usize>> {
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
                        type_hint: type_hint.clone(),
                        is_export: false,
                        initial_value_type: None,
                    },
                    span,
                    scope_id: self.current_scope,
                };

                // Регистрируем переменную в текущем scope
                let hint = if let Some(ref t) = type_hint {
                    TypeHint::Explicit(t.clone())
                } else {
                    TypeHint::Unknown
                };
                self.symbol_table
                    .register_variable(self.current_scope, name, hint, span);

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::Assignment {
                target,
                value,
                span: ast_span,
            } => {
                if let Expression::Identifier { name: var_name, .. } = target {
                    // ✅ ИСПРАВЛЕНИЕ Milestone 3.5: Обрабатываем value expression ПЕРЕД Assignment
                    // Это создаст промежуточные узлы (FunctionCall, MemberAccess) для hover
                    let value_node_idx = if let Expression::Call {
                        function,
                        args,
                        span: call_span,
                    } = &value
                    {
                        self.convert_call_expression(*function.clone(), args.clone(), *call_span)?
                    } else {
                        None
                    };

                    let value_type = self.infer_expression_type(&value);
                    let span = self.ast_span_to_ir_span(ast_span);

                    // ✅ КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Определяем тип для переменной
                    let type_hint = if let Expression::New { type_name, .. } = value {
                        // Для Generic коллекций (Массив, Соответствие, Список)
                        match type_name.as_str() {
                            "Массив" => TypeHint::Generic {
                                base_type: "Массив".to_string(),
                                type_params: vec!["?".to_string()],
                                certainty: 0.0,
                            },
                            "Соответствие" => TypeHint::Generic {
                                base_type: "Соответствие".to_string(),
                                type_params: vec!["?".to_string(), "?".to_string()],
                                certainty: 0.0,
                            },
                            "Список" => TypeHint::Generic {
                                base_type: "Список".to_string(),
                                type_params: vec!["?".to_string()],
                                certainty: 0.0,
                            },
                            _ => TypeHint::Inferred(value_type.clone()),
                        }
                    } else {
                        TypeHint::Inferred(value_type.clone())
                    };

                    // ✅ КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Проверяем, существует ли переменная
                    let variable_exists = self
                        .symbol_table
                        .get_variable_type(self.current_scope, &var_name)
                        .is_some();

                    if !variable_exists {
                        // Переменная не объявлена через VarDeclaration → регистрируем её
                        use tracing::debug;
                        debug!(
                            "Assignment declares new variable: {} with type {:?}",
                            var_name, type_hint
                        );
                        self.symbol_table.register_variable(
                            self.current_scope,
                            var_name.clone(),
                            type_hint,
                            span,
                        );
                    } else {
                        // Переменная уже существует → обновляем тип (flow-sensitive)
                        // ✅ Используем публичный API вместо прямого доступа к scopes
                        if !self.symbol_table.update_variable_type(
                            self.current_scope,
                            var_name.clone(),
                            type_hint,
                        ) {
                            tracing::warn!(
                                "Failed to update variable type for '{}' in scope {:?}",
                                var_name,
                                self.current_scope
                            );
                        }
                    }

                    let node = SemanticNode {
                        kind: SemanticNodeKind::Assignment {
                            variable: var_name.clone(),
                            value_type: value_type.clone(),
                            value_node: value_node_idx, // ✅ MILESTONE 3.5: сохраняем индекс узла value
                        },
                        span,
                        scope_id: self.current_scope,
                    };

                    self.nodes.push(node);
                    return Ok(Some(self.nodes.len() - 1));
                }
                Ok(None) // Если target не Identifier
            }

            Statement::If {
                condition,
                then_body,
                else_body,
                span: ast_span,
            } => {
                let condition_type = self.infer_expression_type(&condition);
                let span = self.ast_span_to_ir_span(ast_span);

                // Создаём scope для then ветки
                let then_scope = self.symbol_table.create_scope(self.current_scope);
                let old_scope = self.current_scope;
                self.current_scope = then_scope;

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
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

                    // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
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

            Statement::While {
                condition,
                body,
                span: ast_span,
            } => {
                let condition_type = self.infer_expression_type(&condition);
                let span = self.ast_span_to_ir_span(ast_span);

                let body_scope = self.symbol_table.create_scope(self.current_scope);
                let old_scope = self.current_scope;
                self.current_scope = body_scope;

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
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

            Statement::For {
                variable,
                start,
                end,
                body,
                span: ast_span,
            } => {
                let range_type = format!(
                    "{}..{}",
                    self.infer_expression_type(&start),
                    self.infer_expression_type(&end)
                );
                let span = self.ast_span_to_ir_span(ast_span);

                let body_scope = self.symbol_table.create_scope(self.current_scope);
                let old_scope = self.current_scope;
                self.current_scope = body_scope;

                // Регистрируем переменную цикла
                self.symbol_table.register_variable(
                    self.current_scope,
                    variable.clone(),
                    TypeHint::Explicit("Число".to_string()),
                    span,
                );

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
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

            Statement::ForEach {
                variable,
                collection,
                body,
                span: ast_span,
            } => {
                let collection_type = self.infer_expression_type(&collection);
                let span = self.ast_span_to_ir_span(ast_span);

                let body_scope = self.symbol_table.create_scope(self.current_scope);
                let old_scope = self.current_scope;
                self.current_scope = body_scope;

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
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

            Statement::Return {
                value,
                span: ast_span,
            } => {
                let value_type = value.as_ref().map(|v| self.infer_expression_type(v));
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
                let span = self.ast_span_to_ir_span(ast_span);

                // Try scope
                let try_scope = self.symbol_table.create_scope(self.current_scope);
                let old_scope = self.current_scope;
                self.current_scope = try_scope;

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
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

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
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

            Statement::Call {
                expression,
                span: ast_span,
            } => {
                let span = self.ast_span_to_ir_span(ast_span);

                // Обрабатываем как FunctionCall
                if let Expression::Call { function, args, .. } = expression {
                    return self.convert_call_expression(*function, args, span); // ✅ Возвращаем индекс
                } else if let Expression::PropertyAccess {
                    object, property, ..
                } = expression
                {
                    // ✅ Извлекаем ИМЯ переменной (только для Identifier)
                    let object_name = match object.as_ref() {
                        Expression::Identifier { name, .. } => Some(name.clone()),
                        _ => None, // Для сложных выражений object_name = None
                    };

                    // Инферим ТИП объекта (всегда)
                    let object_type = self.infer_expression_type(&object);

                    let node = SemanticNode {
                        kind: SemanticNodeKind::MemberAccess {
                            object_name, // ✅ Имя переменной
                            object_type, // ✅ Тип переменной
                            member_name: property,
                            is_method: true,
                        },
                        span,
                        scope_id: self.current_scope,
                    };

                    self.nodes.push(node);
                    return Ok(Some(self.nodes.len() - 1));
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
                span: ast_span,
            } => {
                let span = self.ast_span_to_ir_span(ast_span);
                let body_scope = self.symbol_table.create_scope(self.current_scope);

                let params_vec: Vec<Parameter> = params
                    .iter()
                    .map(|p| Parameter {
                        name: p.clone(),
                        type_hint: None,
                        default_value: None,
                        is_val: false,
                    })
                    .collect();

                // ✅ ИСПРАВЛЕНИЕ: Применяем паттерн из IfStatement
                let old_scope = self.current_scope;
                self.current_scope = body_scope;

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
                let mut body_indices = Vec::new();
                for stmt in body {
                    if let Some(idx) = self.convert_statement(stmt)? {
                        body_indices.push(idx);
                    }
                }

                self.current_scope = old_scope;

                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionDeclaration {
                        name,
                        params: params_vec,
                        return_type: None,
                        body_scope,
                        body: body_indices,
                    },
                    span,
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::ProcedureDecl {
                name,
                params,
                body,
                span: ast_span,
            } => {
                let span = self.ast_span_to_ir_span(ast_span);
                let body_scope = self.symbol_table.create_scope(self.current_scope);

                let params_vec: Vec<Parameter> = params
                    .iter()
                    .map(|p| Parameter {
                        name: p.clone(),
                        type_hint: None,
                        default_value: None,
                        is_val: false,
                    })
                    .collect();

                // ✅ ИСПРАВЛЕНИЕ: Применяем паттерн из IfStatement
                let old_scope = self.current_scope;
                self.current_scope = body_scope;

                // ✅ ИСПРАВЛЕНИЕ: Собираем только прямые дочерние индексы
                let mut body_indices = Vec::new();
                for stmt in body {
                    if let Some(idx) = self.convert_statement(stmt)? {
                        body_indices.push(idx);
                    }
                }

                self.current_scope = old_scope;

                let node = SemanticNode {
                    kind: SemanticNodeKind::ProcedureDeclaration {
                        name,
                        params: params_vec,
                        body_scope,
                        body: body_indices,
                    },
                    span,
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            _ => {
                // Другие statement типы пока пропускаем
                // TODO: Добавить Goto, Label, Execute, RaiseError, AddHandler, RemoveHandler, Await
                Ok(None)
            }
        }
    }

    /// Конвертация вызова функции
    fn convert_call_expression(
        &mut self,
        function: Expression,
        args: Vec<Expression>,
        span: Span,
    ) -> Result<Option<usize>> {
        // ✅ Возвращаем индекс созданного узла
        let function_name = match function {
            Expression::Identifier { name, .. } => name,
            Expression::PropertyAccess {
                object, property, ..
            } => {
                // Метод объекта: объект.Метод()
                let object_type = self.infer_expression_type(&object);
                let arg_types: Vec<String> = args
                    .iter()
                    .map(|arg| self.infer_expression_type(arg))
                    .collect();

                // ✅ НОВОЕ: Generic inference из вызова метода
                // Если это вызов метода переменной (а не выражения),
                // пытаемся вывести Generic тип
                let object_name = if let Expression::Identifier { name, .. } = &*object {
                    self.try_infer_generic_from_method_call(name, &property, &arg_types);
                    Some(name.clone())
                } else {
                    None
                };

                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionCall {
                        function_name: property.clone(),
                        object_name, // ✅ НОВОЕ: имя объекта для методов
                        object_type: Some(object_type),
                        arg_types,
                    },
                    span,
                    scope_id: self.current_scope,
                };

                self.nodes.push(node);
                return Ok(Some(self.nodes.len() - 1)); // ✅ Возвращаем индекс
            }
            _ => "Unknown".to_string(),
        };

        let arg_types: Vec<String> = args
            .iter()
            .map(|arg| self.infer_expression_type(arg))
            .collect();

        let node = SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name,
                object_name: None, // ✅ НОВОЕ: обычная функция, не метод
                object_type: None,
                arg_types,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1)) // ✅ Возвращаем индекс
    }

    /// Вывод типа выражения (простая эвристика)
    fn infer_expression_type(&self, expr: &Expression) -> String {
        match expr {
            Expression::Number { .. } => "Число".to_string(),
            Expression::String { .. } => "Строка".to_string(),
            Expression::Boolean { .. } => "Булево".to_string(),
            Expression::Date { .. } => "Дата".to_string(),
            Expression::Identifier { name, .. } => {
                // Поиск переменной в текущем scope
                self.lookup_variable_type(name)
                    .unwrap_or_else(|| name.clone())
            }
            Expression::New { type_name, .. } => type_name.clone(),
            Expression::PropertyAccess {
                object, property, ..
            } => {
                format!("{}.{}", self.infer_expression_type(object), property)
            }
            Expression::Call { function, .. } => {
                // Тип результата вызова функции
                if let Expression::Identifier {
                    name: func_name, ..
                } = function.as_ref()
                {
                    // Проверяем глобальные функции
                    // ✅ Используем публичный API вместо прямого доступа
                    if let Some(sig) = self.symbol_table.find_function(func_name) {
                        return sig
                            .return_type
                            .clone()
                            .unwrap_or_else(|| "Dynamic".to_string());
                    }
                }
                "Dynamic".to_string()
            }
            _ => "Dynamic".to_string(),
        }
    }

    /// Поиск типа переменной в scope hierarchy
    fn lookup_variable_type(&self, name: &str) -> Option<String> {
        // ✅ Используем публичный API вместо прямого доступа к scopes
        self.symbol_table
            .lookup_variable_in_hierarchy(self.current_scope, name)
            .map(|(_, hint)| match hint {
                TypeHint::Explicit(t) | TypeHint::Inferred(t) => t.clone(),
                TypeHint::Generic {
                    base_type,
                    type_params,
                    ..
                } => {
                    // ✅ Generic тип: форматируем как "Массив<Строка>" или "Соответствие<Строка, Число>"
                    if type_params.is_empty() {
                        base_type.clone()
                    } else {
                        format!("{}<{}>", base_type, type_params.join(", "))
                    }
                }
                TypeHint::Unknown => "Dynamic".to_string(),
            })
    }

    /// Построение Control Flow Graph (для flow-sensitive анализа)
    fn build_cfg(&self) -> Option<ControlFlowGraph> {
        // TODO: Реализовать построение CFG в Milestone 2.3
        // Пока возвращаем None
        None
    }

    /// Конвертировать AST Span в IR Span (Milestone 2.11 - ✅ Task A2 + B1: DEBUG логи)
    ///
    /// Передаёт реальные координаты из tree-sitter AST в семантический IR.
    /// Это позволяет `find_node_at_position()` корректно находить узлы по позиции курсора.
    fn ast_span_to_ir_span(&self, ast_span: crate::parsing::bsl::ast::Span) -> Span {
        use tracing::debug;

        let span = Span {
            start_line: ast_span.start_line,
            start_column: ast_span.start_column,
            end_line: ast_span.end_line,
            end_column: ast_span.end_column,
        };

        // Milestone 2.11 Task B1: DEBUG логи для AST → IR конвертации
        debug!(
            "AST → IR Span conversion: {}:{} - {}:{}",
            span.start_line, span.start_column, span.end_line, span.end_column
        );

        span
    }

    /// Попытка вывести Generic тип из вызова метода коллекции
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// МассивСтрок = Новый Массив();      // Generic<Массив, [?]>
    /// МассивСтрок.Добавить("текст");     // → Generic<Массив, ["Строка"]>
    /// ```
    fn try_infer_generic_from_method_call(
        &mut self,
        receiver: &str,
        method_name: &str,
        arguments: &[String],
    ) {
        use tracing::debug;

        // Получаем текущий тип receiver из SymbolTable
        let current_hint = match self
            .symbol_table
            .get_variable_type(self.current_scope, receiver)
        {
            Some(hint) => hint,
            None => {
                debug!(
                    "try_infer_generic: переменная {} не найдена в scope",
                    receiver
                );
                return;
            }
        };

        // Проверяем, что это Generic тип
        let base_type = match &current_hint {
            TypeHint::Generic { base_type, .. } => base_type.clone(),
            _ => {
                debug!(
                    "try_infer_generic: {} не Generic тип, пропускаем inference",
                    receiver
                );
                return;
            }
        };

        // Проверяем, есть ли GenericInfo для базового типа
        let type_data = match self.repository.find_type(&base_type) {
            Some(data) => data,
            None => {
                debug!(
                    "try_infer_generic: тип {} не найден в TypeRepository",
                    base_type
                );
                return;
            }
        };

        let generic_info = match &type_data.generic_info {
            Some(info) => info,
            None => {
                debug!("try_infer_generic: тип {} не имеет GenericInfo", base_type);
                return;
            }
        };

        // Ищем метод в inference_methods
        for inference_method in &generic_info.inference_methods {
            if method_name != inference_method.method_name {
                continue;
            }

            debug!(
                "try_infer_generic: найден inference метод {}.{}",
                base_type, method_name
            );

            // Для каждого параметра, который определяет Generic тип
            for (i, &param_idx) in inference_method.param_indices.iter().enumerate() {
                if let Some(arg_type) = arguments.get(param_idx) {
                    // Получаем индекс Generic параметра (0 для T в Массив<T>, 0 и 1 для K,V в Соответствие<K,V>)
                    let type_param_idx = inference_method
                        .inferred_type_params
                        .get(i)
                        .copied()
                        .unwrap_or(0);

                    // Обновляем Generic параметр
                    let success = self.symbol_table.update_generic_param(
                        self.current_scope,
                        receiver,
                        type_param_idx,
                        arg_type.clone(),
                    );

                    if success {
                        debug!(
                            "✅ Generic inference: {}.{}() → type_param[{}] = {}",
                            receiver, method_name, type_param_idx, arg_type
                        );
                    } else {
                        debug!(
                            "❌ Generic inference failed: не удалось обновить {} type_param[{}]",
                            receiver, type_param_idx
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::bsl::ast::Span as AstSpan;
    use bsl_shared::domain::repository::InMemoryTypeRepository;

    /// Helper функция для тестов - создаёт пустой TypeRepository
    fn create_test_repository() -> Arc<dyn TypeRepository> {
        Arc::new(InMemoryTypeRepository::new())
    }

    #[test]
    fn test_variable_declaration_conversion() {
        let ast = Program {
            statements: vec![Statement::VarDeclaration {
                name: "x".to_string(),
                type_hint: Some("Число".to_string()),
                span: AstSpan::stub(),
            }],
        };

        let ir = AstToIrConverter::convert(
            ast,
            "Перем x: Число;".to_string(),
            "test.bsl".to_string(),
            create_test_repository(),
        )
        .unwrap();

        assert_eq!(ir.nodes.len(), 1);
        if let SemanticNodeKind::VariableDeclaration {
            name, type_hint, ..
        } = &ir.nodes[0].kind
        {
            assert_eq!(name, "x");
            assert_eq!(type_hint, &Some("Число".to_string()));
        } else {
            panic!("Expected VariableDeclaration");
        }
    }

    #[test]
    fn test_if_statement_with_scope() {
        let ast = Program {
            statements: vec![Statement::If {
                condition: Expression::Boolean {
                    value: true,
                    span: AstSpan::stub(),
                },
                then_body: vec![Statement::VarDeclaration {
                    name: "y".to_string(),
                    type_hint: None,
                    span: AstSpan::stub(),
                }],
                else_body: None,
                span: AstSpan::stub(),
            }],
        };

        let ir = AstToIrConverter::convert(
            ast,
            "Если Истина Тогда Перем y; КонецЕсли".to_string(),
            "test.bsl".to_string(),
            create_test_repository(),
        )
        .unwrap();

        // Должно быть 2 узла: IfStatement + VariableDeclaration
        assert_eq!(ir.nodes.len(), 2);

        // Должно быть 2 scope: root + then branch
        assert_eq!(ir.symbols.scopes.len(), 2);
    }

    #[test]
    fn test_function_call_with_args() {
        let ast = Program {
            statements: vec![Statement::Call {
                expression: Expression::Call {
                    function: Box::new(Expression::Identifier {
                        name: "Сообщить".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![Expression::String {
                        value: "Привет".to_string(),
                        span: AstSpan::stub(),
                    }],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            }],
        };

        let ir = AstToIrConverter::convert(
            ast,
            "Сообщить(\"Привет\");".to_string(),
            "test.bsl".to_string(),
            create_test_repository(),
        )
        .unwrap();

        assert_eq!(ir.nodes.len(), 1);
        if let SemanticNodeKind::FunctionCall {
            function_name,
            arg_types,
            ..
        } = &ir.nodes[0].kind
        {
            assert_eq!(function_name, "Сообщить");
            assert_eq!(arg_types.len(), 1);
            assert_eq!(arg_types[0], "Строка");
        } else {
            panic!("Expected FunctionCall");
        }
    }

    #[test]
    fn test_nested_scopes() {
        let ast = Program {
            statements: vec![
                Statement::VarDeclaration {
                    name: "global".to_string(),
                    type_hint: Some("Строка".to_string()),
                    span: AstSpan::stub(),
                },
                Statement::FunctionDecl {
                    name: "TestFunc".to_string(),
                    params: vec![],
                    body: vec![Statement::VarDeclaration {
                        name: "local".to_string(),
                        type_hint: Some("Число".to_string()),
                        span: AstSpan::stub(),
                    }],
                    span: AstSpan::stub(),
                },
            ],
        };

        let ir = AstToIrConverter::convert(
            ast,
            "Перем global: Строка;\nФункция TestFunc()\n  Перем local: Число;\nКонецФункции"
                .to_string(),
            "test.bsl".to_string(),
            create_test_repository(),
        )
        .unwrap();

        // Должно быть 3 scope: root + function body
        assert!(ir.symbols.scopes.len() >= 2);

        // Глобальная переменная должна быть в root scope
        // ✅ Используем публичный API вместо прямого доступа
        assert!(ir.symbols.has_variable(ir.symbols.root_scope, "global"));
    }

    #[test]
    fn test_function_body_indices() {
        let ast = Program {
            statements: vec![Statement::FunctionDecl {
                name: "TestFunc".to_string(),
                params: vec![],
                body: vec![
                    Statement::VarDeclaration {
                        name: "local".to_string(),
                        type_hint: Some("Число".to_string()),
                        span: AstSpan::stub(),
                    },
                    Statement::Assignment {
                        target: Expression::Identifier {
                            name: "local".to_string(),
                            span: AstSpan::stub(),
                        },
                        value: Expression::Number {
                            value: 42.0,
                            span: AstSpan::stub(),
                        },
                        span: AstSpan::stub(),
                    },
                ],
                span: AstSpan::stub(),
            }],
        };

        let ir = AstToIrConverter::convert(
            ast,
            "Функция TestFunc()\n  Перем local: Число;\n  local = 42;\nКонецФункции".to_string(),
            "test.bsl".to_string(),
            create_test_repository(),
        )
        .unwrap();

        // Проверяем, что есть 3 узла: 2 внутренних + FunctionDeclaration
        assert_eq!(ir.nodes.len(), 3);

        // Проверяем, что FunctionDeclaration содержит индексы тела
        if let SemanticNodeKind::FunctionDeclaration { body, .. } = &ir.nodes[2].kind {
            assert_eq!(body.len(), 2); // VariableDeclaration + Assignment
            assert_eq!(body[0], 0); // Индекс первого узла тела
            assert_eq!(body[1], 1); // Индекс второго узла тела
        } else {
            panic!("Expected FunctionDeclaration at nodes[2]");
        }
    }
}
