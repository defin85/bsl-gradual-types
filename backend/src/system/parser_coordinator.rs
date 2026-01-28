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

use crate::parsing::bsl::ast::{Program, Statement};
use crate::parsing::ParseResult;
use crate::system::ast_cache::AstCache;
use crate::system::disk_cache::{DiskCache, DiskCacheKey};
use crate::system::intellisense_index::{
    IndexItem, IndexItemKind, IndexKind, IntellisenseIndexStore, SymbolKind, SymbolScope,
};
use crate::system::tree_cache::{hash_content, TreeCache};
use crate::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::parsing::Parser as ParserTrait; // Milestone 2.8: Parser trait

fn ast_cache_key(content: &str) -> [u8; 32] {
    *blake3::hash(content.as_bytes()).as_bytes()
}

fn is_cache_disabled_env() -> bool {
    matches!(
        std::env::var("BSL_CACHE_DISABLE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
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

/// Координатор Tree-sitter парсера (Milestone 2.8: без regex fallback)
pub struct ParserCoordinator {
    tree_sitter: TreeSitterParser,
    tree_cache: TreeCache,
    ast_cache: AstCache,
    disk_cache: Arc<DiskCache>,
    cache_scope: Arc<RwLock<AstCacheScope>>,
    cache_enabled: Arc<AtomicBool>,
    repository: Arc<dyn TypeRepository>,
    /// TypeResolver для резолюции типов с active_facet (Milestone 3.17)
    resolver: Option<Arc<TypeResolver>>,
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
    /// Создаёт координатор с указанным TypeRepository.
    ///
    /// Используйте этот метод когда у вас есть готовый TypeRepository
    /// с загруженными платформенными типами.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::system::ParserCoordinator;
    /// use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// use std::sync::Arc;
    ///
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// // Загрузите типы в repo...
    /// let parser = ParserCoordinator::new(repo);
    /// ```
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self {
            tree_sitter: TreeSitterParser::new(),
            tree_cache: TreeCache::new(),
            ast_cache: AstCache::new_from_env(),
            disk_cache: Arc::new(DiskCache::disabled(1)),
            cache_scope: Arc::new(RwLock::new(AstCacheScope::default())),
            cache_enabled: Arc::new(AtomicBool::new(!is_cache_disabled_env())),
            repository,
            resolver: None,
            symbol_index: Arc::new(RwLock::new(None)),
        }
    }

    /// Создаёт координатор с TypeRepository и TypeResolver.
    ///
    /// # Milestone 3.17: TypeResolver DI
    /// TypeResolver используется для резолюции типов с корректным active_facet,
    /// что необходимо для валидации методов фасетных типов.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::system::ParserCoordinator;
    /// use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// use bsl_shared::domain::resolver::TypeResolver;
    /// use std::sync::Arc;
    ///
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let resolver = Arc::new(TypeResolver::new(repo.clone()));
    /// let parser = ParserCoordinator::new_with_resolver(repo, resolver);
    /// ```
    pub fn new_with_resolver(
        repository: Arc<dyn TypeRepository>,
        resolver: Arc<TypeResolver>,
    ) -> Self {
        Self {
            tree_sitter: TreeSitterParser::new(),
            tree_cache: TreeCache::new(),
            ast_cache: AstCache::new_from_env(),
            disk_cache: Arc::new(DiskCache::disabled(1)),
            cache_scope: Arc::new(RwLock::new(AstCacheScope::default())),
            cache_enabled: Arc::new(AtomicBool::new(!is_cache_disabled_env())),
            repository,
            resolver: Some(resolver),
            symbol_index: Arc::new(RwLock::new(None)),
        }
    }

    /// Создаёт координатор с ПУСТЫМ TypeRepository (для обратной совместимости).
    ///
    /// **ВНИМАНИЕ:** Этот метод создаёт пустой репозиторий БЕЗ платформенных типов.
    /// Используйте только для тестов или простых сценариев без необходимости
    /// разрешения платформенных типов.
    ///
    /// Для production кода используйте [`ParserCoordinator::new`] с загруженными типами.
    ///
    /// # Milestone 2.8 Note
    ///
    /// Название "with_fallback" — исторический артефакт. Regex fallback был удалён
    /// в Milestone 2.8. Метод просто создаёт ParserCoordinator с пустым
    /// InMemoryTypeRepository.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::system::ParserCoordinator;
    ///
    /// // Для простых тестов или парсинга без типов
    /// let parser = ParserCoordinator::with_fallback();
    /// let ir = parser.parse_to_ir("Перем x: Число;", "test.bsl");
    /// ```
    pub fn with_fallback() -> Self {
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        let empty_repo = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        Self {
            tree_sitter: TreeSitterParser::new(),
            tree_cache: TreeCache::new(),
            ast_cache: AstCache::new_from_env(),
            disk_cache: Arc::new(DiskCache::disabled(1)),
            cache_scope: Arc::new(RwLock::new(AstCacheScope::default())),
            cache_enabled: Arc::new(AtomicBool::new(!is_cache_disabled_env())),
            repository: empty_repo,
            resolver: None,
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
        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);

        // Попытка получить старое дерево из кеша
        if let Some((old_tree, old_source, old_hash)) = self.tree_cache.get(&file_path) {
            // Проверяем, нужно ли обновление
            if old_hash == new_tree_hash {
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(result);
            }

            // Применяем инкрементальное обновление
            debug!("Applying {} edits incrementally", edits.len());

            match self.tree_sitter.parse_incremental(
                &new_content,
                Some(&old_tree),
                edits,
                &old_source,
            ) {
                Ok((new_tree, program)) => {
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
                    return Ok(program);
                }
                Err(e) => {
                    warn!(
                        "Incremental parsing failed: {}, falling back to full parse",
                        e
                    );
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
                Ok(program)
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

    /// Парсинг с конвертацией в IR (Milestone 2.8)
    ///
    /// # Milestone 3.17: TypeResolver DI
    /// Если ParserCoordinator создан с TypeResolver (через `new_with_resolver()`),
    /// он будет использоваться для резолюции типов с корректным active_facet.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_backend::system::ParserCoordinator;
    ///
    /// let parser = ParserCoordinator::with_fallback();
    /// let ir = parser.parse_to_ir("Перем x: Число;", "test.bsl");
    /// match ir {
    ///     Ok(program) => println!("IR получен с {} узлами", program.nodes.len()),
    ///     Err(e) => eprintln!("Ошибка парсинга: {}", e),
    /// }
    /// ```
    pub fn parse_to_ir(
        &self,
        content: &str,
        file_path: &str,
    ) -> Result<bsl_shared::ir::SemanticProgram, String> {
        use bsl_analysis_v2::AstToIrConverter;

        // 1. Парсинг в AST (tree-sitter) с поддержкой ParseResult
        let parse_result = self.parse_with_cache_for_file(content, file_path)?;

        // Логируем синтаксические ошибки, если они есть
        if parse_result.has_errors() {
            warn!("⚠️ Обнаружены синтаксические ошибки при парсинге:");
            for error in &parse_result.syntax_errors {
                warn!(
                    "  - [{}:{}-{}:{}] {}",
                    error.span.start_line,
                    error.span.start_column,
                    error.span.end_line,
                    error.span.end_column,
                    error.message
                );
            }
        }

        // 2. Конвертация AST → IR (извлекаем program из ParseResult)
        // Milestone 3.17: Передаём TypeResolver для резолюции типов с active_facet
        let signature_index = self.repository.get_signature_index_clone(); // Milestone 3.9
        AstToIrConverter::convert_with_resolver(
            parse_result.program,
            content.to_string(),
            file_path.to_string(),
            self.repository.clone(), // ✅ Передаём TypeRepository для Generic inference
            signature_index, // ✅ Milestone 3.9: Передаём SignatureIndex для return type inference
            self.resolver.clone(), // ✅ Milestone 3.17: Передаём TypeResolver для active_facet
        )
        .map_err(|e| format!("AST to IR conversion failed: {}", e))
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
            Statement::Assignment { .. }
            | Statement::Return { .. }
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
    ) -> Result<(tree_sitter::Tree, ParseResult), String> {
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
            for edit in edits {
                let (input_edit, start_byte, old_end_byte) =
                    Self::text_edit_to_input_edit(&edit, &current_source)?;
                tree.edit(&input_edit);
                debug!("Applied edit: {:?}", input_edit);

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
            Ok((new_tree, program))
        } else {
            // Нет старого дерева — полный парсинг
            let tree = parser
                .parse(new_content, None)
                .ok_or_else(|| "Tree-sitter parsing failed".to_string())?;

            let program = TreeSitterAdapter::convert_tree(&tree, new_content)?;
            Ok((tree, program))
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
    //! - Parser trait для инверсии зависимостей
    //! - Инкрементальный парсинг
    //! - ~200 LOC
    //!
    //! Результат: Упрощение архитектуры + качество анализа
}

// === MILESTONE 2.8: Parser trait implementation ===

/// Реализация Parser trait для ParserCoordinator (Milestone 2.8: Task 3.2)
///
/// Это позволяет:
/// - CLI использовать ParserCoordinator через trait без зависимости от backend
/// - Легко тестировать с mock parser
/// - Инверсия зависимостей: shared не зависит от backend
impl ParserTrait for ParserCoordinator {
    fn parse_to_ir(
        &self,
        content: &str,
        file_path: &str,
    ) -> Result<bsl_shared::ir::SemanticProgram> {
        // Используем существующий метод parse_to_ir
        self.parse_to_ir(content, file_path)
            .map_err(|e| anyhow::anyhow!("ParserCoordinator::parse_to_ir failed: {}", e))
    }

    fn parser_name(&self) -> &'static str {
        "TreeSitter" // Milestone 2.8: Regex fallback удалён
    }

    fn supports_incremental(&self) -> bool {
        true // ParserCoordinator поддерживает инкрементальный парсинг
    }
}

#[cfg(test)]
mod parser_trait_tests {
    use super::*;

    #[test]
    fn test_parser_trait_parse_to_ir() {
        let parser = ParserCoordinator::with_fallback();
        let code = "Перем x: Число;";

        let result = ParserTrait::parse_to_ir(&parser, code, "test.bsl");

        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.nodes.len(), 1); // One variable declaration
    }

    #[test]
    fn test_parser_trait_parser_name() {
        let parser = ParserCoordinator::with_fallback();
        assert_eq!(ParserTrait::parser_name(&parser), "TreeSitter"); // Milestone 2.8: regex удалён
    }

    #[test]
    fn test_parser_trait_supports_incremental() {
        let parser = ParserCoordinator::with_fallback();
        assert!(ParserTrait::supports_incremental(&parser));
    }
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
        let span = Span::new(1, 1, 1, 2);
        let expr = Expression::Number { value: 1.0, span };
        let program = Program {
            statements: vec![
                Statement::VarDeclaration {
                    name: "x".to_string(),
                    type_hint: None,
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
            "y",
            SymbolKind::Variable,
            SymbolScope::Local
        ));
    }
}
