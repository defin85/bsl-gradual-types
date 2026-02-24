//! Parser Coordinator - координация Tree-sitter парсера
//!
//! Milestone 2.8 Task 7: Regex fallback удалён, используется только Tree-sitter

#![allow(clippy::explicit_counter_loop)]

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tracing::{debug, error, warn};
use tree_sitter::{InputEdit, Parser, Point};
use url::Url;

use crate::parsing::bsl::ast::{Expression, Program, Statement};
use crate::parsing::ParseResult;
use crate::system::ast_cache::AstCache;
use crate::system::disk_cache::{DiskCache, DiskCacheKey};
use crate::system::intellisense_index::{
    IndexItem, IndexItemKind, IndexKind, IntellisenseIndexStore, SymbolKind, SymbolScope,
};
use crate::system::runtime_config::{global_runtime_config, RuntimeKey};
use crate::system::tree_cache::{hash_content, TreeCache};
use crate::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::resolver::TypeResolver;

fn ast_cache_key(content: &str) -> [u8; 32] {
    *blake3::hash(content.as_bytes()).as_bytes()
}

fn is_cache_disabled_env() -> bool {
    global_runtime_config()
        .get_bool(RuntimeKey::CacheDisable)
        .unwrap_or(false)
}

/// Текстовое изменение для инкрементального парсинга (из LSP)
#[derive(Debug, Clone)]
pub struct TextEdit {
    /// Начальная позиция изменения (UTF-16 code units, как в LSP)
    pub start_line: u32,
    pub start_utf16_column: u32,
    /// Конечная позиция в старом тексте (UTF-16 code units, как в LSP)
    pub old_end_line: u32,
    pub old_end_utf16_column: u32,
    /// Новый текст (вставленный/замененный)
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseChangedRange {
    pub start_byte: u32,
    pub old_end_byte: u32,
    pub new_end_byte: u32,
}

#[derive(Debug, Clone)]
pub struct ParseSnapshotReport {
    pub parse_result: ParseResult,
    pub line_index: Arc<bsl_line_index::LineIndex>,
    pub changed_ranges: Vec<ParseChangedRange>,
    pub backend_tree_hash: u64,
    pub incremental: bool,
    pub fallback_reason: Option<String>,
}

/// Координатор Tree-sitter парсера (Milestone 2.8: без regex fallback)
pub struct ParserCoordinator {
    tree_sitter: TreeSitterParser,
    tree_cache: TreeCache,
    ast_cache: AstCache,
    disk_cache: Arc<DiskCache>,
    cache_scope: Arc<RwLock<AstCacheScope>>,
    cache_enabled: Arc<AtomicBool>,
    symbol_index: Arc<RwLock<Option<Arc<IntellisenseIndexStore>>>>,
}

#[derive(Debug, Clone, Default)]
struct AstCacheScope {
    project_id: Option<String>,
    config_id: Option<String>,
}

/// TreeSitter парсер с tree-sitter-bsl
pub struct TreeSitterParser {
    parser: Mutex<Parser>,
}

impl ParserCoordinator {
    /// Создаёт координатор парсера.
    ///
    /// Примечание: `TypeRepository` больше не хранится в `ParserCoordinator`.
    /// Типовые зависимости и IR живут в `analysis-v2` (DepsSnapshot + salsa queries).
    pub fn new(_repository: Arc<dyn TypeRepository>) -> Self {
        Self {
            tree_sitter: TreeSitterParser::new(),
            tree_cache: TreeCache::new(),
            ast_cache: AstCache::new_from_env(),
            disk_cache: Arc::new(DiskCache::disabled(1)),
            cache_scope: Arc::new(RwLock::new(AstCacheScope::default())),
            cache_enabled: Arc::new(AtomicBool::new(!is_cache_disabled_env())),
            symbol_index: Arc::new(RwLock::new(None)),
        }
    }

    /// Создаёт координатор парсера (legacy signature).
    ///
    /// Примечание: `TypeRepository`/`TypeResolver` больше не хранятся в `ParserCoordinator`.
    /// Метод оставлен для обратной совместимости с wiring-кодом.
    pub fn new_with_resolver(
        _repository: Arc<dyn TypeRepository>,
        _resolver: Arc<TypeResolver>,
    ) -> Self {
        Self {
            tree_sitter: TreeSitterParser::new(),
            tree_cache: TreeCache::new(),
            ast_cache: AstCache::new_from_env(),
            disk_cache: Arc::new(DiskCache::disabled(1)),
            cache_scope: Arc::new(RwLock::new(AstCacheScope::default())),
            cache_enabled: Arc::new(AtomicBool::new(!is_cache_disabled_env())),
            symbol_index: Arc::new(RwLock::new(None)),
        }
    }

