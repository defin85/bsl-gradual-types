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
use bsl_shared::domain::{CodeLocation, ModuleType};
use bsl_shared::ir::*;
use bsl_shared::utils::hash::hash_content;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use bsl_syntax::ast::{Program, Statement};

use crate::implicit_bindings::{ImplicitBindingResolver, FORM_CONTEXT_BOUND_SYMBOL_KEYS};

/// Конвертер AST -> IR
///
/// Выполняет два прохода:
/// 1. Сбор глобальных символов (функции, процедуры)
/// 2. Конвертация statements -> SemanticNode с построением scope hierarchy
pub struct AstToIrConverter<'a> {
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

    /// Контекстные implicit-символы модуля формы, которые должны быть доступны в процедурах/функциях
    /// кроме `*БезКонтекста`.
    pub(crate) form_context_symbols: Vec<String>,

    /// Стек доступности контекстных form symbols для вложенных процедур/функций.
    /// `false` наследуется вглубь и запрещает инжект context-bound symbols.
    pub(crate) form_context_enabled_stack: Vec<bool>,
    /// Coarse cooperative cancellation checkpoint used by exact IR builds.
    pub(crate) cancellation_checkpoint: Option<&'a dyn Fn()>,
}

impl<'a> AstToIrConverter<'a> {
    /// Создать новый конвертер
    pub(crate) fn new(
        source: String,
        repository: Arc<dyn TypeRepository>,
        signature_index: SignatureIndex,
        resolver: Option<Arc<TypeResolver>>,
        cancellation_checkpoint: Option<&'a dyn Fn()>,
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
            form_context_symbols: Vec::new(),
            form_context_enabled_stack: vec![true],
            cancellation_checkpoint,
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
        Self::convert_with_resolver_and_checkpoint(
            ast,
            source,
            file_path,
            repository,
            signature_index,
            resolver,
            None,
        )
    }

