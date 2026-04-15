//! Parser Coordinator - координация Tree-sitter парсера
//!
//! Milestone 2.8 Task 7: Regex fallback удалён, используется только Tree-sitter

#![allow(clippy::explicit_counter_loop)]

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;
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

#[path = "parser_coordinator/symbol_indexing.rs"]
mod symbol_indexing;

use self::symbol_indexing::{collect_symbol_items, path_to_uri};

fn ast_cache_key(content: &str) -> [u8; 32] {
    *blake3::hash(content.as_bytes()).as_bytes()
}

fn is_cache_disabled_env() -> bool {
    global_runtime_config()
        .get_bool(RuntimeKey::CacheDisable)
        .unwrap_or(false)
}

const PARSE_COORDINATOR_CANCELLED_ERROR: &str = "Tree-sitter parsing cancelled";
const PARSE_SNAPSHOT_FALLBACK_NO_PREVIOUS_TREE: &str = "no_previous_tree";
const PARSE_SNAPSHOT_FALLBACK_NO_EDITS_PROVIDED: &str = "no_edits_provided";
const PARSE_SNAPSHOT_FALLBACK_EDITS_DO_NOT_MATCH_NEW_CONTENT: &str =
    "edits_do_not_match_new_content";
const PARSE_SNAPSHOT_FALLBACK_INPUT_EDIT_CONVERSION_FAILED: &str = "input_edit_conversion_failed";
const PARSE_SNAPSHOT_FALLBACK_INCREMENTAL_PARSE_FAILED: &str = "incremental_parse_failed";
const PARSE_SNAPSHOT_FALLBACK_STALE_PARSER_BASE: &str = "stale_parser_base";
const PARSE_SNAPSHOT_FALLBACK_OTHER: &str = "other";