    /// Создаёт координатор парсера (legacy имя).
    ///
    /// # Milestone 2.8 Note
    ///
    /// Название "with_fallback" — исторический артефакт. Regex fallback был удалён
    /// в Milestone 2.8.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_runtime::system::ParserCoordinator;
    ///
    /// // Для простых тестов или парсинга без типов
    /// let parser = ParserCoordinator::with_fallback();
    /// let _parse_result = parser.parse("Перем x;");
    /// ```
    pub fn with_fallback() -> Self {
        Self {
            tree_sitter: TreeSitterParser::new(),
            tree_cache: TreeCache::new(),
            ast_cache: AstCache::new_from_env(),
            disk_cache: Arc::new(DiskCache::disabled(1)),
            cache_scope: Arc::new(RwLock::new(AstCacheScope::default())),
            cache_enabled: Arc::new(AtomicBool::new(!is_cache_disabled_env())),
            symbol_index: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_disk_cache(mut self, disk_cache: Arc<DiskCache>) -> Self {
        self.disk_cache = disk_cache;
        self
    }

    pub fn set_intellisense_index(&self, index: Arc<IntellisenseIndexStore>) {
        if let Ok(mut slot) = self.symbol_index.write() {
            *slot = Some(index);
        }
    }

    pub fn set_cache_enabled(&self, enabled: bool) {
        self.cache_enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            self.ast_cache.clear();
        }
    }

    pub fn cache_enabled(&self) -> bool {
        self.cache_enabled.load(Ordering::Relaxed)
    }

    pub fn clear_ast_cache(&self) {
        self.ast_cache.clear();
    }

    pub fn ast_cache_stats(&self) -> crate::system::ast_cache::AstCacheStats {
        self.ast_cache.stats()
    }

    pub fn set_cache_scope(&self, project_id: Option<String>, config_id: Option<String>) {
        let mut scope = self
            .cache_scope
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scope.project_id = project_id;
        scope.config_id = config_id;
    }

    fn cache_scope_snapshot(&self) -> AstCacheScope {
        self.cache_scope
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Парсинг через Tree-sitter с поддержкой ParseResult (Milestone 2.7 Task 3)
    pub fn parse(&self, content: &str) -> Result<ParseResult, String> {
        self.parse_with_cache_internal(content, None)
    }

    pub fn parse_with_cache_for_file(
        &self,
        content: &str,
        file_path: &str,
    ) -> Result<ParseResult, String> {
        self.parse_with_cache_internal(content, Some(file_path))
    }

    /// Инкрементальный парсинг для LSP textDocument/didChange с ParseResult (Milestone 2.7 Task 3)
    pub fn parse_incremental(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
    ) -> Result<ParseResult, String> {
        self.parse_incremental_with_report(file_path, new_content, edits)
            .map(|report| report.parse_result)
    }

    pub fn parse_incremental_with_report(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
    ) -> Result<ParseSnapshotReport, String> {
        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);
        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        // Попытка получить старое дерево из кеша
        if let Some((old_tree, old_source, old_hash)) = self.tree_cache.get(&file_path) {
            // Проверяем, нужно ли обновление
            if old_hash == new_tree_hash {
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                });
            }

            // Применяем инкрементальное обновление
            debug!("Applying {} edits incrementally", edits.len());

            match self.tree_sitter.parse_incremental(
                &new_content,
                Some(&old_tree),
                edits,
                &old_source,
            ) {
                Ok((new_tree, program, changed_ranges)) => {
                    // Кешируем новое дерево
                    self.tree_cache.update(
                        &file_path,
                        new_tree,
                        new_content.clone(),
                        new_tree_hash,
                    );
                    self.store_ast_cache(
                        new_hash,
                        &program,
                        Some(file_path.as_path()),
                        &new_content,
                    );
                    return Ok(ParseSnapshotReport {
                        parse_result: program,
                        line_index,
                        changed_ranges,
                        backend_tree_hash: new_tree_hash,
                        incremental: true,
                        fallback_reason: None,
                    });
                }
                Err(e) => {
                    warn!(
                        "Incremental parsing failed: {}, falling back to full parse",
                        e
                    );
                    let fallback_reason = if e == "No edits provided for incremental parsing" {
                        Some("no_edits_provided".to_string())
                    } else {
                        Some(format!("incremental_failed:{e}"))
                    };
                    // Fallback: полный парсинг (Milestone 2.8: только Tree-sitter)
                    debug!("Full parse for file (fallback): {:?}", file_path);
                    return match self.tree_sitter.parse_with_tree(&new_content) {
                        Ok((tree, program)) => {
                            self.store_ast_cache(
                                new_hash,
                                &program,
                                Some(file_path.as_path()),
                                &new_content,
                            );
                            self.tree_cache.set(
                                file_path,
                                tree,
                                new_content.clone(),
                                new_tree_hash,
                            );
                            Ok(ParseSnapshotReport {
                                parse_result: program,
                                line_index,
                                changed_ranges: Vec::new(),
                                backend_tree_hash: new_tree_hash,
                                incremental: false,
                                fallback_reason,
                            })
                        }
                        Err(parse_err) => {
                            error!("TreeSitter parsing failed after fallback: {}", parse_err);
                            Err(format!("Tree-sitter parsing failed: {}", parse_err))
                        }
                    };
                }
            }
        }

