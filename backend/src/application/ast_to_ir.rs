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
use bsl_shared::domain::is_configuration_type_pattern;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::TypeResolution;
// Note: FacetKind removed in Phase 2 refactoring - using SignatureIndex methods instead
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

    /// SignatureIndex для return type inference (Milestone 3.9)
    signature_index: SignatureIndex,

    /// TypeResolver для резолюции типов с active_facet (DI Milestone 3.17)
    resolver: Option<Arc<TypeResolver>>,
}

impl AstToIrConverter {
    /// Создать новый конвертер
    fn new(
        source: String,
        repository: Arc<dyn TypeRepository>,
        signature_index: SignatureIndex,
        resolver: Option<Arc<TypeResolver>>,
    ) -> Self {
        let symbol_table = SymbolTable::new();
        let current_scope = symbol_table.root_scope;

        Self {
            symbol_table,
            current_scope,
            nodes: Vec::new(),
            source,
            repository,
            signature_index,
            resolver,
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
    /// use bsl_shared::domain::signature_index::SignatureIndex;
    /// use std::sync::Arc;
    ///
    /// let ast = Program { statements: vec![] };
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let sig_idx = SignatureIndex::new();
    /// let ir = AstToIrConverter::convert(ast, "source code".to_string(), "test.bsl".to_string(), repo, sig_idx)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn convert(
        ast: Program,
        source: String,
        file_path: String,
        repository: Arc<dyn TypeRepository>,
        signature_index: SignatureIndex,
    ) -> Result<SemanticProgram> {
        Self::convert_with_resolver(ast, source, file_path, repository, signature_index, None)
    }

    /// Главный entry point с TypeResolver: AST → SemanticProgram
    ///
    /// Использует TypeResolver для резолюции типов с корректным active_facet.
    /// Это необходимо для правильной валидации методов фасетных типов.
    ///
    /// # Milestone 3.17: TypeResolver DI
    /// Метод СоздатьЭлемент() для СправочникМенеджер.Контрагенты теперь
    /// корректно резолвится благодаря active_facet = Manager.
    pub fn convert_with_resolver(
        ast: Program,
        source: String,
        file_path: String,
        repository: Arc<dyn TypeRepository>,
        signature_index: SignatureIndex,
        resolver: Option<Arc<TypeResolver>>,
    ) -> Result<SemanticProgram> {
        let mut converter = Self::new(source.clone(), repository, signature_index, resolver);

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
    ///
    /// # Phase 3: TypeResolution для Parameter.type_hint и FunctionSignature.return_type
    ///
    /// - Parameter.type_hint = None (пока не парсим типы параметров из AST)
    /// - FunctionSignature.return_type = None (будет выведен из return statements)
    fn collect_global_symbols(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::FunctionDecl { name, params, .. } => {
                // Phase 3: Parameter.type_hint теперь Option<TypeResolution>
                let params_vec: Vec<Parameter> = params
                    .iter()
                    .map(|p| Parameter {
                        name: p.clone(),
                        type_hint: None, // Phase 3: TypeResolution, не парсим из AST пока
                        default_value: None,
                        is_val: false,
                    })
                    .collect();

                // Phase 3: FunctionSignature.return_type теперь Option<TypeResolution>
                self.symbol_table.register_function(FunctionSignature {
                    name: name.clone(),
                    params: params_vec,
                    return_type: None, // Phase 3: TypeResolution, будет выведен из return
                    is_export: false,
                });
            }
            Statement::ProcedureDecl { name, params, .. } => {
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

                // Phase 3: FunctionSignature.return_type теперь Option<TypeResolution>
                // Процедуры не возвращают значение, поэтому return_type = None
                self.symbol_table.register_procedure(FunctionSignature {
                    name: name.clone(),
                    params: params_vec,
                    return_type: None, // Phase 3: TypeResolution
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

                // Регистрируем переменную в текущем scope
                let resolution = type_hint_resolution.unwrap_or_else(TypeResolution::unknown);
                self.symbol_table
                    .register_variable(self.current_scope, name, resolution, span);

                self.nodes.push(node);
                Ok(Some(self.nodes.len() - 1))
            }

            Statement::Assignment {
                target,
                value,
                span: ast_span,
            } => {
                if let Expression::Identifier { name: var_name, .. } = target {
                    // ✅ ИСПРАВЛЕНИЕ Milestone 3.5 + 3.16: Обрабатываем value expression ПЕРЕД Assignment
                    // Это создаст промежуточные узлы (FunctionCall, MemberAccess) для hover и валидации
                    let value_node_idx = match &value {
                        Expression::Call {
                            function,
                            args,
                            span: call_span,
                        } => self.convert_call_expression(*function.clone(), args.clone(), *call_span)?,

                        // ✅ MILESTONE 3.16: Обрабатываем PropertyAccess для валидации метаданных
                        // Например: Док = Документы.ЗаказКлиента
                        Expression::PropertyAccess {
                            object,
                            property,
                            span: prop_span,
                        } => {
                            // Извлекаем имя объекта (для "Документы.ЗаказКлиента" это "Документы")
                            let object_name = match object.as_ref() {
                                Expression::Identifier { name, .. } => Some(name.clone()),
                                _ => None,
                            };

                            // Phase 3: Инферим тип объекта с TypeResolution
                            let object_type = self.infer_type_resolution(object);
                            let span = self.ast_span_to_ir_span(*prop_span);

                            // Создаём MemberAccess узел для валидации
                            let node = SemanticNode {
                                kind: SemanticNodeKind::MemberAccess {
                                    object_name,
                                    object_type,
                                    member_name: property.clone(),
                                    is_method: false,
                                },
                                span,
                                scope_id: self.current_scope,
                            };

                            self.nodes.push(node);
                            Some(self.nodes.len() - 1)
                        }

                        _ => None,
                    };

                    let value_type = self.infer_expression_type(&value);
                    let span = self.ast_span_to_ir_span(ast_span);

                    // КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Определяем тип для переменной
                    // Phase 3: Используем ref для избежания partial move
                    let type_resolution = if let Expression::New { ref type_name, .. } = value {
                        // ОЧИСТКА: Убираем скобки если tree-sitter включил их в type_name
                        let clean_type_name = type_name.trim().trim_end_matches("()").trim();

                        // Для Generic коллекций (Массив, Соответствие, Список)
                        match clean_type_name {
                            "Массив" => TypeResolution::generic("Массив", &["?"], 0.0),
                            "Соответствие" => TypeResolution::generic("Соответствие", &["?", "?"], 0.0),
                            "Список" => TypeResolution::generic("Список", &["?"], 0.0),
                            _ => {
                                // ✅ ИСПРАВЛЕНИЕ: ДЛЯ ВСЕХ ОСТАЛЬНЫХ ТИПОВ создаём Explicit!
                                TypeResolution::explicit(clean_type_name)
                            }
                        }
                    } else {
                        TypeResolution::inferred(&value_type, 0.8)
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
                            var_name, type_resolution
                        );
                        self.symbol_table.register_variable(
                            self.current_scope,
                            var_name.clone(),
                            type_resolution,
                            span,
                        );
                    } else {
                        // Переменная уже существует → обновляем тип (flow-sensitive)
                        // ✅ Используем публичный API вместо прямого доступа к scopes
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
                    // Используем infer_type_resolution вместо infer_expression_type
                    let value_type_resolution = self.infer_type_resolution(&value);

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

            Statement::If {
                condition,
                then_body,
                else_body,
                span: ast_span,
            } => {
                // Phase 3: Используем infer_type_resolution для условия
                let condition_type = self.infer_type_resolution(&condition);
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
                // Phase 3: Используем infer_type_resolution для условия
                let condition_type = self.infer_type_resolution(&condition);
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

            Statement::ForEach {
                variable,
                collection,
                body,
                span: ast_span,
            } => {
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

                    // Phase 3: Инферим ТИП объекта с TypeResolution
                    let object_type = self.infer_type_resolution(&object);

                    let node = SemanticNode {
                        kind: SemanticNodeKind::MemberAccess {
                            object_name, // ✅ Имя переменной
                            object_type, // ✅ Phase 3: TypeResolution
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

                // Собираем только прямые дочерние индексы
                let mut body_indices = Vec::new();
                for stmt in body {
                    if let Some(idx) = self.convert_statement(stmt)? {
                        body_indices.push(idx);
                    }
                }

                self.current_scope = old_scope;

                // Phase 3: return_type теперь Option<TypeResolution>
                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionDeclaration {
                        name,
                        params: params_vec,
                        return_type: None, // Phase 3: TypeResolution, будет выведен из return
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

                // Собираем только прямые дочерние индексы
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
        // Возвращаем индекс созданного узла
        let function_name = match function {
            Expression::Identifier { name, .. } => name,
            Expression::PropertyAccess {
                object, property, ..
            } => {
                // Метод объекта: объект.Метод()
                // Phase 3: Используем infer_type_resolution для полной информации
                let object_type = self.infer_type_resolution(&object);

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

                let node = SemanticNode {
                    kind: SemanticNodeKind::FunctionCall {
                        function_name: property.clone(),
                        object_name, // Имя объекта для методов
                        // Phase 3: object_type теперь TypeResolution
                        object_type: Some(object_type),
                        // Phase 3: arg_types теперь Vec<TypeResolution>
                        arg_types,
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
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        Ok(Some(self.nodes.len() - 1)) // Возвращаем индекс
    }

    /// Вывод типа выражения (простая эвристика)
    fn infer_expression_type(&self, expr: &Expression) -> String {
        match expr {
            Expression::Number { .. } => "Число".to_string(),
            Expression::String { .. } => "Строка".to_string(),
            Expression::Boolean { .. } => "Булево".to_string(),
            Expression::Date { .. } => "Дата".to_string(),
            Expression::Identifier { name, .. } => {
                // MILESTONE 3.11 Phase 2: tree-sitter может парсить "Справочники.X" как один Identifier
                // Проверяем и трансформируем в Manager facet
                if let Some(dot_pos) = name.find('.') {
                    let collection = &name[..dot_pos];
                    let object_name = &name[dot_pos + 1..];

                    let result = match collection {
                        "Справочники" | "Catalogs" => format!("СправочникМенеджер.{}", object_name),
                        "Документы" | "Documents" => format!("ДокументМенеджер.{}", object_name),
                        "ПланыВидовХарактеристик" | "ChartsOfCharacteristicTypes" => format!("ПланВидовХарактеристикМенеджер.{}", object_name),
                        "ПланыСчетов" | "ChartsOfAccounts" => format!("ПланСчетовМенеджер.{}", object_name),
                        "ПланыВидовРасчета" | "ChartsOfCalculationTypes" => format!("ПланВидовРасчетаМенеджер.{}", object_name),
                        "БизнесПроцессы" | "BusinessProcesses" => format!("БизнесПроцессМенеджер.{}", object_name),
                        "Задачи" | "Tasks" => format!("ЗадачаМенеджер.{}", object_name),
                        "РегистрыСведений" | "InformationRegisters" => format!("РегистрСведенийМенеджер.{}", object_name),
                        "РегистрыНакопления" | "AccumulationRegisters" => format!("РегистрНакопленияМенеджер.{}", object_name),
                        "РегистрыБухгалтерии" | "AccountingRegisters" => format!("РегистрБухгалтерииМенеджер.{}", object_name),
                        "РегистрыРасчета" | "CalculationRegisters" => format!("РегистрРасчетаМенеджер.{}", object_name),
                        _ => {
                            // Не глобальная коллекция - поиск переменной
                            self.lookup_variable_type(name)
                                .unwrap_or_else(|| name.clone())
                        }
                    };
                    return result;
                }

                // Поиск переменной в текущем scope
                self.lookup_variable_type(name)
                    .unwrap_or_else(|| name.clone())
            }
            Expression::New { type_name, .. } => type_name.clone(),
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let base_type = self.infer_expression_type(object);

                // MILESTONE 3.11 Phase 2: PropertyAccess для глобальных коллекций → Manager facet
                match base_type.as_str() {
                    "Справочники" | "Catalogs" => format!("СправочникМенеджер.{}", property),
                    "Документы" | "Documents" => format!("ДокументМенеджер.{}", property),
                    "ПланыВидовХарактеристик" | "ChartsOfCharacteristicTypes" => format!("ПланВидовХарактеристикМенеджер.{}", property),
                    "ПланыСчетов" | "ChartsOfAccounts" => format!("ПланСчетовМенеджер.{}", property),
                    "ПланыВидовРасчета" | "ChartsOfCalculationTypes" => format!("ПланВидовРасчетаМенеджер.{}", property),
                    "БизнесПроцессы" | "BusinessProcesses" => format!("БизнесПроцессМенеджер.{}", property),
                    "Задачи" | "Tasks" => format!("ЗадачаМенеджер.{}", property),
                    "РегистрыСведений" | "InformationRegisters" => format!("РегистрСведенийМенеджер.{}", property),
                    "РегистрыНакопления" | "AccumulationRegisters" => format!("РегистрНакопленияМенеджер.{}", property),
                    "РегистрыБухгалтерии" | "AccountingRegisters" => format!("РегистрБухгалтерииМенеджер.{}", property),
                    "РегистрыРасчета" | "CalculationRegisters" => format!("РегистрРасчетаМенеджер.{}", property),
                    _ => format!("{}.{}", base_type, property),
                }
            }
            Expression::Call { function, .. } => {
                // Тип результата вызова функции
                match function.as_ref() {
                    // 1. Метод объекта: object.Method()
                    Expression::PropertyAccess { object, property, .. } => {
                        let object_type = self.infer_expression_type(object);

                        // Убираем Generic параметры для поиска: "Массив<Строка>" → "Массив"
                        let clean_type = if let Some(idx) = object_type.find('<') {
                            &object_type[..idx]
                        } else {
                            &object_type
                        };

                        // Поиск метода в SignatureIndex (платформенные методы)
                        // MILESTONE 3.11 Phase 2: Facet switching с подстановкой имени объекта
                        if let Some(method) = self.signature_index.find_method(clean_type, property) {
                            // Получаем return_type метода
                            if let Some(return_type) = &method.return_type {
                                // Извлекаем имя объекта из исходного типа
                                // "СправочникМенеджер.Контрагенты" → "Контрагенты"
                                if let Some(metadata_name) = SignatureIndex::extract_metadata_name(&object_type) {
                                    // Подставляем имя в return_type
                                    // "СправочникОбъект" + "Контрагенты" → "СправочникОбъект.Контрагенты"
                                    return SignatureIndex::substitute_type_name(return_type, metadata_name);
                                }

                                // Fallback: возвращаем return_type как есть
                                return return_type.clone();
                            }

                            return "Неопределено".to_string();
                        }

                        "Dynamic".to_string()
                    },

                    // 2. Глобальная функция: ТипЗнч()
                    Expression::Identifier { name: func_name, .. } => {
                        // SignatureIndex для платформенных функций
                        if let Some(sig) = self.signature_index.find_global_function(func_name) {
                            return sig.return_type.clone().unwrap_or_else(|| "Неопределено".to_string());
                        }

                        // Fallback: пользовательские функции из SymbolTable
                        // Phase 3: return_type теперь Option<TypeResolution>
                        if let Some(sig) = self.symbol_table.find_function(func_name) {
                            return sig
                                .return_type
                                .as_ref()
                                .map(|r| r.type_name())
                                .unwrap_or_else(|| "Dynamic".to_string());
                        }

                        "Dynamic".to_string()
                    },

                    _ => "Dynamic".to_string()
                }
            }
            _ => "Dynamic".to_string(),
        }
    }

    /// Поиск типа переменной в scope hierarchy
    fn lookup_variable_type(&self, name: &str) -> Option<String> {
        // Используем публичный API вместо прямого доступа к scopes
        self.symbol_table
            .lookup_variable_in_hierarchy(self.current_scope, name)
            .map(|(_, resolution)| resolution.type_name())
    }

    /// Phase 3: Вывод типа с полной информацией TypeResolution
    ///
    /// В отличие от `infer_expression_type()`, возвращает TypeResolution
    /// с Certainty и ResolutionSource для точного отслеживания происхождения типа.
    ///
    /// # Milestone 3.17: TypeResolver DI
    /// Использует TypeResolver для резолюции конфигурационных типов с корректным active_facet.
    /// Это критично для валидации методов фасетных типов (СправочникМенеджер.СоздатьЭлемент()).
    fn infer_type_resolution(&self, expr: &Expression) -> TypeResolution {
        match expr {
            // Примитивные литералы — высокая уверенность
            Expression::Number { .. } => TypeResolution::primitive("Число"),
            Expression::String { .. } => TypeResolution::primitive("Строка"),
            Expression::Boolean { .. } => TypeResolution::primitive("Булево"),
            Expression::Date { .. } => TypeResolution::primitive("Дата"),

            // Идентификаторы — поиск в SymbolTable
            Expression::Identifier { name, .. } => {
                // Проверяем на Неопределено/Null (парсятся как идентификаторы)
                let name_lower = name.to_lowercase();
                if name_lower == "неопределено" || name_lower == "undefined" {
                    return TypeResolution::primitive("Неопределено");
                }
                if name_lower == "null" {
                    return TypeResolution::primitive("Null");
                }

                // Сначала ищем в SymbolTable
                if let Some(resolution) = self
                    .symbol_table
                    .get_variable_type(self.current_scope, name)
                {
                    // Milestone 3.17: Если active_facet отсутствует, но это конфигурационный тип,
                    // пробуем обогатить через TypeResolver
                    if resolution.active_facet.is_none() {
                        let type_name = resolution.type_name();
                        if is_configuration_type_pattern(&type_name) {
                            if let Some(ref resolver) = self.resolver {
                                let enriched = resolver.resolve_expression_sync(&type_name);
                                if enriched.active_facet.is_some() {
                                    return enriched;
                                }
                            }
                        }
                    }
                    return resolution.clone();
                }

                // Переменная не найдена — unknown
                TypeResolution::unknown()
            }

            // Новые конструкции (Новый Тип())
            Expression::New { type_name, .. } => {
                // Очищаем скобки если tree-sitter включил их
                let clean_type_name = type_name.trim().trim_end_matches("()").trim();
                TypeResolution::explicit(clean_type_name)
            }

            // Доступ к свойству (object.property) — критично для конфигурационных типов
            // MILESTONE 3.17: Используем TypeResolver для установки active_facet
            Expression::PropertyAccess { object, property, .. } => {
                let base = self.infer_type_resolution(object);
                let type_str = format!("{}.{}", base.type_name(), property);

                // Проверяем, является ли это конфигурационным типом (Справочники.X, Документы.X, etc.)
                if is_configuration_type_pattern(&type_str) {
                    // Используем TypeResolver для корректной резолюции с active_facet
                    if let Some(ref resolver) = self.resolver {
                        return resolver.resolve_expression_sync(&type_str);
                    }
                }

                // Fallback для обычных типов
                TypeResolution::inferred(&type_str, 0.7)
            }

            // Вызов функции/метода
            Expression::Call { .. } => {
                let type_str = self.infer_expression_type(expr);
                TypeResolution::inferred(&type_str, 0.6)
            }

            // Бинарные/унарные операции
            Expression::Binary { .. } | Expression::Unary { .. } => {
                let type_str = self.infer_expression_type(expr);
                TypeResolution::inferred(&type_str, 0.8)
            }

            // Остальные случаи — используем fallback
            _ => {
                let type_str = self.infer_expression_type(expr);
                if type_str.is_empty() || type_str == "Unknown" || type_str == "Dynamic" {
                    TypeResolution::unknown()
                } else {
                    TypeResolution::inferred(&type_str, 0.5)
                }
            }
        }
    }

    // TODO: Интеграция parse_compiler_directive() для context tracking (Milestone 3.11 Phase 2)
    // Функция была удалена по результатам code review.
    // Будет переинтегрирована когда:
    // - Реализуем парсинг директив при создании SemanticProgram
    // - Передадим текущую директиву в RuntimeExecutionContext
    // - Реализуем context warnings в реальных сценариях
    //
    // Исходная реализация парсила директивы вида:
    // - &НаСервере / &AtServer → OnServer
    // - &НаСервереБезКонтекста / &AtServerNoContext → OnServerNoContext
    // - &НаКлиенте / &AtClient → OnClient
    // - &НаКлиентеНаСервереБезКонтекста / &AtClientAtServerNoContext → OnClientOnServerNoContext

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
        let current_resolution = match self
            .symbol_table
            .get_variable_type(self.current_scope, receiver)
        {
            Some(resolution) => resolution,
            None => {
                debug!(
                    "try_infer_generic: переменная {} не найдена в scope",
                    receiver
                );
                return;
            }
        };

        // Проверяем, что это Generic тип
        use bsl_shared::domain::types::ResolutionResult;
        let base_type = match &current_resolution.result {
            ResolutionResult::Generic(gen) => gen.base_type.clone(),
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
    use bsl_shared::domain::signature_index::SignatureIndex;

    /// Helper функция для тестов - создаёт пустой TypeRepository
    fn create_test_repository() -> Arc<dyn TypeRepository> {
        Arc::new(InMemoryTypeRepository::new())
    }

    /// Helper функция для тестов - создаёт пустой SignatureIndex
    fn create_test_signature_index() -> SignatureIndex {
        SignatureIndex::new()
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
            create_test_signature_index(),
        )
        .unwrap();

        assert_eq!(ir.nodes.len(), 1);
        if let SemanticNodeKind::VariableDeclaration {
            name, type_hint, ..
        } = &ir.nodes[0].kind
        {
            assert_eq!(name, "x");
            // Phase 3: type_hint теперь Option<TypeResolution>
            assert!(type_hint.is_some());
            assert_eq!(type_hint.as_ref().unwrap().type_name(), "Число");
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
            create_test_signature_index(),
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
            create_test_signature_index(),
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
            // Phase 3: arg_types теперь Vec<TypeResolution>
            assert_eq!(arg_types[0].type_name(), "Строка");
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
            create_test_signature_index(),
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
            create_test_signature_index(),
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

#[cfg(test)]
mod directive_tests {
    use bsl_shared::domain::code_location::CompilerDirective;

    /// Test helper: Парсит директиву компиляции из текста
    /// TODO: Будет переинтегрирована в основной код при полной реализации context tracking
    fn parse_compiler_directive_test(text: &str) -> CompilerDirective {
        let text_lower = text.to_lowercase();

        // Порядок проверки важен: сначала более длинные директивы
        if text_lower.contains("&наклиентенасерверебезконтекста")
            || text_lower.contains("&atclientatservernocontext") {
            CompilerDirective::OnClientOnServerNoContext
        } else if text_lower.contains("&насерверебезконтекста")
            || text_lower.contains("&atservernocontext") {
            CompilerDirective::OnServerNoContext
        } else if text_lower.contains("&насервере")
            || text_lower.contains("&atserver") {
            CompilerDirective::OnServer
        } else if text_lower.contains("&наклиенте")
            || text_lower.contains("&atclient") {
            CompilerDirective::OnClient
        } else {
            CompilerDirective::Unknown
        }
    }

    #[test]
    fn test_parse_server_directive() {
        let text = "&НаСервере\nПроцедура Тест()\nКонецПроцедуры";
        let directive = parse_compiler_directive_test(text);
        assert_eq!(directive, CompilerDirective::OnServer);
    }

    #[test]
    fn test_parse_client_directive() {
        let text = "&НаКлиенте\nПроцедура Тест()\nКонецПроцедуры";
        let directive = parse_compiler_directive_test(text);
        assert_eq!(directive, CompilerDirective::OnClient);
    }

    #[test]
    fn test_parse_server_no_context_directive() {
        let text = "&НаСервереБезКонтекста\nПроцедура Тест()\nКонецПроцедуры";
        let directive = parse_compiler_directive_test(text);
        assert_eq!(directive, CompilerDirective::OnServerNoContext);
    }

    #[test]
    fn test_parse_universal_directive() {
        let text = "&НаКлиентеНаСервереБезКонтекста\nПроцедура Тест()\nКонецПроцедуры";
        let directive = parse_compiler_directive_test(text);
        assert_eq!(directive, CompilerDirective::OnClientOnServerNoContext);
    }

    #[test]
    fn test_parse_no_directive() {
        let text = "Процедура Тест()\nКонецПроцедуры";
        let directive = parse_compiler_directive_test(text);
        assert_eq!(directive, CompilerDirective::Unknown);
    }

    #[test]
    fn test_parse_english_directive() {
        let text = "&AtServer\nProcedure Test()\nEndProcedure";
        let directive = parse_compiler_directive_test(text);
        assert_eq!(directive, CompilerDirective::OnServer);
    }
}
