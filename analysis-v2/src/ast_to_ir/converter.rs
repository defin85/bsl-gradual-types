//! Конвертер AST -> IR
//!
//! Преобразует синтаксическое представление (AST из tree-sitter)
//! в семантическое представление (IR в shared).
//!
//! # Архитектура
//!
//! ```text
//! AST (bsl-syntax) -> AstToIrConverter -> SemanticProgram (bsl-shared)
//! ```

use anyhow::Result;
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::{CodeLocation, ModuleType};
use bsl_shared::ir::*;
use bsl_shared::utils::hash::hash_content;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bsl_syntax::ast::{Program, Statement};

/// Конвертер AST -> IR
///
/// Выполняет два прохода:
/// 1. Сбор глобальных символов (функции, процедуры)
/// 2. Конвертация statements -> SemanticNode с построением scope hierarchy
pub struct AstToIrConverter {
    /// Таблица символов в процессе построения
    pub(crate) symbol_table: SymbolTable,

    /// Текущий scope
    pub(crate) current_scope: ScopeId,

    /// Семантические узлы
    pub(crate) nodes: Vec<SemanticNode>,

    /// Исходный код (для дополнительной информации в диагностике)
    #[allow(dead_code)]
    pub(crate) source: String,

    /// TypeRepository для доступа к Generic метаданным коллекций
    pub(crate) repository: Arc<dyn TypeRepository>,

    /// SignatureIndex для return type inference (Milestone 3.9)
    pub(crate) signature_index: SignatureIndex,

    /// TypeResolver для резолюции типов с active_facet (DI Milestone 3.17)
    pub(crate) resolver: Option<Arc<TypeResolver>>,

    /// TypeMetadataLookup для получения свойств фасетных типов (Milestone 3.18)
    pub(crate) metadata_lookup: TypeMetadataLookup,
}

impl AstToIrConverter {
    /// Создать новый конвертер
    pub(crate) fn new(
        source: String,
        repository: Arc<dyn TypeRepository>,
        signature_index: SignatureIndex,
        resolver: Option<Arc<TypeResolver>>,
    ) -> Self {
        let symbol_table = SymbolTable::new();
        let current_scope = symbol_table.root_scope;

        let metadata_lookup = TypeMetadataLookup::new(repository.clone());

        Self {
            symbol_table,
            current_scope,
            nodes: Vec::new(),
            source,
            repository,
            signature_index,
            resolver,
            metadata_lookup,
        }
    }

    /// Главный entry point: AST -> SemanticProgram
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_analysis_v2::AstToIrConverter;
    /// use bsl_syntax::ast::Program;
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

    /// Главный entry point с TypeResolver: AST -> SemanticProgram
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

        // Milestone: инжект контекста модуля (FormModule) в SymbolTable
        converter.seed_module_context(&file_path);

        // Проход 1: Сбор глобальных функций/процедур
        for statement in &ast.statements {
            converter.collect_global_symbols(statement)?;
        }

        // Проход 2: Конвертация statements -> SemanticNode
        // Игнорируем индексы для root level - они нам не нужны
        for statement in ast.statements {
            let _ = converter.convert_statement(statement)?;
        }

        // Post-pass: вывод return types функций по IR (Return nodes) + применение к вызовам/присваиваниям.
        // Это нужно, чтобы выражения вида `X = ЛокальнаяФункция()` получали тип даже если
        // функция объявлена ниже по тексту.
        converter.infer_function_return_types_from_ir();
        converter.apply_inferred_function_return_types();

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

    fn infer_function_return_types_from_ir(&mut self) {
        // Собираем апдейты отдельно, чтобы не держать мутабельные и иммутабельные заимствования одновременно.
        let mut updates: Vec<(usize, String, TypeResolution)> = Vec::new();

        for (node_idx, node) in self.nodes.iter().enumerate() {
            let SemanticNodeKind::FunctionDeclaration { name, body, .. } = &node.kind else {
                continue;
            };

            let mut returns = Vec::new();
            self.collect_return_types(body, &mut returns);

            let inferred = self.merge_return_types(returns);
            updates.push((node_idx, name.clone(), inferred));
        }

        for (node_idx, func_name, return_type) in updates {
            if let SemanticNodeKind::FunctionDeclaration {
                return_type: rt_slot,
                ..
            } = &mut self.nodes[node_idx].kind
            {
                *rt_slot = Some(return_type.clone());
            }

            // Обновляем SymbolTable, чтобы TypeInference мог резолвить вызовы функции.
            let _ = self
                .symbol_table
                .set_function_return_type(&func_name, return_type);
        }
    }

