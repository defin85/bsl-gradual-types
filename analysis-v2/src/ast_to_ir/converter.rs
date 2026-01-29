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
use bsl_shared::domain::{CodeLocation, ModuleType};
use bsl_shared::ir::*;
use bsl_shared::utils::hash::hash_content;
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
    #[allow(dead_code)]
    pub(crate) signature_index: SignatureIndex,

    /// TypeResolver для резолюции типов с active_facet (DI Milestone 3.17)
    #[allow(dead_code)]
    pub(crate) resolver: Option<Arc<TypeResolver>>,

    /// TypeMetadataLookup для получения свойств фасетных типов (Milestone 3.18)
    #[allow(dead_code)]
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

        let span = Span::stub();
        let root = self.symbol_table.root_scope;

        // Базовые implicit символы модуля формы (только биндинги имён, без TypeResolution)
        self.symbol_table
            .register_variable(root, "Объект".to_string(), span);
        self.symbol_table
            .register_variable(root, "Элементы".to_string(), span);
        self.symbol_table
            .register_variable(root, "ЭтаФорма".to_string(), span);

        // Реквизиты формы (из синтетического типа `Формы.*`) — тоже только имена.
        if let Some(form_type) = self.repository.find_type(&form_type_name) {
            for prop in form_type.properties {
                if prop.name == "Объект" || prop.name == "Элементы" || prop.prop_type.is_empty()
                {
                    continue;
                }
                self.symbol_table.register_variable(root, prop.name, span);
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

                self.symbol_table.register_function(FunctionSignature {
                    name: name.clone(),
                    params: params_vec,
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

                self.symbol_table.register_procedure(FunctionSignature {
                    name: name.clone(),
                    params: params_vec,
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
    /// Это позволяет `find_node_at_byte_offset()` корректно находить узлы по позиции курсора.
    pub(crate) fn ast_span_to_ir_span(&self, ast_span: Span) -> Span {
        use tracing::debug;

        // Milestone 2.11 Task B1: DEBUG логи для AST -> IR конвертации
        debug!(
            "AST -> IR Span conversion: {}..{}",
            ast_span.start, ast_span.end
        );

        ast_span
    }
}