#[cfg(test)]
static PARSE_SNAPSHOT_FULL_PARSE_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
fn maybe_inject_parse_snapshot_full_parse_delay_for_test() {
    if let Some(delay_ms) = std::env::var("BSL_TEST_PARSE_SNAPSHOT_FULL_PARSE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

#[cfg(not(test))]
fn maybe_inject_parse_snapshot_full_parse_delay_for_test() {}

#[cfg(test)]
fn record_parse_snapshot_full_parse_attempt_for_test() {
    PARSE_SNAPSHOT_FULL_PARSE_ATTEMPTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(not(test))]
fn record_parse_snapshot_full_parse_attempt_for_test() {}

fn maybe_inject_current_context_parse_progress_delay_for_test() {
    if let Some(delay_ms) = std::env::var("BSL_TEST_CURRENT_CONTEXT_PARSE_PROGRESS_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

fn maybe_inject_parse_snapshot_parse_progress_delay_for_test() {
    if let Some(delay_ms) = std::env::var("BSL_TEST_PARSE_SNAPSHOT_PARSE_PROGRESS_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
    {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

fn maybe_inject_parse_snapshot_program_conversion_progress_delay_for_test() {
    if let Some(delay_ms) =
        std::env::var("BSL_TEST_PARSE_SNAPSHOT_PROGRAM_CONVERSION_PROGRESS_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
    {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

fn maybe_inject_parse_snapshot_optional_cache_enrichment_delay_for_test() -> Option<u64> {
    std::env::var("BSL_TEST_PARSE_SNAPSHOT_OPTIONAL_CACHE_ENRICHMENT_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn maybe_inject_parse_snapshot_tree_cache_install_delay_for_test() -> Option<u64> {
    std::env::var("BSL_TEST_PARSE_SNAPSHOT_TREE_CACHE_INSTALL_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn maybe_inject_parse_snapshot_syntax_error_assembly_delay_for_test() -> Option<u64> {
    std::env::var("BSL_TEST_PARSE_SNAPSHOT_SYNTAX_ERROR_ASSEMBLY_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn maybe_inject_parse_snapshot_publishable_artifact_packaging_delay_for_test() -> Option<u64> {
    std::env::var("BSL_TEST_PARSE_SNAPSHOT_PUBLISHABLE_ARTIFACT_PACKAGING_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn duration_to_u64_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn canonical_parse_snapshot_fallback_reason(error: &str) -> &'static str {
    if error == "No edits provided for incremental parsing" {
        return PARSE_SNAPSHOT_FALLBACK_NO_EDITS_PROVIDED;
    }
    if error == "Edits do not match new content" {
        return PARSE_SNAPSHOT_FALLBACK_EDITS_DO_NOT_MATCH_NEW_CONTENT;
    }
    if error == "Incremental parsing failed" {
        return PARSE_SNAPSHOT_FALLBACK_INCREMENTAL_PARSE_FAILED;
    }
    if error.starts_with("Input edit conversion failed:") {
        return PARSE_SNAPSHOT_FALLBACK_INPUT_EDIT_CONVERSION_FAILED;
    }
    PARSE_SNAPSHOT_FALLBACK_OTHER
}

#[cfg(test)]
fn maybe_force_incremental_parse_failure_for_test() -> bool {
    std::env::var("BSL_TEST_FORCE_INCREMENTAL_PARSE_FAILURE")
        .ok()
        .as_deref()
        == Some("1")
}

#[cfg(not(test))]
fn maybe_force_incremental_parse_failure_for_test() -> bool {
    false
}

#[cfg(test)]
fn maybe_force_incremental_adapter_error_for_test() -> Option<String> {
    const KEY: &str = "BSL_TEST_FORCE_INCREMENTAL_ADAPTER_ERROR";
    if std::env::var(KEY).ok().as_deref() == Some("1") {
        std::env::remove_var(KEY);
        return Some("Forced incremental adapter error for test".to_string());
    }
    None
}

#[cfg(not(test))]
fn maybe_force_incremental_adapter_error_for_test() -> Option<String> {
    None
}

#[cfg(test)]
pub(crate) fn reset_parse_snapshot_full_parse_attempts_for_test() {
    PARSE_SNAPSHOT_FULL_PARSE_ATTEMPTS.store(0, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
pub(crate) fn get_parse_snapshot_full_parse_attempts_for_test() -> usize {
    PARSE_SNAPSHOT_FULL_PARSE_ATTEMPTS.load(std::sync::atomic::Ordering::SeqCst)
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
    pub backend_tree: Arc<tree_sitter::Tree>,
    pub backend_tree_hash: u64,
    pub incremental: bool,
    pub fallback_reason: Option<String>,
    pub parse_exec_subphases: ParseSnapshotExecSubphaseAttribution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSnapshotExecSubphase {
    CoreParseBuild,
    OptionalCacheEnrichment,
}

impl ParseSnapshotExecSubphase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CoreParseBuild => "core_parse_build",
            Self::OptionalCacheEnrichment => "optional_cache_enrichment",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSnapshotCoreBuildCheckpoint {
    ParserTreeBuild,
    ExactReadySnapshotAssembly,
    TreeCacheInstall,
}

impl ParseSnapshotCoreBuildCheckpoint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParserTreeBuild => "parser_tree_build",
            Self::ExactReadySnapshotAssembly => "exact_ready_snapshot_assembly",
            Self::TreeCacheInstall => "tree_cache_install",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSnapshotAssemblyCheckpoint {
    ProgramLowering,
    PublishableArtifactPackaging,
    SyntaxErrorCollection,
}

impl ParseSnapshotAssemblyCheckpoint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProgramLowering => "program_lowering",
            Self::PublishableArtifactPackaging => "publishable_artifact_packaging",
            Self::SyntaxErrorCollection => "syntax_error_collection",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParseSnapshotExecSubphaseAttribution {
    pub core_parse_build_ms: Option<u64>,
    pub optional_cache_enrichment_ms: Option<u64>,
    pub deferred_optional_cache_enrichment: bool,
    pub deferred_tree_cache_install: bool,
    pub deferred_syntax_error_assembly: bool,
}

impl ParseSnapshotExecSubphaseAttribution {
    pub fn dominant_subphase(&self) -> Option<(&'static str, u64)> {
        [
            ("core_parse_build", self.core_parse_build_ms),
            (
                "optional_cache_enrichment",
                self.optional_cache_enrichment_ms,
            ),
        ]
        .into_iter()
        .filter_map(|(subphase, duration_ms)| duration_ms.map(|value| (subphase, value)))
        .max_by_key(|(_, duration_ms)| *duration_ms)
    }
}

#[derive(Clone, Copy, Default)]
pub struct ParseSnapshotExecutionOptions<'a> {
    pub save_critical_initial: bool,
    pub save_critical_requested: Option<&'a AtomicBool>,
    pub progress_callback: Option<&'a (dyn Fn(ParseSnapshotExecSubphase) + Send + Sync)>,
    pub core_build_progress_callback:
        Option<&'a (dyn Fn(ParseSnapshotCoreBuildCheckpoint) + Send + Sync)>,
    pub assembly_progress_callback:
        Option<&'a (dyn Fn(ParseSnapshotAssemblyCheckpoint) + Send + Sync)>,
}

enum ParseSnapshotTreeCacheInstallOp {
    Set {
        file_path: PathBuf,
        tree: tree_sitter::Tree,
        source: String,
        content_hash: u64,
    },
    Update {
        file_path: PathBuf,
        tree: tree_sitter::Tree,
        source: String,
        content_hash: u64,
    },
}

#[derive(Debug, Clone)]
pub struct CurrentContextParseReport {
    pub parse_result: ParseResult,
    pub line_index: Arc<bsl_line_index::LineIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ParseSnapshotSingleflightKey {
    file_path: PathBuf,
    content_hash: [u8; 32],
}

#[derive(Debug)]
struct ParseSnapshotSingleflightEntry {
    result: Mutex<Option<Result<ParseSnapshotReport, String>>>,
    ready: Condvar,
}

/// Координатор Tree-sitter парсера (Milestone 2.8: без regex fallback)
pub struct ParserCoordinator {
    tree_sitter: TreeSitterParser,
    tree_cache: TreeCache,
    parse_snapshot_singleflight:
        Mutex<HashMap<ParseSnapshotSingleflightKey, Arc<ParseSnapshotSingleflightEntry>>>,
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
            parse_snapshot_singleflight: Mutex::new(HashMap::new()),
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
            parse_snapshot_singleflight: Mutex::new(HashMap::new()),
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
            parse_snapshot_singleflight: Mutex::new(HashMap::new()),
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

        if let Some((old_tree, _old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree: old_tree.clone(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                });
            }
        }

        let key = ParseSnapshotSingleflightKey {
            file_path: file_path.clone(),
            content_hash: new_hash,
        };
        let (entry, is_leader) = self.begin_parse_snapshot_singleflight(key.clone());
        if !is_leader {
            return Self::wait_parse_snapshot_singleflight(entry.as_ref());
        }

        let result = self.parse_incremental_with_report_singleflight_leader(
            file_path,
            new_content,
            edits,
            new_hash,
            new_tree_hash,
        );
        self.complete_parse_snapshot_singleflight(&key, &entry, &result);
        result
    }

    pub fn parse_incremental_with_report_with_cancellation(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
        cancellation_flag: &AtomicBool,
    ) -> Result<ParseSnapshotReport, String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);

        if let Some((old_tree, _old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree: old_tree.clone(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                });
            }
        }

        let key = ParseSnapshotSingleflightKey {
            file_path: file_path.clone(),
            content_hash: new_hash,
        };
        let (entry, is_leader) = self.begin_parse_snapshot_singleflight(key.clone());
        if !is_leader {
            return Self::wait_parse_snapshot_singleflight_with_cancellation(
                entry.as_ref(),
                cancellation_flag,
            );
        }

        let result = self.parse_incremental_with_report_singleflight_leader_with_cancellation(
            file_path,
            new_content,
            edits,
            new_hash,
            new_tree_hash,
            cancellation_flag,
        );
        self.complete_parse_snapshot_singleflight(&key, &entry, &result);
        result
    }

    pub fn parse_incremental_with_report_with_cancellation_and_options(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> Result<ParseSnapshotReport, String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);

        if let Some((old_tree, _old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                Self::notify_parse_snapshot_exec_subphase(
                    &options,
                    ParseSnapshotExecSubphase::CoreParseBuild,
                );
                let core_started = std::time::Instant::now();
                let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                let mut parse_exec_subphases = ParseSnapshotExecSubphaseAttribution {
                    core_parse_build_ms: Some(duration_to_u64_ms(core_started.elapsed())),
                    ..Default::default()
                };
                match self.run_optional_cache_enrichment_with_cancellation(
                    &file_path,
                    new_hash,
                    &new_content,
                    &result,
                    cancellation_flag,
                    options,
                )? {
                    Some(elapsed_ms) => {
                        parse_exec_subphases.optional_cache_enrichment_ms = Some(elapsed_ms);
                    }
                    None => {
                        parse_exec_subphases.deferred_optional_cache_enrichment = true;
                    }
                }
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree: old_tree.clone(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                    parse_exec_subphases,
                });
            }
        }

        let key = ParseSnapshotSingleflightKey {
            file_path: file_path.clone(),
            content_hash: new_hash,
        };
        let (entry, is_leader) = self.begin_parse_snapshot_singleflight(key.clone());
        if !is_leader {
            return Self::wait_parse_snapshot_singleflight_with_cancellation(
                entry.as_ref(),
                cancellation_flag,
            );
        }

        let result = self
            .parse_incremental_with_report_singleflight_leader_with_cancellation_and_options(
                file_path,
                new_content,
                edits,
                new_hash,
                new_tree_hash,
                cancellation_flag,
                options,
            );
        self.complete_parse_snapshot_singleflight(&key, &entry, &result);
        result
    }

    pub fn parse_full_with_report(
        &self,
        file_path: PathBuf,
        new_content: String,
        fallback_reason: &'static str,
    ) -> Result<ParseSnapshotReport, String> {
        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);
        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        if let Some((old_tree, _old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree: old_tree.clone(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                });
            }
        }

        debug!("Forced full parse for file: {:?}", file_path);
        maybe_inject_parse_snapshot_full_parse_delay_for_test();
        record_parse_snapshot_full_parse_attempt_for_test();
        match self.tree_sitter.parse_with_tree(&new_content) {
            Ok((tree, program)) => {
                let backend_tree = Arc::new(tree.clone());
                self.store_ast_cache(new_hash, &program, Some(file_path.as_path()), &new_content);
                self.tree_cache
                    .set(file_path, tree, new_content.clone(), new_tree_hash);
                Ok(ParseSnapshotReport {
                    parse_result: program,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree,
                    backend_tree_hash: new_tree_hash,
                    incremental: false,
                    fallback_reason: Some(fallback_reason.to_string()),
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                })
            }
            Err(error) => {
                error!(
                    "TreeSitter parsing failed during forced full parse: {}",
                    error
                );
                Err(format!("Tree-sitter parsing failed: {}", error))
            }
        }
    }

    pub fn parse_full_with_report_with_cancellation(
        &self,
        file_path: PathBuf,
        new_content: String,
        fallback_reason: &'static str,
        cancellation_flag: &AtomicBool,
    ) -> Result<ParseSnapshotReport, String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);
        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        if let Some((old_tree, _old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree: old_tree.clone(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                });
            }
        }

        debug!("Forced full parse for file: {:?}", file_path);
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        maybe_inject_parse_snapshot_full_parse_delay_for_test();
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        record_parse_snapshot_full_parse_attempt_for_test();
        match self
            .tree_sitter
            .parse_with_tree_cancellation(&new_content, None, cancellation_flag)
        {
            Ok((tree, program)) => {
                let backend_tree = Arc::new(tree.clone());
                self.store_ast_cache(new_hash, &program, Some(file_path.as_path()), &new_content);
                self.tree_cache
                    .set(file_path, tree, new_content.clone(), new_tree_hash);
                Ok(ParseSnapshotReport {
                    parse_result: program,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree,
                    backend_tree_hash: new_tree_hash,
                    incremental: false,
                    fallback_reason: Some(fallback_reason.to_string()),
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                })
            }
            Err(error) => {
                if is_parse_cancelled_error(&error) {
                    Err(error)
                } else {
                    error!(
                        "TreeSitter parsing failed during forced full parse: {}",
                        error
                    );
                    Err(format!("Tree-sitter parsing failed: {}", error))
                }
            }
        }
    }

    fn finalize_parse_snapshot_report_with_options(
        &self,
        file_path: &PathBuf,
        new_content: &str,
        new_hash: [u8; 32],
        new_tree_hash: u64,
        line_index: Arc<bsl_line_index::LineIndex>,
        tree: tree_sitter::Tree,
        parse_result: ParseResult,
        changed_ranges: Vec<ParseChangedRange>,
        incremental: bool,
        fallback_reason: Option<String>,
        core_started: std::time::Instant,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
        tree_cache_install_op: Option<ParseSnapshotTreeCacheInstallOp>,
        deferred_syntax_error_assembly: bool,
    ) -> Result<ParseSnapshotReport, String> {
        let backend_tree = Arc::new(tree.clone());
        let mut parse_exec_subphases = ParseSnapshotExecSubphaseAttribution {
            ..Default::default()
        };
        parse_exec_subphases.deferred_syntax_error_assembly = deferred_syntax_error_assembly;
        if let Some(install_op) = tree_cache_install_op {
            match self.run_tree_cache_install_with_cancellation(
                install_op,
                cancellation_flag,
                options,
            )? {
                Some(_) => {}
                None => {
                    parse_exec_subphases.deferred_tree_cache_install = true;
                }
            }
        }
        parse_exec_subphases.core_parse_build_ms = Some(duration_to_u64_ms(core_started.elapsed()));
        match self.run_optional_cache_enrichment_with_cancellation(
            file_path.as_path(),
            new_hash,
            new_content,
            &parse_result,
            cancellation_flag,
            options,
        )? {
            Some(elapsed_ms) => {
                parse_exec_subphases.optional_cache_enrichment_ms = Some(elapsed_ms);
            }
            None => {
                parse_exec_subphases.deferred_optional_cache_enrichment = true;
            }
        }
        Ok(ParseSnapshotReport {
            parse_result,
            line_index,
            changed_ranges,
            backend_tree,
            backend_tree_hash: new_tree_hash,
            incremental,
            fallback_reason,
            parse_exec_subphases,
        })
    }

    pub fn parse_full_with_report_with_cancellation_and_options(
        &self,
        file_path: PathBuf,
        new_content: String,
        fallback_reason: &'static str,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> Result<ParseSnapshotReport, String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);
        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        if let Some((old_tree, _old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                Self::notify_parse_snapshot_exec_subphase(
                    &options,
                    ParseSnapshotExecSubphase::CoreParseBuild,
                );
                let core_started = std::time::Instant::now();
                Self::notify_parse_snapshot_core_build_checkpoint(
                    &options,
                    ParseSnapshotCoreBuildCheckpoint::ExactReadySnapshotAssembly,
                );
                debug!("Content unchanged, using cached tree");
                let (result, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &old_tree,
                        &new_content,
                        cancellation_flag,
                        options,
                    )?;
                return self.finalize_parse_snapshot_report_with_options(
                    &file_path,
                    &new_content,
                    new_hash,
                    new_tree_hash,
                    line_index,
                    old_tree.as_ref().clone(),
                    result,
                    Vec::new(),
                    true,
                    None,
                    core_started,
                    cancellation_flag,
                    options,
                    None,
                    deferred_syntax_error_assembly,
                );
            }
        }

        debug!("Forced full parse for file: {:?}", file_path);
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        maybe_inject_parse_snapshot_full_parse_delay_for_test();
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        record_parse_snapshot_full_parse_attempt_for_test();
        Self::notify_parse_snapshot_exec_subphase(
            &options,
            ParseSnapshotExecSubphase::CoreParseBuild,
        );
        let core_started = std::time::Instant::now();
        Self::notify_parse_snapshot_core_build_checkpoint(
            &options,
            ParseSnapshotCoreBuildCheckpoint::ParserTreeBuild,
        );
        match self.tree_sitter.parse_tree_only_with_cancellation(
            &new_content,
            None,
            cancellation_flag,
        ) {
            Ok(tree) => {
                let (program, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &tree,
                        &new_content,
                        cancellation_flag,
                        options,
                    )?;
                let tree_for_cache = tree.clone();
                self.finalize_parse_snapshot_report_with_options(
                    &file_path,
                    &new_content,
                    new_hash,
                    new_tree_hash,
                    line_index,
                    tree,
                    program,
                    Vec::new(),
                    false,
                    Some(fallback_reason.to_string()),
                    core_started,
                    cancellation_flag,
                    options,
                    Some(ParseSnapshotTreeCacheInstallOp::Set {
                        file_path: file_path.clone(),
                        tree: tree_for_cache,
                        source: new_content.clone(),
                        content_hash: new_tree_hash,
                    }),
                    deferred_syntax_error_assembly,
                )
            }
            Err(error) => {
                if is_parse_cancelled_error(&error) {
                    Err(error)
                } else {
                    error!(
                        "TreeSitter parsing failed during forced full parse: {}",
                        error
                    );
                    Err(format!("Tree-sitter parsing failed: {}", error))
                }
            }
        }
    }

    pub fn tree_cache_matches_source_for_file(&self, file_path: &Path, source: &str) -> bool {
        self.tree_cache.get(&file_path.to_path_buf()).is_some_and(
            |(_, cached_source, cached_hash)| {
                cached_hash == hash_content(source) && cached_source == source
            },
        )
    }

    pub fn parse_snapshot_fallback_stale_parser_base_reason() -> &'static str {
        PARSE_SNAPSHOT_FALLBACK_STALE_PARSER_BASE
    }

    pub fn parse_current_context_with_cancellation(
        &self,
        file_path: PathBuf,
        new_content: String,
        cancellation_flag: &AtomicBool,
    ) -> Result<CurrentContextParseReport, String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let new_hash = ast_cache_key(&new_content);
        let new_tree_hash = hash_content(&new_content);
        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        if let Some((old_tree, _old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(CurrentContextParseReport {
                    parse_result: result,
                    line_index,
                });
            }
        }

        debug!("Current-context full parse for file: {:?}", file_path);
        record_parse_snapshot_full_parse_attempt_for_test();
        let (tree, result) =
            self.tree_sitter
                .parse_with_tree_cancellation(&new_content, None, cancellation_flag)?;
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        self.store_ast_cache(new_hash, &result, Some(file_path.as_path()), &new_content);
        self.tree_cache
            .set(file_path, tree, new_content.clone(), new_tree_hash);

        Ok(CurrentContextParseReport {
            parse_result: result,
            line_index,
        })
    }

    pub fn prime_tree_cache_for_file(
        &self,
        file_path: PathBuf,
        source: String,
        backend_tree: Arc<tree_sitter::Tree>,
        content_hash: u64,
    ) {
        if self
            .tree_cache
            .get(&file_path)
            .is_some_and(|(_, _, cached_hash)| cached_hash == content_hash)
        {
            return;
        }
        self.tree_cache
            .set_shared(file_path, backend_tree, source, content_hash);
    }

    fn begin_parse_snapshot_singleflight(
        &self,
        key: ParseSnapshotSingleflightKey,
    ) -> (Arc<ParseSnapshotSingleflightEntry>, bool) {
        let mut singleflight = self
            .parse_snapshot_singleflight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = singleflight.get(&key) {
            return (Arc::clone(existing), false);
        }
        let entry = Arc::new(ParseSnapshotSingleflightEntry {
            result: Mutex::new(None),
            ready: Condvar::new(),
        });
        singleflight.insert(key, Arc::clone(&entry));
        (entry, true)
    }

    fn wait_parse_snapshot_singleflight(
        entry: &ParseSnapshotSingleflightEntry,
    ) -> Result<ParseSnapshotReport, String> {
        let mut result = entry
            .result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(ready) = result.as_ref() {
                return ready.clone();
            }
            result = entry
                .ready
                .wait(result)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    fn wait_parse_snapshot_singleflight_with_cancellation(
        entry: &ParseSnapshotSingleflightEntry,
        cancellation_flag: &AtomicBool,
    ) -> Result<ParseSnapshotReport, String> {
        let mut result = entry
            .result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(ready) = result.as_ref() {
                return ready.clone();
            }
            if cancellation_flag.load(Ordering::SeqCst) {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
            let (next_result, _) = entry
                .ready
                .wait_timeout(result, Duration::from_millis(25))
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            result = next_result;
        }
    }

    fn complete_parse_snapshot_singleflight(
        &self,
        key: &ParseSnapshotSingleflightKey,
        entry: &Arc<ParseSnapshotSingleflightEntry>,
        result: &Result<ParseSnapshotReport, String>,
    ) {
        {
            let mut slot = entry
                .result
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *slot = Some(result.clone());
        }
        entry.ready.notify_all();
        let mut singleflight = self
            .parse_snapshot_singleflight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if singleflight
            .get(key)
            .is_some_and(|current| Arc::ptr_eq(current, entry))
        {
            singleflight.remove(key);
        }
    }

    fn parse_incremental_with_report_singleflight_leader(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
        new_hash: [u8; 32],
        new_tree_hash: u64,
    ) -> Result<ParseSnapshotReport, String> {
        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        if let Some((old_tree, old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree: old_tree.clone(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                });
            }

            debug!("Applying {} edits incrementally", edits.len());

            match self.tree_sitter.parse_incremental(
                &new_content,
                Some(&old_tree),
                edits,
                &old_source,
            ) {
                Ok((new_tree, program, changed_ranges)) => {
                    let backend_tree = Arc::new(new_tree.clone());
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
                        backend_tree,
                        backend_tree_hash: new_tree_hash,
                        incremental: true,
                        fallback_reason: None,
                        parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                    });
                }
                Err(e) => {
                    warn!(
                        "Incremental parsing failed: {}, falling back to full parse",
                        e
                    );
                    let fallback_reason =
                        Some(canonical_parse_snapshot_fallback_reason(&e).to_string());
                    debug!("Full parse for file (fallback): {:?}", file_path);
                    maybe_inject_parse_snapshot_full_parse_delay_for_test();
                    record_parse_snapshot_full_parse_attempt_for_test();
                    return match self.tree_sitter.parse_with_tree(&new_content) {
                        Ok((tree, program)) => {
                            let backend_tree = Arc::new(tree.clone());
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
                                backend_tree,
                                backend_tree_hash: new_tree_hash,
                                incremental: false,
                                fallback_reason,
                                parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(
                                ),
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

        debug!("Full parse for file: {:?}", file_path);
        maybe_inject_parse_snapshot_full_parse_delay_for_test();
        record_parse_snapshot_full_parse_attempt_for_test();
        match self.tree_sitter.parse_with_tree(&new_content) {
            Ok((tree, program)) => {
                let backend_tree = Arc::new(tree.clone());
                self.store_ast_cache(new_hash, &program, Some(file_path.as_path()), &new_content);
                self.tree_cache
                    .set(file_path, tree, new_content.clone(), new_tree_hash);
                Ok(ParseSnapshotReport {
                    parse_result: program,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree,
                    backend_tree_hash: new_tree_hash,
                    incremental: false,
                    fallback_reason: Some(PARSE_SNAPSHOT_FALLBACK_NO_PREVIOUS_TREE.to_string()),
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                })
            }
            Err(e) => {
                error!("TreeSitter parsing failed: {}", e);
                Err(format!("Tree-sitter parsing failed: {}", e))
            }
        }
    }

    fn parse_incremental_with_report_singleflight_leader_with_cancellation(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
        new_hash: [u8; 32],
        new_tree_hash: u64,
        cancellation_flag: &AtomicBool,
    ) -> Result<ParseSnapshotReport, String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        if let Some((old_tree, old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                debug!("Content unchanged, using cached tree");
                let result = TreeSitterAdapter::convert_tree(&old_tree, &new_content)?;
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                self.store_ast_memory(new_hash, &result);
                self.update_symbol_index(&file_path, &result);
                return Ok(ParseSnapshotReport {
                    parse_result: result,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree: old_tree.clone(),
                    backend_tree_hash: new_tree_hash,
                    incremental: true,
                    fallback_reason: None,
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                });
            }

            debug!("Applying {} edits incrementally", edits.len());

            match self.tree_sitter.parse_incremental_with_cancellation(
                &new_content,
                Some(&old_tree),
                edits,
                &old_source,
                cancellation_flag,
            ) {
                Ok((new_tree, program, changed_ranges)) => {
                    let backend_tree = Arc::new(new_tree.clone());
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
                        backend_tree,
                        backend_tree_hash: new_tree_hash,
                        incremental: true,
                        fallback_reason: None,
                        parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                    });
                }
                Err(error) if is_parse_cancelled_error(&error) => {
                    return Err(error);
                }
                Err(error) => {
                    warn!(
                        "Incremental parsing failed: {}, falling back to full parse",
                        error
                    );
                    let fallback_reason =
                        Some(canonical_parse_snapshot_fallback_reason(&error).to_string());
                    debug!("Full parse for file (fallback): {:?}", file_path);
                    if cancellation_flag.load(Ordering::SeqCst) {
                        return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                    }
                    maybe_inject_parse_snapshot_full_parse_delay_for_test();
                    if cancellation_flag.load(Ordering::SeqCst) {
                        return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                    }
                    record_parse_snapshot_full_parse_attempt_for_test();
                    return match self.tree_sitter.parse_with_tree_cancellation(
                        &new_content,
                        None,
                        cancellation_flag,
                    ) {
                        Ok((tree, program)) => {
                            let backend_tree = Arc::new(tree.clone());
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
                                backend_tree,
                                backend_tree_hash: new_tree_hash,
                                incremental: false,
                                fallback_reason,
                                parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(
                                ),
                            })
                        }
                        Err(parse_error) => {
                            if is_parse_cancelled_error(&parse_error) {
                                Err(parse_error)
                            } else {
                                error!("TreeSitter parsing failed after fallback: {}", parse_error);
                                Err(format!("Tree-sitter parsing failed: {}", parse_error))
                            }
                        }
                    };
                }
            }
        }

        debug!("Full parse for file: {:?}", file_path);
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        maybe_inject_parse_snapshot_full_parse_delay_for_test();
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        record_parse_snapshot_full_parse_attempt_for_test();
        match self
            .tree_sitter
            .parse_with_tree_cancellation(&new_content, None, cancellation_flag)
        {
            Ok((tree, program)) => {
                let backend_tree = Arc::new(tree.clone());
                self.store_ast_cache(new_hash, &program, Some(file_path.as_path()), &new_content);
                self.tree_cache
                    .set(file_path, tree, new_content.clone(), new_tree_hash);
                Ok(ParseSnapshotReport {
                    parse_result: program,
                    line_index,
                    changed_ranges: Vec::new(),
                    backend_tree,
                    backend_tree_hash: new_tree_hash,
                    incremental: false,
                    fallback_reason: Some(PARSE_SNAPSHOT_FALLBACK_NO_PREVIOUS_TREE.to_string()),
                    parse_exec_subphases: ParseSnapshotExecSubphaseAttribution::default(),
                })
            }
            Err(error) => {
                if is_parse_cancelled_error(&error) {
                    Err(error)
                } else {
                    error!("TreeSitter parsing failed: {}", error);
                    Err(format!("Tree-sitter parsing failed: {}", error))
                }
            }
        }
    }

    fn parse_incremental_with_report_singleflight_leader_with_cancellation_and_options(
        &self,
        file_path: PathBuf,
        new_content: String,
        edits: Vec<TextEdit>,
        new_hash: [u8; 32],
        new_tree_hash: u64,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> Result<ParseSnapshotReport, String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let line_index = Arc::new(bsl_line_index::LineIndex::new(&new_content));

        if let Some((old_tree, old_source, old_hash)) = self.tree_cache.get(&file_path) {
            if old_hash == new_tree_hash {
                Self::notify_parse_snapshot_exec_subphase(
                    &options,
                    ParseSnapshotExecSubphase::CoreParseBuild,
                );
                let core_started = std::time::Instant::now();
                debug!("Content unchanged, using cached tree");
                let (result, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &old_tree,
                        &new_content,
                        cancellation_flag,
                        options,
                    )?;
                return self.finalize_parse_snapshot_report_with_options(
                    &file_path,
                    &new_content,
                    new_hash,
                    new_tree_hash,
                    line_index,
                    old_tree.as_ref().clone(),
                    result,
                    Vec::new(),
                    true,
                    None,
                    core_started,
                    cancellation_flag,
                    options,
                    None,
                    deferred_syntax_error_assembly,
                );
            }

            debug!("Applying {} edits incrementally", edits.len());
            Self::notify_parse_snapshot_exec_subphase(
                &options,
                ParseSnapshotExecSubphase::CoreParseBuild,
            );
            let core_started = std::time::Instant::now();
            Self::notify_parse_snapshot_core_build_checkpoint(
                &options,
                ParseSnapshotCoreBuildCheckpoint::ParserTreeBuild,
            );
            let fallback_reason = match self
                .tree_sitter
                .parse_incremental_tree_only_with_cancellation(
                    &new_content,
                    Some(&old_tree),
                    edits,
                    &old_source,
                    cancellation_flag,
                ) {
                Ok((new_tree, changed_ranges)) => {
                    if let Some(forced_error) = maybe_force_incremental_adapter_error_for_test() {
                        warn!(
                            "Incremental tree-to-AST conversion failed: {}, falling back to full parse",
                            forced_error
                        );
                        Some(PARSE_SNAPSHOT_FALLBACK_INCREMENTAL_PARSE_FAILED.to_string())
                    } else {
                        match self.run_exact_ready_snapshot_assembly_with_cancellation(
                            &new_tree,
                            &new_content,
                            cancellation_flag,
                            options,
                        ) {
                            Ok((program, deferred_syntax_error_assembly)) => {
                                return self.finalize_parse_snapshot_report_with_options(
                                    &file_path,
                                    &new_content,
                                    new_hash,
                                    new_tree_hash,
                                    line_index,
                                    new_tree.clone(),
                                    program,
                                    changed_ranges,
                                    true,
                                    None,
                                    core_started,
                                    cancellation_flag,
                                    options,
                                    Some(ParseSnapshotTreeCacheInstallOp::Update {
                                        file_path: file_path.clone(),
                                        tree: new_tree,
                                        source: new_content.clone(),
                                        content_hash: new_tree_hash,
                                    }),
                                    deferred_syntax_error_assembly,
                                );
                            }
                            Err(_error) if cancellation_flag.load(Ordering::SeqCst) => {
                                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                            }
                            Err(error) => {
                                warn!(
                                    "Incremental tree-to-AST conversion failed: {}, falling back to full parse",
                                    error
                                );
                                Some(PARSE_SNAPSHOT_FALLBACK_INCREMENTAL_PARSE_FAILED.to_string())
                            }
                        }
                    }
                }
                Err(error) if is_parse_cancelled_error(&error) => {
                    return Err(error);
                }
                Err(error) => {
                    warn!(
                        "Incremental parsing failed: {}, falling back to full parse",
                        error
                    );
                    Some(canonical_parse_snapshot_fallback_reason(&error).to_string())
                }
            };

            debug!("Full parse for file (fallback): {:?}", file_path);
            if cancellation_flag.load(Ordering::SeqCst) {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
            maybe_inject_parse_snapshot_full_parse_delay_for_test();
            if cancellation_flag.load(Ordering::SeqCst) {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
            record_parse_snapshot_full_parse_attempt_for_test();
            Self::notify_parse_snapshot_exec_subphase(
                &options,
                ParseSnapshotExecSubphase::CoreParseBuild,
            );
            let fallback_core_started = std::time::Instant::now();
            Self::notify_parse_snapshot_core_build_checkpoint(
                &options,
                ParseSnapshotCoreBuildCheckpoint::ParserTreeBuild,
            );
            return match self.tree_sitter.parse_tree_only_with_cancellation(
                &new_content,
                None,
                cancellation_flag,
            ) {
                Ok(tree) => {
                    let (program, deferred_syntax_error_assembly) = self
                        .run_exact_ready_snapshot_assembly_with_cancellation(
                            &tree,
                            &new_content,
                            cancellation_flag,
                            options,
                        )?;
                    self.finalize_parse_snapshot_report_with_options(
                        &file_path,
                        &new_content,
                        new_hash,
                        new_tree_hash,
                        line_index,
                        tree.clone(),
                        program,
                        Vec::new(),
                        false,
                        fallback_reason,
                        fallback_core_started,
                        cancellation_flag,
                        options,
                        Some(ParseSnapshotTreeCacheInstallOp::Set {
                            file_path: file_path.clone(),
                            tree,
                            source: new_content.clone(),
                            content_hash: new_tree_hash,
                        }),
                        deferred_syntax_error_assembly,
                    )
                }
                Err(parse_error) => {
                    if is_parse_cancelled_error(&parse_error) {
                        Err(parse_error)
                    } else {
                        error!("TreeSitter parsing failed after fallback: {}", parse_error);
                        Err(format!("Tree-sitter parsing failed: {}", parse_error))
                    }
                }
            };
        }

        debug!("Full parse for file: {:?}", file_path);
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        maybe_inject_parse_snapshot_full_parse_delay_for_test();
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        record_parse_snapshot_full_parse_attempt_for_test();
        Self::notify_parse_snapshot_exec_subphase(
            &options,
            ParseSnapshotExecSubphase::CoreParseBuild,
        );
        let core_started = std::time::Instant::now();
        Self::notify_parse_snapshot_core_build_checkpoint(
            &options,
            ParseSnapshotCoreBuildCheckpoint::ParserTreeBuild,
        );
        match self.tree_sitter.parse_tree_only_with_cancellation(
            &new_content,
            None,
            cancellation_flag,
        ) {
            Ok(tree) => {
                let (program, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &tree,
                        &new_content,
                        cancellation_flag,
                        options,
                    )?;
                self.finalize_parse_snapshot_report_with_options(
                    &file_path,
                    &new_content,
                    new_hash,
                    new_tree_hash,
                    line_index,
                    tree.clone(),
                    program,
                    Vec::new(),
                    false,
                    Some(PARSE_SNAPSHOT_FALLBACK_NO_PREVIOUS_TREE.to_string()),
                    core_started,
                    cancellation_flag,
                    options,
                    Some(ParseSnapshotTreeCacheInstallOp::Set {
                        file_path: file_path.clone(),
                        tree,
                        source: new_content.clone(),
                        content_hash: new_tree_hash,
                    }),
                    deferred_syntax_error_assembly,
                )
            }
            Err(error) => {
                if is_parse_cancelled_error(&error) {
                    Err(error)
                } else {
                    error!("TreeSitter parsing failed: {}", error);
                    Err(format!("Tree-sitter parsing failed: {}", error))
                }
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

    fn notify_parse_snapshot_exec_subphase(
        options: &ParseSnapshotExecutionOptions<'_>,
        subphase: ParseSnapshotExecSubphase,
    ) {
        if let Some(callback) = options.progress_callback {
            callback(subphase);
        }
    }

    fn notify_parse_snapshot_core_build_checkpoint(
        options: &ParseSnapshotExecutionOptions<'_>,
        checkpoint: ParseSnapshotCoreBuildCheckpoint,
    ) {
        if let Some(callback) = options.core_build_progress_callback {
            callback(checkpoint);
        }
    }

    fn notify_parse_snapshot_assembly_checkpoint(
        options: &ParseSnapshotExecutionOptions<'_>,
        checkpoint: ParseSnapshotAssemblyCheckpoint,
    ) {
        if let Some(callback) = options.assembly_progress_callback {
            callback(checkpoint);
        }
    }

    fn save_critical_requested(options: ParseSnapshotExecutionOptions<'_>) -> bool {
        options.save_critical_initial
            || options
                .save_critical_requested
                .is_some_and(|flag| flag.load(Ordering::SeqCst))
    }

    fn run_exact_ready_snapshot_assembly_with_cancellation(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> Result<(ParseResult, bool), String> {
        Self::notify_parse_snapshot_core_build_checkpoint(
            &options,
            ParseSnapshotCoreBuildCheckpoint::ExactReadySnapshotAssembly,
        );
        Self::notify_parse_snapshot_assembly_checkpoint(
            &options,
            ParseSnapshotAssemblyCheckpoint::ProgramLowering,
        );
        let parse_result = TreeSitterAdapter::convert_tree_fast_with_observer(
            tree,
            content,
            |_, _| {
                maybe_inject_parse_snapshot_program_conversion_progress_delay_for_test();
                if cancellation_flag.load(Ordering::SeqCst) {
                    Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string())
                } else {
                    Ok(())
                }
            },
        )?;
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        Self::notify_parse_snapshot_assembly_checkpoint(
            &options,
            ParseSnapshotAssemblyCheckpoint::PublishableArtifactPackaging,
        );
        if let Some(delay_ms) =
            maybe_inject_parse_snapshot_publishable_artifact_packaging_delay_for_test()
        {
            let deadline = std::time::Instant::now() + Duration::from_millis(delay_ms);
            while std::time::Instant::now() < deadline {
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                if Self::save_critical_requested(options) {
                    return Ok((parse_result, true));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        if Self::save_critical_requested(options) {
            return Ok((parse_result, true));
        }

        Self::notify_parse_snapshot_assembly_checkpoint(
            &options,
            ParseSnapshotAssemblyCheckpoint::SyntaxErrorCollection,
        );
        if let Some(delay_ms) = maybe_inject_parse_snapshot_syntax_error_assembly_delay_for_test() {
            let deadline = std::time::Instant::now() + Duration::from_millis(delay_ms);
            while std::time::Instant::now() < deadline {
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                if Self::save_critical_requested(options) {
                    return Ok((parse_result, true));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        if Self::save_critical_requested(options) {
            return Ok((parse_result, true));
        }

        let syntax_errors = TreeSitterAdapter::collect_syntax_errors_only(tree, content);
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        if syntax_errors.is_empty() {
            Ok((parse_result, false))
        } else {
            Ok((
                ParseResult::with_errors(parse_result.program, syntax_errors),
                false,
            ))
        }
    }

    fn run_optional_cache_enrichment_with_cancellation(
        &self,
        file_path: &Path,
        content_hash: [u8; 32],
        content: &str,
        result: &ParseResult,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> Result<Option<u64>, String> {
        if Self::save_critical_requested(options) {
            return Ok(None);
        }

        Self::notify_parse_snapshot_exec_subphase(
            &options,
            ParseSnapshotExecSubphase::OptionalCacheEnrichment,
        );
        let optional_started = std::time::Instant::now();
        if let Some(delay_ms) =
            maybe_inject_parse_snapshot_optional_cache_enrichment_delay_for_test()
        {
            let deadline = std::time::Instant::now() + Duration::from_millis(delay_ms);
            while std::time::Instant::now() < deadline {
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                if Self::save_critical_requested(options) {
                    return Ok(None);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        if Self::save_critical_requested(options) {
            return Ok(None);
        }

        self.store_ast_memory(content_hash, result);
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        if Self::save_critical_requested(options) {
            return Ok(None);
        }

        self.update_symbol_index(file_path, result);
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        if Self::save_critical_requested(options) {
            return Ok(None);
        }

        if file_path.exists() {
            let path_str = file_path.to_string_lossy();
            let _ = self.store_ast_in_disk(&path_str, content, result);
        }
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        Ok(Some(duration_to_u64_ms(optional_started.elapsed())))
    }

    fn run_tree_cache_install_with_cancellation(
        &self,
        install_op: ParseSnapshotTreeCacheInstallOp,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> Result<Option<u64>, String> {
        if Self::save_critical_requested(options) {
            return Ok(None);
        }

        Self::notify_parse_snapshot_core_build_checkpoint(
            &options,
            ParseSnapshotCoreBuildCheckpoint::TreeCacheInstall,
        );
        let tree_cache_started = std::time::Instant::now();
        if let Some(delay_ms) = maybe_inject_parse_snapshot_tree_cache_install_delay_for_test() {
            let deadline = std::time::Instant::now() + Duration::from_millis(delay_ms);
            while std::time::Instant::now() < deadline {
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                if Self::save_critical_requested(options) {
                    return Ok(None);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        if Self::save_critical_requested(options) {
            return Ok(None);
        }

        match install_op {
            ParseSnapshotTreeCacheInstallOp::Set {
                file_path,
                tree,
                source,
                content_hash,
            } => {
                self.tree_cache.set(file_path, tree, source, content_hash);
            }
            ParseSnapshotTreeCacheInstallOp::Update {
                file_path,
                tree,
                source,
                content_hash,
            } => {
                self.tree_cache
                    .update(file_path.as_path(), tree, source, content_hash);
            }
        }

        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        Ok(Some(duration_to_u64_ms(tree_cache_started.elapsed())))
    }

    pub fn complete_deferred_parse_snapshot_cache_enrichment(
        &self,
        file_path: &Path,
        content: &str,
        result: &ParseResult,
        update_symbol_index: bool,
    ) {
        let content_hash = ast_cache_key(content);
        self.store_ast_memory(content_hash, result);
        if update_symbol_index {
            self.update_symbol_index(file_path, result);
        }
        if file_path.exists() {
            let path_str = file_path.to_string_lossy();
            let _ = self.store_ast_in_disk(&path_str, content, result);
        }
    }

    pub fn complete_deferred_parse_snapshot_syntax_error_assembly(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
        result: &ParseResult,
    ) -> ParseResult {
        let syntax_errors = TreeSitterAdapter::collect_syntax_errors_only(tree, content);
        if syntax_errors.is_empty() {
            ParseResult::success(result.program.clone())
        } else {
            ParseResult::with_errors(result.program.clone(), syntax_errors)
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

pub fn is_parse_cancelled_error(error: &str) -> bool {
    error == PARSE_COORDINATOR_CANCELLED_ERROR
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

    fn parse_with_tree_cancellation(
        &self,
        content: &str,
        old_tree: Option<&tree_sitter::Tree>,
        cancellation_flag: &AtomicBool,
    ) -> Result<(tree_sitter::Tree, ParseResult), String> {
        let tree = self.parse_tree_only_with_cancellation(content, old_tree, cancellation_flag)?;
        let result = TreeSitterAdapter::convert_tree(&tree, content)?;
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        Ok((tree, result))
    }

    fn parse_tree_only_with_cancellation(
        &self,
        content: &str,
        old_tree: Option<&tree_sitter::Tree>,
        cancellation_flag: &AtomicBool,
    ) -> Result<tree_sitter::Tree, String> {
        let mut parser = self
            .parser
            .lock()
            .map_err(|e| format!("Failed to lock parser: {}", e))?;

        let bytes = content.as_bytes();
        let len = bytes.len();
        let mut progress = |_state: &tree_sitter::ParseState| {
            maybe_inject_current_context_parse_progress_delay_for_test();
            maybe_inject_parse_snapshot_parse_progress_delay_for_test();
            cancellation_flag.load(Ordering::SeqCst)
        };
        let tree = parser
            .parse_with_options(
                &mut |i, _| (i < len).then(|| &bytes[i..]).unwrap_or_default(),
                old_tree,
                Some(tree_sitter::ParseOptions::new().progress_callback(&mut progress)),
            )
            .ok_or_else(|| {
                if cancellation_flag.load(Ordering::SeqCst) {
                    PARSE_COORDINATOR_CANCELLED_ERROR.to_string()
                } else {
                    "Tree-sitter parsing failed".to_string()
                }
            })?;
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        Ok(tree)
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
                    Self::text_edit_to_input_edit(&edit, &current_source)
                        .map_err(|err| format!("Input edit conversion failed: {err}"))?;
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

            if maybe_force_incremental_parse_failure_for_test() {
                return Err("Incremental parsing failed".to_string());
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

            if let Some(forced_error) = maybe_force_incremental_adapter_error_for_test() {
                warn!(
                    "Incremental tree-to-AST conversion failed: {}",
                    forced_error
                );
                return Err("Incremental parsing failed".to_string());
            }

            let program =
                TreeSitterAdapter::convert_tree(&new_tree, new_content).map_err(|error| {
                    warn!("Incremental tree-to-AST conversion failed: {}", error);
                    "Incremental parsing failed".to_string()
                })?;
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

    fn parse_incremental_with_cancellation(
        &self,
        new_content: &str,
        old_tree: Option<&tree_sitter::Tree>,
        edits: Vec<TextEdit>,
        old_source: &str,
        cancellation_flag: &AtomicBool,
    ) -> Result<(tree_sitter::Tree, ParseResult, Vec<ParseChangedRange>), String> {
        let (new_tree, changed_ranges) = self.parse_incremental_tree_only_with_cancellation(
            new_content,
            old_tree,
            edits,
            old_source,
            cancellation_flag,
        )?;
        if let Some(forced_error) = maybe_force_incremental_adapter_error_for_test() {
            warn!(
                "Incremental tree-to-AST conversion failed: {}",
                forced_error
            );
            return Err("Incremental parsing failed".to_string());
        }

        let program = TreeSitterAdapter::convert_tree(&new_tree, new_content).map_err(|error| {
            if cancellation_flag.load(Ordering::SeqCst) {
                PARSE_COORDINATOR_CANCELLED_ERROR.to_string()
            } else {
                warn!("Incremental tree-to-AST conversion failed: {}", error);
                "Incremental parsing failed".to_string()
            }
        })?;
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }
        Ok((new_tree, program, changed_ranges))
    }

    fn parse_incremental_tree_only_with_cancellation(
        &self,
        new_content: &str,
        old_tree: Option<&tree_sitter::Tree>,
        edits: Vec<TextEdit>,
        old_source: &str,
        cancellation_flag: &AtomicBool,
    ) -> Result<(tree_sitter::Tree, Vec<ParseChangedRange>), String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let mut parser = self
            .parser
            .lock()
            .map_err(|e| format!("Failed to lock parser: {}", e))?;

        if let Some(mut tree) = old_tree.cloned() {
            if edits.is_empty() {
                return Err("No edits provided for incremental parsing".to_string());
            }

            let mut current_source = old_source.to_string();
            let mut changed_ranges = Vec::with_capacity(edits.len());
            for edit in edits {
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                let (input_edit, start_byte, old_end_byte) =
                    Self::text_edit_to_input_edit(&edit, &current_source)
                        .map_err(|err| format!("Input edit conversion failed: {err}"))?;
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

            if maybe_force_incremental_parse_failure_for_test() {
                return Err("Incremental parsing failed".to_string());
            }

            let bytes = new_content.as_bytes();
            let len = bytes.len();
            let mut progress = |_state: &tree_sitter::ParseState| {
                maybe_inject_parse_snapshot_parse_progress_delay_for_test();
                cancellation_flag.load(Ordering::SeqCst)
            };
            let new_tree = parser
                .parse_with_options(
                    &mut |i, _| (i < len).then(|| &bytes[i..]).unwrap_or_default(),
                    Some(&tree),
                    Some(tree_sitter::ParseOptions::new().progress_callback(&mut progress)),
                )
                .ok_or_else(|| {
                    if cancellation_flag.load(Ordering::SeqCst) {
                        PARSE_COORDINATOR_CANCELLED_ERROR.to_string()
                    } else {
                        "Incremental parsing failed".to_string()
                    }
                })?;

            debug!(
                "Incremental parse: {} nodes, {} bytes",
                new_tree.root_node().descendant_count(),
                new_content.len()
            );

            if cancellation_flag.load(Ordering::SeqCst) {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }

            Ok((new_tree, changed_ranges))
        } else {
            let bytes = new_content.as_bytes();
            let len = bytes.len();
            let mut progress = |_state: &tree_sitter::ParseState| {
                maybe_inject_parse_snapshot_parse_progress_delay_for_test();
                cancellation_flag.load(Ordering::SeqCst)
            };
            let tree = parser
                .parse_with_options(
                    &mut |i, _| (i < len).then(|| &bytes[i..]).unwrap_or_default(),
                    None,
                    Some(tree_sitter::ParseOptions::new().progress_callback(&mut progress)),
                )
                .ok_or_else(|| {
                    if cancellation_flag.load(Ordering::SeqCst) {
                        PARSE_COORDINATOR_CANCELLED_ERROR.to_string()
                    } else {
                        "Tree-sitter parsing failed".to_string()
                    }
                })?;
            if cancellation_flag.load(Ordering::SeqCst) {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
            Ok((tree, Vec::new()))
        }
    }

    /// Конвертировать TextEdit → tree-sitter InputEdit
    fn text_edit_to_input_edit(
        edit: &TextEdit,
        source: &str,
    ) -> Result<(InputEdit, usize, usize), String> {
        use crate::system::positioning::LineIndex;

        let index = LineIndex::new(source);
        Self::validate_utf16_position(
            &index,
            source,
            edit.start_line,
            edit.start_utf16_column,
            "start",
        )?;
        Self::validate_utf16_position(
            &index,
            source,
            edit.old_end_line,
            edit.old_end_utf16_column,
            "old_end",
        )?;
        let start_byte =
            index.utf16_position_to_byte_offset(source, edit.start_line, edit.start_utf16_column);
        let old_end_byte = index.utf16_position_to_byte_offset(
            source,
            edit.old_end_line,
            edit.old_end_utf16_column,
        );
        if old_end_byte < start_byte {
            return Err("Edit end precedes start".to_string());
        }

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

    fn validate_utf16_position(
        index: &crate::system::positioning::LineIndex,
        source: &str,
        line: u32,
        utf16_column: u32,
        label: &str,
    ) -> Result<(), String> {
        let line = line as usize;
        if line >= index.line_count() {
            return Err(format!("{label} line out of range"));
        }
        let line_text = index.line_text(source, line);
        let max_utf16 = line_text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
        if utf16_column > max_utf16 {
            return Err(format!("{label} utf16 column out of range"));
        }
        Ok(())
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
#[path = "parser_coordinator/tests.rs"]
mod tests;