    fn apply_inferred_function_return_types(&mut self) {
        // 1) Собираем return types всех пользовательских функций
        let mut return_types: HashMap<String, TypeResolution> = HashMap::new();
        for (name, sig) in self.symbol_table.iter_functions() {
            if let Some(rt) = &sig.return_type {
                return_types.insert(name.clone(), rt.clone());
            }
        }
        if return_types.is_empty() {
            return;
        }

        // 2) Обогащаем FunctionCall(result_type) для глобальных вызовов пользовательских функций
        let mut updated_calls: HashMap<usize, TypeResolution> = HashMap::new();
        for (idx, node) in self.nodes.iter_mut().enumerate() {
            let SemanticNodeKind::FunctionCall {
                function_name,
                object_type: None,
                result_type,
                ..
            } = &mut node.kind
            else {
                continue;
            };

            let Some(rt) = return_types.get(function_name) else {
                continue;
            };

            if result_type.is_unknown() || result_type.is_undeclared_variable().is_some() {
                *result_type = rt.clone();
                updated_calls.insert(idx, rt.clone());
            }
        }
        if updated_calls.is_empty() {
            return;
        }

        // 3) Обновляем Assignment(value_type) и тип переменной по ссылке value_node
        for node in self.nodes.iter_mut() {
            let scope_id = node.scope_id;
            let span = node.span;

            let SemanticNodeKind::Assignment {
                variable,
                value_type,
                value_node: Some(value_node_idx),
            } = &mut node.kind
            else {
                continue;
            };

            let Some(rt) = updated_calls.get(value_node_idx) else {
                continue;
            };

            *value_type = rt.clone();

            // Обновляем переменную в том scope, где она была зарегистрирована.
            if let Some((decl_scope_id, _)) = self
                .symbol_table
                .lookup_variable_in_hierarchy(scope_id, variable)
            {
                let _ = self.symbol_table.update_variable_type(
                    decl_scope_id,
                    variable.clone(),
                    rt.clone(),
                );
            } else {
                // Неожиданная ситуация: переменная не была зарегистрирована на этапе конвертации.
                // Регистрируем в function scope, чтобы hover/type_at_position не теряли тип.
                self.symbol_table.register_variable_in_function_scope(
                    scope_id,
                    variable.clone(),
                    rt.clone(),
                    span,
                );
            }
        }
    }

    fn collect_return_types(&self, nodes: &[usize], out: &mut Vec<TypeResolution>) {
        for &node_idx in nodes {
            let Some(node) = self.nodes.get(node_idx) else {
                continue;
            };

            match &node.kind {
                SemanticNodeKind::Return { value_type } => {
                    out.push(
                        value_type
                            .clone()
                            .unwrap_or_else(|| TypeResolution::explicit("Неопределено")),
                    );
                }
                SemanticNodeKind::IfStatement {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.collect_return_types(then_branch, out);
                    if let Some(else_branch) = else_branch {
                        self.collect_return_types(else_branch, out);
                    }
                }
                SemanticNodeKind::WhileLoop { body, .. } => self.collect_return_types(body, out),
                SemanticNodeKind::ForLoop { body, .. } => self.collect_return_types(body, out),
                SemanticNodeKind::ForEachLoop { body, .. } => self.collect_return_types(body, out),
                SemanticNodeKind::TryExcept {
                    try_body,
                    except_body,
                } => {
                    self.collect_return_types(try_body, out);
                    self.collect_return_types(except_body, out);
                }
                SemanticNodeKind::BlockScope { statements, .. } => {
                    self.collect_return_types(statements, out)
                }
                _ => {}
            }
        }
    }