        // Fallback: полный парсинг (Milestone 2.8: только Tree-sitter)
        debug!("Full parse for file: {:?}", file_path);
        match self.tree_sitter.parse_with_tree(&new_content) {
            Ok((tree, program)) => {
                self.store_ast_cache(new_hash, &program, Some(file_path.as_path()), &new_content);
                self.tree_cache
                    .set(file_path, tree, new_content.clone(), new_tree_hash);
                Ok(ParseSnapshotReport {
                    parse_result: program,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree_hash: new_tree_hash,
                    incremental: false,
                    fallback_reason: Some("no_previous_tree".to_string()),
                })
            }
            Err(e) => {
                error!("TreeSitter parsing failed: {}", e);
                Err(format!("Tree-sitter parsing failed: {}", e))
            }
        }
    }

    fn parse_with_cache_internal(
        &self,
        content: &str,
        file_path: Option<&str>,
    ) -> Result<ParseResult, String> {
        let content_hash = ast_cache_key(content);

        if self.cache_enabled() {
            if let Some(cached) = self.ast_cache.get(content_hash) {
                if let Some(path) = file_path {
                    self.update_symbol_index(Path::new(path), &cached);
                }
                return Ok((*cached).clone());
            }
        }

        if self.cache_enabled() {
            if let Some(path) = file_path {
                if Path::new(path).exists() {
                    if let Ok(Some(cached)) = self.try_load_ast_from_disk(path, content) {
                        let cached_arc = Arc::new(cached.clone());
                        self.ast_cache.put(content_hash, cached_arc);
                        self.update_symbol_index(Path::new(path), &cached);
                        return Ok(cached);
                    }
                }
            }
        }

        match self.tree_sitter.parse(content) {
            Ok(result) => {
                if result.has_errors() {
                    warn!(
                        "TreeSitter parsing completed with {} syntax errors",
                        result.syntax_errors.len()
                    );
                } else {
                    debug!("TreeSitter parsing successful");
                }

                self.store_ast_cache(content_hash, &result, file_path.map(Path::new), content);
                Ok(result)
            }
            Err(tree_sitter_error) => {
                error!("TreeSitter parsing failed: {}", tree_sitter_error);
                Err(format!("Tree-sitter parsing failed: {}", tree_sitter_error))
            }
        }
    }

    fn store_ast_cache(
        &self,
        content_hash: [u8; 32],
        result: &ParseResult,
        file_path: Option<&Path>,
        content: &str,
    ) {
        self.store_ast_memory(content_hash, result);

        if let Some(path) = file_path {
            self.update_symbol_index(path, result);
            if path.exists() {
                let path_str = path.to_string_lossy();
                let _ = self.store_ast_in_disk(&path_str, content, result);
            }
        }
    }

    fn store_ast_memory(&self, content_hash: [u8; 32], result: &ParseResult) {
        if self.cache_enabled() {
            self.ast_cache.put(content_hash, Arc::new(result.clone()));
        }
    }