    pub(crate) fn convert_with_resolver_and_checkpoint(
        ast: Program,
        source: String,
        file_path: String,
        repository: Arc<dyn TypeRepository>,
        signature_index: SignatureIndex,
        resolver: Option<Arc<TypeResolver>>,
        cancellation_checkpoint: Option<&'a dyn Fn()>,
    ) -> Result<SemanticProgram> {
        let mut converter = Self::new(
            source.clone(),
            repository,
            signature_index,
            resolver,
            cancellation_checkpoint,
        );

        // Milestone: инжект контекста модуля (FormModule) в SymbolTable
        converter.seed_module_context(&file_path);

        // Проход 1: Сбор глобальных функций/процедур
        for statement in &ast.statements {
            converter.cancellation_checkpoint();
            converter.collect_global_symbols(statement)?;
        }

        // Проход 2: Конвертация statements -> SemanticNode
        // Игнорируем индексы для root level - они нам не нужны
        for statement in ast.statements {
            converter.cancellation_checkpoint();
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
            semantic_facts: SemanticFacts::default(),
        })
    }

    #[inline(always)]
    pub(crate) fn cancellation_checkpoint(&self) {
        if let Some(checkpoint) = self.cancellation_checkpoint {
            checkpoint();
        }
    }

    fn seed_module_context(&mut self, file_path: &str) {
        let path = Path::new(file_path);
        let Ok(location) = CodeLocation::determine_from_path(path) else {
            return;
        };

        let span = Span::stub();
        let root = self.symbol_table.root_scope;
        self.form_context_symbols.clear();

        let binding_resolver = ImplicitBindingResolver::new();
        let bindings = binding_resolver.bindings_for_module(&location.module_type);
        if matches!(location.module_type, ModuleType::FormModule { .. }) {
            let mut seen = BTreeSet::new();
            let mut form_type_name: Option<String> = None;
            for binding in &bindings {
                let key = binding.name.to_lowercase();
                if seen.insert(key) {
                    self.form_context_symbols.push(binding.name.to_string());
                }
                if form_type_name.is_none() {
                    form_type_name = binding
                        .descriptor
                        .as_ref()
                        .and_then(|descriptor| descriptor.form_type_name());
                }
            }

            let Some(form_type_name) = form_type_name else {
                return;
            };

            // Реквизиты формы (из синтетического типа `Формы.*`) доступны в контексте формы.
            if let Some(form_type) = self.repository.find_type(&form_type_name) {
                for prop in form_type.properties {
                    let key = prop.name.to_lowercase();
                    if FORM_CONTEXT_BOUND_SYMBOL_KEYS.contains(&key.as_str())
                        || prop.prop_type.is_empty()
                    {
                        continue;
                    }
                    if seen.insert(key) {
                        self.form_context_symbols.push(prop.name);
                    }
                }
            }
            return;
        }

        for binding in bindings {
            self.symbol_table
                .register_variable(root, binding.name.to_string(), span);
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
        #[derive(Debug, Clone, Copy)]
        struct LoopFrame {
            header_id: CfgNodeId,
            after_loop_id: CfgNodeId,
        }

        fn normalize_ws(text: &str) -> String {
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        }

        fn slice_span(source: &str, span: Span) -> Option<&str> {
            let start = span.start as usize;
            let end = span.end as usize;
            source.get(start..end)
        }

        fn first_line(text: &str) -> &str {
            text.split(['\n', '\r']).next().unwrap_or(text)
        }

        fn strip_prefixes<'a>(text: &'a str, prefixes: &[&str]) -> &'a str {
            for p in prefixes {
                if let Some(rest) = text.strip_prefix(p) {
                    return rest;
                }
            }
            text
        }

        fn strip_suffixes<'a>(text: &'a str, suffixes: &[&str]) -> &'a str {
            for s in suffixes {
                if let Some(rest) = text.strip_suffix(s) {
                    return rest;
                }
            }
            text
        }

        fn extract_condition_from_header(header_line: &str, kind: &SemanticNodeKind) -> String {
            let header = normalize_ws(header_line);

            match kind {
                SemanticNodeKind::IfStatement { .. } => {
                    let after_if = strip_prefixes(
                        header.trim_start(),
                        &["Если ", "если ", "IF ", "If ", "if "],
                    );
                    let trimmed = strip_suffixes(
                        after_if.trim_end(),
                        &[" Тогда", " тогда", " Then", " then"],
                    );
                    normalize_ws(trimmed)
                }
                SemanticNodeKind::WhileLoop { .. } => {
                    let after_while = strip_prefixes(
                        header.trim_start(),
                        &["Пока ", "пока ", "WHILE ", "While ", "while "],
                    );
                    let trimmed =
                        strip_suffixes(after_while.trim_end(), &[" Цикл", " цикл", " Do", " do"]);
                    normalize_ws(trimmed)
                }
                SemanticNodeKind::ForLoop { .. } | SemanticNodeKind::ForEachLoop { .. } => {
                    let after_for = strip_prefixes(
                        header.trim_start(),
                        &["Для ", "для ", "FOR ", "For ", "for "],
                    );
                    let trimmed =
                        strip_suffixes(after_for.trim_end(), &[" Цикл", " цикл", " Do", " do"]);
                    normalize_ws(trimmed)
                }
                _ => normalize_ws(header_line),
            }
        }

        fn node_snippet(source: &str, node: &SemanticNode) -> String {
            let Some(raw) = slice_span(source, node.span) else {
                return String::new();
            };
            normalize_ws(raw)
        }

        fn add_cfg_node(cfg: &mut ControlFlowGraph, kind: CfgNodeKind) -> CfgNodeId {
            let id = cfg.nodes().len();
            let node_id = cfg.add_node(CfgNode { id, kind });
            debug_assert_eq!(id, node_id);
            node_id
        }

        fn add_cfg_node_from_ir(
            cfg: &mut ControlFlowGraph,
            kind: CfgNodeKind,
            ir_node: &SemanticNode,
            ir_node_index: Option<usize>,
        ) -> CfgNodeId {
            let id = add_cfg_node(cfg, kind);
            cfg.set_node_span(id, Some(ir_node.span));
            cfg.set_node_ir_node_index(id, ir_node_index);
            id
        }

        fn header_span(source: &str, full: Span) -> Span {
            let Some(raw) = slice_span(source, full) else {
                return full;
            };

            let line_end = raw
                .bytes()
                .position(|b| b == b'\n' || b == b'\r')
                .unwrap_or(raw.len());

            let end = (full.start as usize)
                .saturating_add(line_end)
                .min(full.end as usize) as u32;
            // Пустой span не содержит offset; делаем минимум 1 байт.
            let end = end.max(full.start.saturating_add(1)).min(full.end);

            Span::new(full.start, end)
        }

        struct Builder<'a> {
            source: &'a str,
            ir_nodes: &'a [SemanticNode],
            cfg: ControlFlowGraph,
            loop_stack: Vec<LoopFrame>,
        }

        impl<'a> Builder<'a> {
            fn build_block(
                &mut self,
                stmts: &[usize],
                fn_exit: CfgNodeId,
            ) -> Option<(CfgNodeId, Vec<CfgNodeId>)> {
                let mut iter = stmts.iter().copied();
                let first_stmt = iter.next()?;
                let (entry, mut open) = self.build_stmt(first_stmt, fn_exit);

                for stmt in iter {
                    let (next_entry, next_open) = self.build_stmt(stmt, fn_exit);
                    for from in open {
                        self.cfg.add_edge(from, next_entry, EdgeKind::Unconditional);
                    }
                    open = next_open;
                }

                Some((entry, open))
            }

            fn build_stmt(
                &mut self,
                stmt_idx: usize,
                fn_exit: CfgNodeId,
            ) -> (CfgNodeId, Vec<CfgNodeId>) {
                let Some(node) = self.ir_nodes.get(stmt_idx) else {
                    let id = add_cfg_node(
                        &mut self.cfg,
                        CfgNodeKind::BasicBlock {
                            statements: vec![format!("<invalid stmt idx: {}>", stmt_idx)],
                        },
                    );
                    return (id, vec![id]);
                };

                match &node.kind {
                    SemanticNodeKind::Assignment {
                        variable,
                        value_span,
                        ..
                    } => {
                        let value = slice_span(self.source, *value_span)
                            .map(normalize_ws)
                            .unwrap_or_default();

                        let id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::Assignment {
                                variable: variable.clone(),
                                value,
                            },
                            node,
                            Some(stmt_idx),
                        );
                        (id, vec![id])
                    }

                    SemanticNodeKind::IfStatement {
                        then_branch,
                        else_branch,
                        ..
                    } => {
                        let raw = slice_span(self.source, node.span).unwrap_or("");
                        let condition = extract_condition_from_header(first_line(raw), &node.kind);

                        let cond_id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::Conditional { condition },
                            node,
                            Some(stmt_idx),
                        );
                        // Span условного узла должен быть только "шапкой" (первая строка),
                        // иначе mapping "позиция → CFG контекст" начинает зависеть от эвристик.
                        let cond_span = header_span(self.source, node.span);
                        self.cfg.set_node_span(cond_id, Some(cond_span));

                        let merge_id = add_cfg_node(
                            &mut self.cfg,
                            CfgNodeKind::BasicBlock {
                                statements: Vec::new(),
                            },
                        );

                        if let Some((then_entry, then_open)) =
                            self.build_block(then_branch, fn_exit)
                        {
                            self.cfg
                                .add_edge(cond_id, then_entry, EdgeKind::ConditionalTrue);
                            for from in then_open {
                                self.cfg.add_edge(from, merge_id, EdgeKind::Unconditional);
                            }
                        } else {
                            let then_empty_id = add_cfg_node(
                                &mut self.cfg,
                                CfgNodeKind::BasicBlock {
                                    statements: Vec::new(),
                                },
                            );
                            let else_start = else_branch
                                .as_ref()
                                .filter(|b| !b.is_empty())
                                .and_then(|b| {
                                    b.iter()
                                        .filter_map(|idx| {
                                            self.ir_nodes.get(*idx).map(|n| n.span.start)
                                        })
                                        .min()
                                })
                                .unwrap_or(node.span.end);
                            self.cfg.set_node_span(
                                then_empty_id,
                                Some(Span::new(cond_span.end, else_start)),
                            );
                            self.cfg
                                .add_edge(cond_id, then_empty_id, EdgeKind::ConditionalTrue);
                            self.cfg
                                .add_edge(then_empty_id, merge_id, EdgeKind::Unconditional);
                        }

                        if let Some(else_branch) = else_branch {
                            if let Some((else_entry, else_open)) =
                                self.build_block(else_branch, fn_exit)
                            {
                                self.cfg
                                    .add_edge(cond_id, else_entry, EdgeKind::ConditionalFalse);
                                for from in else_open {
                                    self.cfg.add_edge(from, merge_id, EdgeKind::Unconditional);
                                }
                            } else {
                                self.cfg
                                    .add_edge(cond_id, merge_id, EdgeKind::ConditionalFalse);
                            }
                        } else {
                            self.cfg
                                .add_edge(cond_id, merge_id, EdgeKind::ConditionalFalse);
                        }

                        (cond_id, vec![merge_id])
                    }

                    SemanticNodeKind::WhileLoop { body, .. }
                    | SemanticNodeKind::ForLoop { body, .. }
                    | SemanticNodeKind::ForEachLoop { body, .. } => {
                        let raw = slice_span(self.source, node.span).unwrap_or("");
                        let condition = extract_condition_from_header(first_line(raw), &node.kind);

                        let header_id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::LoopHeader { condition },
                            node,
                            Some(stmt_idx),
                        );
                        let header_span = header_span(self.source, node.span);
                        self.cfg.set_node_span(header_id, Some(header_span));

                        let after_loop_id = add_cfg_node(
                            &mut self.cfg,
                            CfgNodeKind::BasicBlock {
                                statements: Vec::new(),
                            },
                        );

                        self.cfg
                            .add_edge(header_id, after_loop_id, EdgeKind::LoopExit);

                        let body_marker_id = add_cfg_node(&mut self.cfg, CfgNodeKind::LoopBody);
                        self.cfg.set_node_span(
                            body_marker_id,
                            Some(Span::new(header_span.end, node.span.end)),
                        );
                        self.cfg
                            .add_edge(header_id, body_marker_id, EdgeKind::ConditionalTrue);

                        self.loop_stack.push(LoopFrame {
                            header_id,
                            after_loop_id,
                        });

                        if let Some((body_entry, body_open)) = self.build_block(body, fn_exit) {
                            self.cfg
                                .add_edge(body_marker_id, body_entry, EdgeKind::Unconditional);
                            for from in body_open {
                                self.cfg.add_edge(from, header_id, EdgeKind::LoopBack);
                            }
                        } else {
                            self.cfg
                                .add_edge(body_marker_id, header_id, EdgeKind::LoopBack);
                        }

                        let _ = self.loop_stack.pop();

                        (header_id, vec![after_loop_id])
                    }

                    SemanticNodeKind::Return { value_node } => {
                        let value = value_node
                            .and_then(|idx| self.ir_nodes.get(idx))
                            .and_then(|n| slice_span(self.source, n.span))
                            .map(normalize_ws)
                            .unwrap_or_else(String::new);

                        let statement = if value.is_empty() {
                            "return".to_string()
                        } else {
                            format!("return {}", value)
                        };

                        let id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::BasicBlock {
                                statements: vec![statement],
                            },
                            node,
                            Some(stmt_idx),
                        );
                        self.cfg.add_edge(id, fn_exit, EdgeKind::Unconditional);
                        (id, Vec::new())
                    }

                    SemanticNodeKind::Break => {
                        let id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::BasicBlock {
                                statements: vec!["break".to_string()],
                            },
                            node,
                            Some(stmt_idx),
                        );

                        if let Some(frame) = self.loop_stack.last().copied() {
                            self.cfg
                                .add_edge(id, frame.after_loop_id, EdgeKind::LoopExit);
                            return (id, Vec::new());
                        }

                        (id, vec![id])
                    }

                    SemanticNodeKind::Continue => {
                        let id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::BasicBlock {
                                statements: vec!["continue".to_string()],
                            },
                            node,
                            Some(stmt_idx),
                        );

                        if let Some(frame) = self.loop_stack.last().copied() {
                            self.cfg.add_edge(id, frame.header_id, EdgeKind::LoopBack);
                            return (id, Vec::new());
                        }

                        (id, vec![id])
                    }

                    SemanticNodeKind::TryExcept {
                        try_body,
                        except_body,
                    } => {
                        let cond_id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::Conditional {
                                condition: "exception".to_string(),
                            },
                            node,
                            Some(stmt_idx),
                        );
                        let merge_id = add_cfg_node(
                            &mut self.cfg,
                            CfgNodeKind::BasicBlock {
                                statements: Vec::new(),
                            },
                        );

                        if let Some((try_entry, try_open)) = self.build_block(try_body, fn_exit) {
                            self.cfg
                                .add_edge(cond_id, try_entry, EdgeKind::ConditionalTrue);
                            for from in try_open {
                                self.cfg.add_edge(from, merge_id, EdgeKind::Unconditional);
                            }
                        } else {
                            self.cfg
                                .add_edge(cond_id, merge_id, EdgeKind::ConditionalTrue);
                        }

                        if let Some((except_entry, except_open)) =
                            self.build_block(except_body, fn_exit)
                        {
                            self.cfg
                                .add_edge(cond_id, except_entry, EdgeKind::ConditionalFalse);
                            for from in except_open {
                                self.cfg.add_edge(from, merge_id, EdgeKind::Unconditional);
                            }
                        } else {
                            self.cfg
                                .add_edge(cond_id, merge_id, EdgeKind::ConditionalFalse);
                        }

                        (cond_id, vec![merge_id])
                    }

                    SemanticNodeKind::FunctionCall {
                        function_name,
                        object_name,
                        object_node,
                        ..
                    } => {
                        let (object, method) = match (object_name, object_node) {
                            (Some(name), _) => (name.clone(), function_name.clone()),
                            (None, Some(idx)) => (
                                self.ir_nodes
                                    .get(*idx)
                                    .map(|n| node_snippet(self.source, n))
                                    .unwrap_or_else(|| "<expr>".to_string()),
                                function_name.clone(),
                            ),
                            (None, None) => {
                                let id = add_cfg_node_from_ir(
                                    &mut self.cfg,
                                    CfgNodeKind::BasicBlock {
                                        statements: vec![format!("{}()", function_name)],
                                    },
                                    node,
                                    Some(stmt_idx),
                                );
                                return (id, vec![id]);
                            }
                        };

                        let id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::MethodCall {
                                object,
                                method,
                                arguments: Vec::new(),
                            },
                            node,
                            Some(stmt_idx),
                        );
                        (id, vec![id])
                    }

                    SemanticNodeKind::MemberAccess {
                        object_node,
                        object_name,
                        member_name,
                        access_kind,
                        ..
                    } => {
                        let object = match (object_name, object_node) {
                            (Some(name), _) => name.clone(),
                            (None, Some(idx)) => self
                                .ir_nodes
                                .get(*idx)
                                .map(|n| node_snippet(self.source, n))
                                .unwrap_or_else(|| "<expr>".to_string()),
                            (None, None) => "<expr>".to_string(),
                        };

                        let kind = if access_kind.is_method() {
                            CfgNodeKind::MethodCall {
                                object,
                                method: member_name.clone(),
                                arguments: Vec::new(),
                            }
                        } else {
                            CfgNodeKind::PropertyAccess {
                                object,
                                property: member_name.clone(),
                            }
                        };

                        let id = add_cfg_node_from_ir(&mut self.cfg, kind, node, Some(stmt_idx));
                        (id, vec![id])
                    }

                    _ => {
                        let id = add_cfg_node_from_ir(
                            &mut self.cfg,
                            CfgNodeKind::BasicBlock {
                                statements: vec![node_snippet(self.source, node)],
                            },
                            node,
                            Some(stmt_idx),
                        );
                        (id, vec![id])
                    }
                }
            }

            fn build_component(&mut self, stmts: &[usize]) {
                let entry_id = add_cfg_node(&mut self.cfg, CfgNodeKind::Entry);
                let exit_id = add_cfg_node(&mut self.cfg, CfgNodeKind::Exit);

                if let Some((body_entry, body_open)) = self.build_block(stmts, exit_id) {
                    self.cfg
                        .add_edge(entry_id, body_entry, EdgeKind::Unconditional);
                    for from in body_open {
                        self.cfg.add_edge(from, exit_id, EdgeKind::Unconditional);
                    }
                } else {
                    self.cfg
                        .add_edge(entry_id, exit_id, EdgeKind::Unconditional);
                }
            }
        }

        let mut builder = Builder {
            source: self.source.as_str(),
            ir_nodes: &self.nodes,
            cfg: ControlFlowGraph::new(),
            loop_stack: Vec::new(),
        };

        let mut has_executable = false;

        // 1) Псевдо-компонент для root-scope statements (если они есть)
        let root_scope = self.symbol_table.root_scope;
        let mut root_stmts: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, n)| {
                if n.scope_id != root_scope {
                    return None;
                }
                match n.kind {
                    SemanticNodeKind::FunctionDeclaration { .. }
                    | SemanticNodeKind::ProcedureDeclaration { .. } => None,
                    SemanticNodeKind::BinaryExpression { .. }
                    | SemanticNodeKind::UnaryExpression { .. }
                    | SemanticNodeKind::TernaryExpression { .. }
                    | SemanticNodeKind::AwaitExpression { .. }
                    | SemanticNodeKind::VariableAccess { .. }
                    | SemanticNodeKind::GlobalPropertyAccess { .. } => None,
                    _ => Some(idx),
                }
            })
            .collect();

        root_stmts.sort_by_key(|idx| self.nodes[*idx].span.start);

        if !root_stmts.is_empty() {
            has_executable = true;
            builder.build_component(&root_stmts);
        }

        // 2) Компоненты для тел функций/процедур
        for node in &self.nodes {
            match &node.kind {
                SemanticNodeKind::FunctionDeclaration { body, .. }
                | SemanticNodeKind::ProcedureDeclaration { body, .. } => {
                    if body.is_empty() {
                        continue;
                    }
                    has_executable = true;
                    builder.build_component(body);
                }
                _ => {}
            }
        }

        if !has_executable {
            let mut cfg = ControlFlowGraph::new();
            let entry_id = cfg.add_node(CfgNode {
                id: 0,
                kind: CfgNodeKind::Entry,
            });
            let exit_id = cfg.add_node(CfgNode {
                id: 1,
                kind: CfgNodeKind::Exit,
            });
            cfg.add_edge(entry_id, exit_id, EdgeKind::Unconditional);
            return Some(cfg);
        }

        Some(builder.cfg)
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