    fn merge_return_types(&self, return_types: Vec<TypeResolution>) -> TypeResolution {
        use bsl_shared::domain::types::WeightedType;
        use bsl_shared::domain::types::{
            Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult,
            ResolutionSource,
        };

        if return_types.is_empty() {
            return TypeResolution::inferred("Неопределено");
        }

        // Если хоть где-то тип неизвестен — итог тоже неизвестен (консервативно).
        if return_types.iter().any(|t| t.is_unknown()) {
            return TypeResolution::unknown();
        }

        let union_variants: Vec<WeightedType> = return_types
            .iter()
            .map(|t| match &t.result {
                ResolutionResult::Concrete(concrete) => WeightedType::new(concrete.clone()),
                _ => WeightedType::new(ConcreteType::Platform(PlatformType {
                    name: t.type_name(),
                })),
            })
            .collect();

        TypeResolution {
            certainty: Certainty::Inferred,
            result: ResolutionResult::normalize_union(union_variants),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn seed_module_context(&mut self, file_path: &str) {
        let path = Path::new(file_path);
        let Ok(location) = CodeLocation::determine_from_path(path) else {
            return;
        };

        let ModuleType::FormModule {
            form_name,
            owner_type,
        } = location.module_type
        else {
            return;
        };

        let Some((xml_kind, object_name)) = owner_type.split_once('.') else {
            return;
        };

        let Some(kind) = MetadataKind::from_xml_tag(xml_kind) else {
            return;
        };

        let collection = kind.display_name();

        let form_type_name = format!("Формы.{}.{}.{}", collection, object_name, form_name);
        let form_object_type_name = format!("ДанныеФормыОбъект.{}.{}", collection, object_name);
        let elements_type_name =
            format!("ЭлементыФормы.{}.{}.{}", collection, object_name, form_name);

        let span = Span::stub();
        let root = self.symbol_table.root_scope;

        // Базовые implicit символы модуля формы
        self.symbol_table.register_variable(
            root,
            "Объект".to_string(),
            TypeResolution::explicit(&form_object_type_name),
            span,
        );
        self.symbol_table.register_variable(
            root,
            "Элементы".to_string(),
            TypeResolution::explicit(&elements_type_name),
            span,
        );
        self.symbol_table.register_variable(
            root,
            "ЭтаФорма".to_string(),
            TypeResolution::explicit(&form_type_name),
            span,
        );

        // Реквизиты формы (из синтетического типа `Формы.*`)
        if let Some(form_type) = self.repository.find_type(&form_type_name) {
            for prop in form_type.properties {
                if prop.name == "Объект" || prop.name == "Элементы" || prop.prop_type.is_empty()
                {
                    continue;
                }
                self.symbol_table.register_variable(
                    root,
                    prop.name,
                    TypeResolution::explicit(&prop.prop_type),
                    span,
                );
            }
        }
    }

    /// Сбор глобальных символов (функции, процедуры)
    ///
    /// # Phase 3: TypeResolution для Parameter.type_hint и FunctionSignature.return_type
    ///
    /// - Parameter.type_hint = None (пока не парсим типы параметров из AST)
    /// - FunctionSignature.return_type = None (будет выведен из return statements)
    pub(crate) fn collect_global_symbols(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::FunctionDecl {
                name,
                params,
                compiler_directive: _,
                ..
            } => {
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
            Statement::ProcedureDecl {
                name,
                params,
                compiler_directive: _,
                ..
            } => {
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

    /// Построение Control Flow Graph (для flow-sensitive анализа)
    pub(crate) fn build_cfg(&self) -> Option<ControlFlowGraph> {
        // TODO: Реализовать построение CFG в Milestone 2.3
        // Пока возвращаем None
        None
    }

    /// Конвертировать AST Span в IR Span (Milestone 2.11)
    ///
    /// Передаёт реальные координаты из tree-sitter AST в семантический IR.
    /// Это позволяет `find_node_at_position()` корректно находить узлы по позиции курсора.
    pub(crate) fn ast_span_to_ir_span(&self, ast_span: Span) -> Span {
        use tracing::debug;

        // Milestone 2.11 Task B1: DEBUG логи для AST -> IR конвертации
        debug!(
            "AST -> IR Span conversion: {}:{} - {}:{}",
            ast_span.start_line, ast_span.start_column, ast_span.end_line, ast_span.end_column
        );

        ast_span
    }
}