    fn update_symbol_index(&self, file_path: &Path, result: &ParseResult) {
        let index = self.symbol_index.read().ok().and_then(|slot| slot.clone());
        let Some(index) = index else {
            return;
        };
        let uri = path_to_uri(file_path);
        let items = collect_symbol_items(&result.program, &uri);
        index.replace_symbols_for_uri(&uri, items);
    }

    fn try_load_ast_from_disk(
        &self,
        file_path: &str,
        content: &str,
    ) -> Result<Option<ParseResult>, String> {
        if !self.cache_enabled() {
            return Ok(None);
        }
        let key = self.build_ast_cache_key(file_path, content);
        self.disk_cache
            .try_get::<ParseResult>(&key)
            .map_err(|e| format!("AST disk cache read failed: {}", e))
    }

    fn store_ast_in_disk(
        &self,
        file_path: &str,
        content: &str,
        result: &ParseResult,
    ) -> Result<(), String> {
        if !self.cache_enabled() {
            return Ok(());
        }
        let key = self.build_ast_cache_key(file_path, content);
        self.disk_cache
            .get_or_build_with(&key, || Ok(result.clone()), |_| true)
            .map(|_| ())
            .map_err(|e| format!("AST disk cache write failed: {}", e))
    }

    fn build_ast_cache_key(&self, file_path: &str, content: &str) -> DiskCacheKey {
        let canonical = std::fs::canonicalize(file_path)
            .ok()
            .unwrap_or_else(|| PathBuf::from(file_path));
        let source_identity = canonical.to_string_lossy().to_string();
        let source_fingerprint = blake3::hash(content.as_bytes()).to_hex().to_string();
        let settings_fingerprint = format!("ast_v1|{}", env!("CARGO_PKG_VERSION"));
        let key_hash = blake3::hash(
            format!(
                "{}|{}|{}",
                source_identity, source_fingerprint, settings_fingerprint
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string();

        let scope = self.cache_scope_snapshot();
        let mut key = DiskCacheKey::new(
            "ast",
            key_hash,
            source_identity,
            source_fingerprint,
            settings_fingerprint,
        );
        if let Some(project_id) = scope.project_id {
            key = key.with_project_id(project_id);
        }
        if let Some(config_id) = scope.config_id {
            key = key.with_config_id(config_id);
        }
        key
    }

    /// Загрузка платформенных типов (упрощенная)
    ///
    /// Примечание: В новой архитектуре данные о методах приходят из syntax_helper.
    /// Эта функция создаёт только базовые типы-коллекции и применяет GenericInfo.
    pub async fn load_platform_types(&self, repository: &Arc<dyn TypeRepository>) -> Result<()> {
        use bsl_shared::domain::types::{RawDataSource, RawTypeData};

        debug!("Loading platform types via simple parser coordination");

        // Базовые типы-коллекции (без методов - методы из syntax_helper)
        let platform_types = vec![
            RawTypeData {
                name: "Массив".to_string(),
                english_name: "Array".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Соответствие".to_string(),
                english_name: "Map".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "СписокЗначений".to_string(),
                english_name: "ValueList".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ТабличнаяЧасть".to_string(),
                english_name: "TabularSection".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ];

        debug!("Loaded {} basic platform types", platform_types.len());

        // Загружаем типы в репозиторий
        repository
            .load_types(platform_types)
            .map_err(|e| anyhow::anyhow!("Failed to load platform types: {}", e))?;

        // Применяем GenericInfo для inference
        crate::data::loaders::apply_generic_info_to_repository(repository.as_ref());

        let stats = repository.get_stats();
        debug!(
            "TypeRepository stats after platform types load: {} types total",
            stats.total_types
        );

        Ok(())
    }
}

fn path_to_uri(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).ok();
    if let Some(canonical) = canonical {
        if let Ok(url) = Url::from_file_path(canonical) {
            return url.to_string();
        }
    }
    if let Ok(url) = Url::from_file_path(path) {
        return url.to_string();
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("file://{}", normalized)
}

fn collect_symbol_items(program: &Program, uri: &str) -> Vec<IndexItem> {
    let mut items = Vec::new();
    collect_symbol_items_from_statements(&program.statements, uri, true, &mut items);
    items
}

fn collect_symbol_items_from_statements(
    statements: &[Statement],
    uri: &str,
    is_top_level: bool,
    items: &mut Vec<IndexItem>,
) {
    for statement in statements {
        match statement {
            Statement::VarDeclaration { name, span, .. } => {
                let scope = if is_top_level {
                    SymbolScope::Module
                } else {
                    SymbolScope::Local
                };
                items.push(symbol_item(
                    name,
                    SymbolKind::Variable,
                    scope,
                    Some(*span),
                    uri,
                ));
            }
            Statement::FunctionDecl {
                name,
                params,
                body,
                is_export,
                span,
                ..
            } => {
                let mut item = symbol_item(
                    name,
                    SymbolKind::Function,
                    SymbolScope::Module,
                    Some(*span),
                    uri,
                );
                if *is_export {
                    item.visibility = Some(crate::system::intellisense_index::Visibility::Public);
                }
                items.push(item);
                for param in params {
                    items.push(symbol_item(
                        param,
                        SymbolKind::Parameter,
                        SymbolScope::Local,
                        None,
                        uri,
                    ));
                }
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::ProcedureDecl {
                name,
                params,
                body,
                is_export,
                span,
                ..
            } => {
                let mut item = symbol_item(
                    name,
                    SymbolKind::Procedure,
                    SymbolScope::Module,
                    Some(*span),
                    uri,
                );
                if *is_export {
                    item.visibility = Some(crate::system::intellisense_index::Visibility::Public);
                }
                items.push(item);
                for param in params {
                    items.push(symbol_item(
                        param,
                        SymbolKind::Parameter,
                        SymbolScope::Local,
                        None,
                        uri,
                    ));
                }
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::For {
                variable,
                body,
                span,
                ..
            } => {
                items.push(symbol_item(
                    variable,
                    SymbolKind::Variable,
                    SymbolScope::Local,
                    Some(*span),
                    uri,
                ));
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::ForEach {
                variable,
                body,
                span,
                ..
            } => {
                items.push(symbol_item(
                    variable,
                    SymbolKind::Variable,
                    SymbolScope::Local,
                    Some(*span),
                    uri,
                ));
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::Assignment { target, span, .. } => {
                if let Expression::Identifier { name, .. } = target {
                    let scope = if is_top_level {
                        SymbolScope::Module
                    } else {
                        SymbolScope::Local
                    };
                    items.push(symbol_item(
                        name,
                        SymbolKind::Variable,
                        scope,
                        Some(*span),
                        uri,
                    ));
                }
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => {
                collect_symbol_items_from_statements(then_body, uri, false, items);
                if let Some(else_body) = else_body {
                    collect_symbol_items_from_statements(else_body, uri, false, items);
                }
            }
            Statement::While { body, .. } => {
                collect_symbol_items_from_statements(body, uri, false, items);
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                collect_symbol_items_from_statements(try_body, uri, false, items);
                collect_symbol_items_from_statements(except_body, uri, false, items);
            }
            Statement::Return { .. }
            | Statement::Call { .. }
            | Statement::Break { .. }
            | Statement::Continue { .. }
            | Statement::Goto { .. }
            | Statement::Label { .. }
            | Statement::Execute { .. }
            | Statement::RaiseError { .. }
            | Statement::AddHandler { .. }
            | Statement::RemoveHandler { .. }
            | Statement::Await { .. } => {}
        }
    }
}

fn symbol_item(
    name: &str,
    kind: SymbolKind,
    scope: SymbolScope,
    span: Option<bsl_shared::ir::Span>,
    uri: &str,
) -> IndexItem {
    let mut item = IndexItem::new(name, IndexItemKind::Symbol(kind), IndexKind::Symbol);
    item.uri = Some(uri.to_string());
    item.scope = Some(scope);
    item.span = span;
    item
}

impl TreeSitterParser {
    fn new() -> Self {
        let mut parser = Parser::new();

        // Инициализация BSL грамматики от alkoleft
        if let Err(err) = parser.set_language(&tree_sitter_bsl::LANGUAGE.into()) {
            error!("Failed to load BSL grammar: {:?}", err);
        }

        debug!("Tree-sitter-bsl parser initialized");

        Self {
            parser: Mutex::new(parser),
        }
    }

    fn parse(&self, content: &str) -> Result<ParseResult, String> {
        self.parse_with_tree(content).map(|(_, program)| program)
    }

    fn parse_with_tree(&self, content: &str) -> Result<(tree_sitter::Tree, ParseResult), String> {
        // Парсинг с использованием tree-sitter-bsl
        let mut parser = self
            .parser
            .lock()
            .map_err(|e| format!("Failed to lock parser: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Tree-sitter parsing failed".to_string())?;

        let root_node = tree.root_node();

        debug!(
            "Tree-sitter parsed: {} nodes, {} bytes",
            root_node.descendant_count(),
            content.len()
        );

        // Конвертация tree-sitter AST → ParseResult через TreeSitterAdapter
        let result = TreeSitterAdapter::convert_tree(&tree, content)?;
        Ok((tree, result))
    }

    /// Инкрементальный парсинг с использованием старого дерева (Milestone 2.7 Task 3)
    fn parse_incremental(
        &self,
        new_content: &str,
        old_tree: Option<&tree_sitter::Tree>,
        edits: Vec<TextEdit>,
        old_source: &str,
    ) -> Result<(tree_sitter::Tree, ParseResult, Vec<ParseChangedRange>), String> {
        let mut parser = self
            .parser
            .lock()
            .map_err(|e| format!("Failed to lock parser: {}", e))?;

        // Применяем редактирования к старому дереву
        if let Some(mut tree) = old_tree.cloned() {
            if edits.is_empty() {
                return Err("No edits provided for incremental parsing".to_string());
            }

            let mut current_source = old_source.to_string();
            let mut changed_ranges = Vec::with_capacity(edits.len());
            for edit in edits {
                let (input_edit, start_byte, old_end_byte) =
                    Self::text_edit_to_input_edit(&edit, &current_source)?;
                tree.edit(&input_edit);
                debug!("Applied edit: {:?}", input_edit);
                changed_ranges.push(ParseChangedRange {
                    start_byte: start_byte as u32,
                    old_end_byte: old_end_byte as u32,
                    new_end_byte: input_edit.new_end_byte as u32,
                });

                current_source =
                    apply_edit_to_source(&current_source, start_byte, old_end_byte, &edit.new_text);
            }

            if current_source != new_content {
                return Err("Edits do not match new content".to_string());
            }

            // Парсим с использованием отредактированного дерева
            let new_tree = parser
                .parse(new_content, Some(&tree))
                .ok_or_else(|| "Incremental parsing failed".to_string())?;

            debug!(
                "Incremental parse: {} nodes, {} bytes",
                new_tree.root_node().descendant_count(),
                new_content.len()
            );

            let program = TreeSitterAdapter::convert_tree(&new_tree, new_content)?;
            Ok((new_tree, program, changed_ranges))
        } else {
            // Нет старого дерева — полный парсинг
            let tree = parser
                .parse(new_content, None)
                .ok_or_else(|| "Tree-sitter parsing failed".to_string())?;

            let program = TreeSitterAdapter::convert_tree(&tree, new_content)?;
            Ok((tree, program, Vec::new()))
        }
    }

    /// Конвертировать TextEdit → tree-sitter InputEdit
    fn text_edit_to_input_edit(
        edit: &TextEdit,
        source: &str,
    ) -> Result<(InputEdit, usize, usize), String> {
        use crate::system::positioning::LineIndex;

        let index = LineIndex::new(source);
        let start_byte =
            index.utf16_position_to_byte_offset(source, edit.start_line, edit.start_utf16_column);
        let old_end_byte = index.utf16_position_to_byte_offset(
            source,
            edit.old_end_line,
            edit.old_end_utf16_column,
        );

        let start_position =
            index.utf16_position_to_point(source, edit.start_line, edit.start_utf16_column);
        let old_end_position =
            index.utf16_position_to_point(source, edit.old_end_line, edit.old_end_utf16_column);

        let inserted_bytes = edit.new_text.len();
        let new_end_byte = start_byte + inserted_bytes;
        let new_end_position = apply_text_to_point(start_position, &edit.new_text);

        Ok((
            InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position,
                old_end_position,
                new_end_position,
            },
            start_byte,
            old_end_byte,
        ))
    }
}

fn apply_text_to_point(start: Point, text: &str) -> Point {
    let mut row = start.row;
    let mut column = start.column;
    let mut last_line_start = 0usize;
    for (idx, b) in text.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            row += 1;
            last_line_start = idx + 1;
        }
    }

    if row == start.row {
        column += text.len();
    } else {
        column = text.len().saturating_sub(last_line_start);
    }

    Point::new(row, column)
}

fn apply_edit_to_source(
    source: &str,
    start_byte: usize,
    old_end_byte: usize,
    new_text: &str,
) -> String {
    let start = start_byte.min(source.len());
    let end = old_end_byte.min(source.len());

    let mut result = String::with_capacity(
        source.len().saturating_sub(end.saturating_sub(start)) + new_text.len(),
    );
    result.push_str(&source[..start]);
    result.push_str(new_text);
    if end < source.len() {
        result.push_str(&source[end..]);
    }
    result
}

// === Milestone 2.8 Task 7: RegexParser удалён ===
// Regex fallback legacy был удалён, используется только Tree-sitter

// === COMPARISON WITH COMPLEX PARSING ===

#[cfg(test)]
mod comparison_notes {
    //! Сравнение архитектур парсинга
    //!
    //! Old Complex (UnifiedParserCoordinator):
    //! - Strategy pattern с 3+ парсерами
    //! - TreeSitterStrategy + SyntaxHelperStrategy + RegexFallback
    //! - Parser selection logic
    //! - ~300+ LOC
    //!
    //! Current (ParserCoordinator после Milestone 2.8):
    //! - Только Tree-sitter (regex legacy удалён)
    //! - Инкрементальный парсинг + кэширование AST/Tree
    //! - ~200 LOC
    //!
    //! Результат: Упрощение архитектуры + качество анализа
}

#[cfg(test)]
mod symbol_index_tests {
    use super::*;
    use crate::parsing::bsl::ast::{Expression, Program, Statement};
    use bsl_shared::ir::Span;

    fn has_symbol(items: &[IndexItem], name: &str, kind: SymbolKind, scope: SymbolScope) -> bool {
        items.iter().any(|item| {
            item.name == name
                && item.kind == IndexItemKind::Symbol(kind)
                && item.scope == Some(scope)
        })
    }

    #[test]
    fn collect_symbol_items_from_program() {
        let span = Span::new(1, 2);
        let expr = Expression::Number { value: 1.0, span };
        let program = Program {
            statements: vec![
                Statement::VarDeclaration {
                    name: "x".to_string(),
                    type_hint: None,
                    span,
                },
                Statement::Assignment {
                    target: Expression::Identifier {
                        name: "assigned_top".to_string(),
                        span,
                    },
                    value: expr.clone(),
                    span,
                },
                Statement::FunctionDecl {
                    name: "Func".to_string(),
                    params: vec!["a".to_string(), "b".to_string()],
                    body: vec![
                        Statement::VarDeclaration {
                            name: "y".to_string(),
                            type_hint: None,
                            span,
                        },
                        Statement::Assignment {
                            target: Expression::Identifier {
                                name: "assigned_local".to_string(),
                                span,
                            },
                            value: expr.clone(),
                            span,
                        },
                        Statement::Assignment {
                            target: Expression::PropertyAccess {
                                object: Box::new(Expression::Identifier {
                                    name: "obj".to_string(),
                                    span,
                                }),
                                property: "field".to_string(),
                                span,
                            },
                            value: expr.clone(),
                            span,
                        },
                        Statement::For {
                            variable: "i".to_string(),
                            start: expr.clone(),
                            end: expr.clone(),
                            body: Vec::new(),
                            span,
                        },
                    ],
                    compiler_directive: None,
                    is_export: false,
                    span,
                },
                Statement::ProcedureDecl {
                    name: "Proc".to_string(),
                    params: vec!["p".to_string()],
                    body: vec![Statement::ForEach {
                        variable: "item".to_string(),
                        collection: expr.clone(),
                        body: Vec::new(),
                        span,
                    }],
                    compiler_directive: None,
                    is_export: true,
                    span,
                },
                Statement::If {
                    condition: expr.clone(),
                    then_body: vec![Statement::VarDeclaration {
                        name: "z".to_string(),
                        type_hint: None,
                        span,
                    }],
                    else_body: Some(vec![Statement::VarDeclaration {
                        name: "w".to_string(),
                        type_hint: None,
                        span,
                    }]),
                    span,
                },
            ],
        };

        let items = collect_symbol_items(&program, "file:///test.bsl");

        assert!(has_symbol(
            &items,
            "x",
            SymbolKind::Variable,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "assigned_top",
            SymbolKind::Variable,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "Func",
            SymbolKind::Function,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "a",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "b",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "y",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "assigned_local",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(!has_symbol(
            &items,
            "field",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "i",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "Proc",
            SymbolKind::Procedure,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            &items,
            "p",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "item",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "z",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            &items,
            "w",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
    }

    #[test]
    fn update_symbol_index_on_parse() {
        let parser = ParserCoordinator::with_fallback();
        let index = Arc::new(IntellisenseIndexStore::new("cfg", "platform"));
        parser.set_intellisense_index(index.clone());

        let code = r#"Перем x;
Процедура Test(p)
    Неявная = 1;
    Перем y;
КонецПроцедуры"#;
        let file_path = "test.bsl";

        let result = parser.parse_with_cache_for_file(code, file_path);
        assert!(result.is_ok());

        let uri = path_to_uri(Path::new(file_path));
        let snapshot = index.snapshot();
        let items = snapshot
            .symbol_index
            .get(&uri)
            .expect("symbols missing")
            .as_ref();

        assert!(has_symbol(
            items,
            "x",
            SymbolKind::Variable,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            items,
            "Test",
            SymbolKind::Procedure,
            SymbolScope::Module
        ));
        assert!(has_symbol(
            items,
            "p",
            SymbolKind::Parameter,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            items,
            "Неявная",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
        assert!(has_symbol(
            items,
            "y",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
    }
}

#[cfg(test)]
mod parse_snapshot_tests {
    use super::*;

    #[test]
    fn incremental_snapshot_matches_full_parse_result() {
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-parity.bsl");
        let base = "Процедура Тест()\n    x = 1;\nКонецПроцедуры".to_string();
        let updated = "Процедура Тест()\n    x = 2;\nКонецПроцедуры".to_string();

        let seed = parser
            .parse_incremental_with_report(file_path.clone(), base, Vec::new())
            .expect("seed snapshot");
        assert!(!seed.incremental);

        let report = parser
            .parse_incremental_with_report(
                file_path,
                updated.clone(),
                vec![TextEdit {
                    start_line: 1,
                    start_utf16_column: 0,
                    old_end_line: 1,
                    old_end_utf16_column: 10,
                    new_text: "    x = 2;".to_string(),
                }],
            )
            .expect("incremental report");

        assert!(report.incremental);
        assert!(report.fallback_reason.is_none());
        assert!(!report.changed_ranges.is_empty());

        let full = ParserCoordinator::with_fallback()
            .parse(&updated)
            .expect("full parse");
        let incremental_json =
            serde_json::to_string(&report.parse_result).expect("serialize incremental parse");
        let full_json = serde_json::to_string(&full).expect("serialize full parse");
        assert_eq!(incremental_json, full_json);
    }

    #[test]
    fn incremental_snapshot_reports_fallback_reason_when_edits_missing() {
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-fallback.bsl");

        parser
            .parse_incremental_with_report(
                file_path.clone(),
                "Процедура Тест()\n    x = 1;\nКонецПроцедуры".to_string(),
                Vec::new(),
            )
            .expect("seed snapshot");

        let report = parser
            .parse_incremental_with_report(
                file_path,
                "Процедура Тест()\n    x = 2;\nКонецПроцедуры".to_string(),
                Vec::new(),
            )
            .expect("fallback parse report");
        assert!(!report.incremental);
        assert_eq!(report.fallback_reason.as_deref(), Some("no_edits_provided"));
    }

    #[test]
    fn incremental_snapshot_handles_edit_burst_without_drift() {
        let parser = ParserCoordinator::with_fallback();
        let file_path = PathBuf::from("snapshot-burst.bsl");
        let initial_text = "Процедура Тест()\n    x = 0;\nКонецПроцедуры".to_string();

        parser
            .parse_incremental_with_report(file_path.clone(), initial_text, Vec::new())
            .expect("seed snapshot");

        for step in 1..=32_u8 {
            let next_digit = char::from(b'0' + (step % 10));
            let updated = format!("Процедура Тест()\n    x = {};\nКонецПроцедуры", next_digit);

            let report = parser
                .parse_incremental_with_report(
                    file_path.clone(),
                    updated.clone(),
                    vec![TextEdit {
                        start_line: 1,
                        start_utf16_column: 8,
                        old_end_line: 1,
                        old_end_utf16_column: 9,
                        new_text: next_digit.to_string(),
                    }],
                )
                .expect("incremental burst parse");

            assert!(report.incremental, "step {step} should stay incremental");
            assert!(
                report.fallback_reason.is_none(),
                "step {step} must not fallback"
            );
            assert_eq!(report.changed_ranges.len(), 1);
        }
    }
}
