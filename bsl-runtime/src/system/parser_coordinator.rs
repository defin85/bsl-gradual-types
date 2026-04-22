//! Parser Coordinator - координация Tree-sitter парсера
//!
//! Milestone 2.8 Task 7: Regex fallback удалён, используется только Tree-sitter

#![allow(clippy::explicit_counter_loop)]

use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;
use tracing::{debug, error, warn};
use tree_sitter::{InputEdit, Node, Parser, Point};
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
use crate::system::tree_sitter_adapter::{
    LoweringExecutionAttribution, LoweringReuseNodePlan, LoweringReusePlan,
    LoweringReusePlanOutcome, RoutineBodyLoweringReusePlan, TreeSitterAdapter,
};
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

fn exact_program_lowering_reuse_enabled() -> bool {
    global_runtime_config()
        .get_bool(RuntimeKey::IntellisenseV2ExactProgramLoweringReuseEnabled)
        .unwrap_or(true)
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
    pub program_lowering_summary: ParseSnapshotProgramLoweringSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSnapshotProgramLoweringReuseOutcome {
    FullRebuild,
    ReusedPrefix,
    TopLevelReuse,
    RoutineBodyReuse,
}

impl ParseSnapshotProgramLoweringReuseOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FullRebuild => "full_rebuild",
            Self::ReusedPrefix => "reused_prefix",
            Self::TopLevelReuse => "top_level_reuse",
            Self::RoutineBodyReuse => "routine_body_reuse",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSnapshotProgramLoweringSummary {
    pub reuse_outcome: ParseSnapshotProgramLoweringReuseOutcome,
    pub reused_lowering_units: u64,
    pub rebuilt_lowering_units: u64,
    pub reused_window_count: u64,
    pub rebuilt_window_count: u64,
    pub largest_rebuilt_window_lowering_units: u64,
    pub fully_reused_top_level_node_count: u64,
    pub fully_rebuilt_top_level_node_count: u64,
    pub routine_body_reuse_node_count: u64,
    pub fully_reused_top_level_lowering_units: u64,
    pub fully_rebuilt_top_level_lowering_units: u64,
    pub routine_body_reused_prefix_lowering_units: u64,
    pub routine_body_reused_suffix_lowering_units: u64,
    pub routine_body_rebuilt_lowering_units: u64,
    pub reuse_plan_build_source: Option<ParseSnapshotProgramLoweringReusePlanBuildSource>,
    pub reuse_plan_take_if_unique_hit: Option<bool>,
    pub reuse_plan_borrowed_cache_hit: Option<bool>,
    pub reuse_plan_build_ms: Option<u64>,
    pub reuse_plan_owned_build_ms: Option<u64>,
    pub reuse_plan_borrowed_build_ms: Option<u64>,
    pub reuse_plan_rebase_ms: Option<u64>,
    pub reuse_plan_rebase_statement_count: Option<u64>,
    pub reused_progress_ms: Option<u64>,
    pub reused_progress_call_count: Option<u64>,
    pub rebuild_dispatch_ms: Option<u64>,
    pub rebuild_dispatch_call_count: Option<u64>,
    pub rebuild_dispatch_callable_ms: Option<u64>,
    pub rebuild_dispatch_callable_call_count: Option<u64>,
    pub rebuild_dispatch_callable_body_dispatch_ms: Option<u64>,
    pub rebuild_dispatch_callable_body_dispatch_call_count: Option<u64>,
    pub rebuild_dispatch_callable_non_body_dispatch_ms: Option<u64>,
    pub rebuild_dispatch_control_flow_ms: Option<u64>,
    pub rebuild_dispatch_control_flow_call_count: Option<u64>,
    pub rebuild_dispatch_simple_ms: Option<u64>,
    pub rebuild_dispatch_simple_call_count: Option<u64>,
    pub rebuild_dispatch_other_ms: Option<u64>,
    pub rebuild_dispatch_other_call_count: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSnapshotProgramLoweringReusePlanBuildSource {
    Owned,
    Borrowed,
}

impl ParseSnapshotProgramLoweringReusePlanBuildSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Borrowed => "borrowed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ParseSnapshotProgramLoweringAttribution {
    reuse_plan_build_source: Option<ParseSnapshotProgramLoweringReusePlanBuildSource>,
    reuse_plan_take_if_unique_hit: Option<bool>,
    reuse_plan_borrowed_cache_hit: Option<bool>,
    reuse_plan_build_ms: Option<u64>,
    reuse_plan_owned_build_ms: Option<u64>,
    reuse_plan_borrowed_build_ms: Option<u64>,
    reuse_plan_rebase_ms: Option<u64>,
    reuse_plan_rebase_statement_count: Option<u64>,
    reused_progress_ms: Option<u64>,
    reused_progress_call_count: Option<u64>,
    rebuild_dispatch_ms: Option<u64>,
    rebuild_dispatch_call_count: Option<u64>,
    rebuild_dispatch_callable_ms: Option<u64>,
    rebuild_dispatch_callable_call_count: Option<u64>,
    rebuild_dispatch_callable_body_dispatch_ms: Option<u64>,
    rebuild_dispatch_callable_body_dispatch_call_count: Option<u64>,
    rebuild_dispatch_callable_non_body_dispatch_ms: Option<u64>,
    rebuild_dispatch_control_flow_ms: Option<u64>,
    rebuild_dispatch_control_flow_call_count: Option<u64>,
    rebuild_dispatch_simple_ms: Option<u64>,
    rebuild_dispatch_simple_call_count: Option<u64>,
    rebuild_dispatch_other_ms: Option<u64>,
    rebuild_dispatch_other_call_count: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseSnapshotProgramLoweringSummaryNode {
    ReuseStatement {
        reused_lowering_units: u64,
    },
    Rebuild,
    RebuildRoutineBody {
        reused_prefix_lowering_units: u64,
        reused_suffix_lowering_units: u64,
    },
}

#[derive(Debug, Clone)]
struct ParseSnapshotProgramLoweringSummaryPlan {
    reuse_outcome: ParseSnapshotProgramLoweringReuseOutcome,
    nodes: Vec<ParseSnapshotProgramLoweringSummaryNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutineBodyReuseDecision {
    ReuseWholeStatement,
    ReuseWindow(RoutineBodyReuseWindow),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoutineBodyReuseWindow {
    original_body_len: usize,
    reused_prefix_len: usize,
    reused_suffix_start: usize,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseSnapshotExactReadyControl {
    Continue,
    SaveCritical,
    Cancel,
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
    pub reused_program_prefix: Option<&'a [Statement]>,
    pub lowering_reuse_plan: Option<&'a LoweringReusePlan>,
    lowering_reuse_summary: Option<&'a ParseSnapshotProgramLoweringSummaryPlan>,
    lowering_reuse_attribution: Option<&'a ParseSnapshotProgramLoweringAttribution>,
    pub exact_ready_snapshot_control_callback:
        Option<&'a (dyn Fn() -> ParseSnapshotExactReadyControl + Send + Sync)>,
    pub progress_callback: Option<&'a (dyn Fn(ParseSnapshotExecSubphase) + Send + Sync)>,
    pub core_build_progress_callback:
        Option<&'a (dyn Fn(ParseSnapshotCoreBuildCheckpoint) + Send + Sync)>,
    pub assembly_progress_callback:
        Option<&'a (dyn Fn(ParseSnapshotAssemblyCheckpoint) + Send + Sync)>,
}

#[derive(Clone, Copy, Default)]
pub struct PrimeTreeCacheFromSourceOptions<'a> {
    pub skip_optional_ast_priming_initial: bool,
    pub skip_optional_ast_priming_requested: Option<&'a AtomicBool>,
}

impl PrimeTreeCacheFromSourceOptions<'_> {
    fn skip_optional_ast_priming(self) -> bool {
        self.skip_optional_ast_priming_initial
            || self
                .skip_optional_ast_priming_requested
                .is_some_and(|flag| flag.load(Ordering::SeqCst))
    }
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

    pub fn prime_ast_cache_for_source(
        &self,
        source: &str,
        parse_result: Arc<bsl_syntax::ast::ParseResult>,
    ) {
        self.ast_cache.put(ast_cache_key(source), parse_result);
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &result,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &result,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(&result, options);
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
                    program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &result,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
                });
            }
        }

        debug!("Forced full parse for file: {:?}", file_path);
        maybe_inject_parse_snapshot_full_parse_delay_for_test();
        record_parse_snapshot_full_parse_attempt_for_test();
        match self.tree_sitter.parse_with_tree(&new_content) {
            Ok((tree, program)) => {
                let backend_tree = Arc::new(tree.clone());
                let program_lowering_summary = Self::summarize_program_lowering(
                    &program,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &result,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &program,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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

    #[allow(clippy::too_many_arguments)]
    fn finalize_parse_snapshot_report_with_options(
        &self,
        file_path: &Path,
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
        let program_lowering_summary = Self::summarize_program_lowering(&parse_result, options);
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
            file_path,
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
            program_lowering_summary,
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
                let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();
                let (result, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &old_tree,
                        &new_content,
                        cancellation_flag,
                        options,
                        &mut lowering_attribution,
                        None,
                    )?;
                let report_options = ParseSnapshotExecutionOptions {
                    lowering_reuse_attribution: Some(&lowering_attribution),
                    ..options
                };
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
                    report_options,
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
                let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();
                let (program, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &tree,
                        &new_content,
                        cancellation_flag,
                        options,
                        &mut lowering_attribution,
                        None,
                    )?;
                let tree_for_cache = tree.clone();
                let report_options = ParseSnapshotExecutionOptions {
                    lowering_reuse_attribution: Some(&lowering_attribution),
                    ..options
                };
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
                    report_options,
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

    pub fn prime_tree_cache_from_source_with_cancellation(
        &self,
        file_path: PathBuf,
        source: String,
        cancellation_flag: &AtomicBool,
    ) -> Result<(), String> {
        self.prime_tree_cache_from_source_with_cancellation_and_options(
            file_path,
            source,
            cancellation_flag,
            PrimeTreeCacheFromSourceOptions::default(),
        )
    }

    pub fn prime_tree_cache_from_source_with_cancellation_and_options(
        &self,
        file_path: PathBuf,
        source: String,
        cancellation_flag: &AtomicBool,
        options: PrimeTreeCacheFromSourceOptions<'_>,
    ) -> Result<(), String> {
        if cancellation_flag.load(Ordering::SeqCst) {
            return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
        }

        let content_hash = hash_content(&source);
        if self
            .tree_cache
            .get(&file_path)
            .is_some_and(|(_, cached_source, cached_hash)| {
                cached_hash == content_hash && cached_source == source
            })
        {
            return Ok(());
        }

        match self
            .tree_sitter
            .parse_tree_only_with_cancellation(&source, None, cancellation_flag)
        {
            Ok(tree) => {
                if cancellation_flag.load(Ordering::SeqCst) {
                    return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                }
                if exact_program_lowering_reuse_enabled() && !options.skip_optional_ast_priming() {
                    match TreeSitterAdapter::convert_tree_fast(&tree, &source) {
                        Ok(parse_result) => {
                            self.store_ast_memory(ast_cache_key(&source), &parse_result);
                        }
                        Err(error) => {
                            warn!(
                                "Tree-sitter AST priming failed during tree-cache prime: {}",
                                error
                            );
                        }
                    }
                }
                self.tree_cache.set(file_path, tree, source, content_hash);
                Ok(())
            }
            Err(error) => {
                if is_parse_cancelled_error(&error) {
                    Err(error)
                } else {
                    error!(
                        "Tree-sitter parsing failed during tree-cache prime: {}",
                        error
                    );
                    Err(format!("Tree-sitter parsing failed: {}", error))
                }
            }
        }
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &result,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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
                    let program_lowering_summary = Self::summarize_program_lowering(
                        &program,
                        ParseSnapshotExecutionOptions::default(),
                    );
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
                        program_lowering_summary,
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
                            let program_lowering_summary = Self::summarize_program_lowering(
                                &program,
                                ParseSnapshotExecutionOptions::default(),
                            );
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
                                program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &program,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &result,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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
                    let program_lowering_summary = Self::summarize_program_lowering(
                        &program,
                        ParseSnapshotExecutionOptions::default(),
                    );
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
                        program_lowering_summary,
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
                            let program_lowering_summary = Self::summarize_program_lowering(
                                &program,
                                ParseSnapshotExecutionOptions::default(),
                            );
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
                                program_lowering_summary,
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
                let program_lowering_summary = Self::summarize_program_lowering(
                    &program,
                    ParseSnapshotExecutionOptions::default(),
                );
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
                    program_lowering_summary,
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

    #[allow(clippy::too_many_arguments)]
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
                let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();
                let (result, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &old_tree,
                        &new_content,
                        cancellation_flag,
                        options,
                        &mut lowering_attribution,
                        None,
                    )?;
                let report_options = ParseSnapshotExecutionOptions {
                    lowering_reuse_attribution: Some(&lowering_attribution),
                    ..options
                };
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
                    report_options,
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
                        let mut lowering_attribution =
                            ParseSnapshotProgramLoweringAttribution::default();
                        let mut lowering_reuse_plan = self.build_exact_lowering_reuse_plan(
                            &old_source,
                            &new_tree,
                            &changed_ranges,
                            &mut lowering_attribution,
                        );
                        let lowering_reuse_summary = lowering_reuse_plan
                            .as_ref()
                            .map(Self::build_program_lowering_summary_plan);
                        let assembly_options = ParseSnapshotExecutionOptions {
                            lowering_reuse_summary: lowering_reuse_summary.as_ref(),
                            ..options
                        };
                        match self.run_exact_ready_snapshot_assembly_with_cancellation(
                            &new_tree,
                            &new_content,
                            cancellation_flag,
                            assembly_options,
                            &mut lowering_attribution,
                            lowering_reuse_plan.as_mut(),
                        ) {
                            Ok((program, deferred_syntax_error_assembly)) => {
                                let report_options = ParseSnapshotExecutionOptions {
                                    lowering_reuse_summary: lowering_reuse_summary.as_ref(),
                                    lowering_reuse_attribution: Some(&lowering_attribution),
                                    ..options
                                };
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
                                    report_options,
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
                    let mut lowering_attribution =
                        ParseSnapshotProgramLoweringAttribution::default();
                    let (program, deferred_syntax_error_assembly) = self
                        .run_exact_ready_snapshot_assembly_with_cancellation(
                            &tree,
                            &new_content,
                            cancellation_flag,
                            options,
                            &mut lowering_attribution,
                            None,
                        )?;
                    let report_options = ParseSnapshotExecutionOptions {
                        lowering_reuse_attribution: Some(&lowering_attribution),
                        ..options
                    };
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
                        report_options,
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
                let mut lowering_attribution = ParseSnapshotProgramLoweringAttribution::default();
                let (program, deferred_syntax_error_assembly) = self
                    .run_exact_ready_snapshot_assembly_with_cancellation(
                        &tree,
                        &new_content,
                        cancellation_flag,
                        options,
                        &mut lowering_attribution,
                        None,
                    )?;
                let report_options = ParseSnapshotExecutionOptions {
                    lowering_reuse_attribution: Some(&lowering_attribution),
                    ..options
                };
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
                    report_options,
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

    fn exact_ready_snapshot_control(
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> ParseSnapshotExactReadyControl {
        if cancellation_flag.load(Ordering::SeqCst) {
            return ParseSnapshotExactReadyControl::Cancel;
        }

        if let Some(callback) = options.exact_ready_snapshot_control_callback {
            match callback() {
                ParseSnapshotExactReadyControl::Cancel => {
                    return ParseSnapshotExactReadyControl::Cancel;
                }
                ParseSnapshotExactReadyControl::SaveCritical => {
                    return ParseSnapshotExactReadyControl::SaveCritical;
                }
                ParseSnapshotExactReadyControl::Continue => {}
            }
        }

        if Self::save_critical_requested(options) {
            ParseSnapshotExactReadyControl::SaveCritical
        } else {
            ParseSnapshotExactReadyControl::Continue
        }
    }

    fn build_exact_lowering_reuse_plan(
        &self,
        old_source: &str,
        new_tree: &tree_sitter::Tree,
        changed_ranges: &[ParseChangedRange],
        attribution: &mut ParseSnapshotProgramLoweringAttribution,
    ) -> Option<LoweringReusePlan> {
        if !exact_program_lowering_reuse_enabled() {
            return None;
        }
        let cache_key = ast_cache_key(old_source);
        attribution.reuse_plan_take_if_unique_hit = Some(false);
        attribution.reuse_plan_borrowed_cache_hit = Some(false);
        if let Some(previous_parse_result) = self.ast_cache.take_if_unique(cache_key) {
            attribution.reuse_plan_take_if_unique_hit = Some(true);
            let started = std::time::Instant::now();
            match Self::derive_exact_lowering_reuse_plan_owned(
                previous_parse_result,
                new_tree,
                changed_ranges,
                attribution,
            ) {
                Ok(plan) => {
                    let elapsed_ms = duration_to_u64_ms(started.elapsed());
                    attribution.reuse_plan_build_source =
                        Some(ParseSnapshotProgramLoweringReusePlanBuildSource::Owned);
                    attribution.reuse_plan_build_ms = Some(elapsed_ms);
                    attribution.reuse_plan_owned_build_ms = Some(elapsed_ms);
                    return Some(plan);
                }
                Err(previous_parse_result) => {
                    self.ast_cache
                        .put(cache_key, Arc::new(previous_parse_result));
                }
            }
        }
        let previous_parse_result = self.ast_cache.get(cache_key)?;
        attribution.reuse_plan_borrowed_cache_hit = Some(true);
        let started = std::time::Instant::now();
        let plan = Self::derive_exact_lowering_reuse_plan(
            previous_parse_result.as_ref(),
            new_tree,
            changed_ranges,
            attribution,
        )?;
        let elapsed_ms = duration_to_u64_ms(started.elapsed());
        attribution.reuse_plan_build_source =
            Some(ParseSnapshotProgramLoweringReusePlanBuildSource::Borrowed);
        attribution.reuse_plan_build_ms = Some(elapsed_ms);
        attribution.reuse_plan_borrowed_build_ms = Some(elapsed_ms);
        Some(plan)
    }

    fn derive_exact_lowering_reuse_plan(
        previous_parse_result: &ParseResult,
        new_tree: &tree_sitter::Tree,
        changed_ranges: &[ParseChangedRange],
        attribution: &mut ParseSnapshotProgramLoweringAttribution,
    ) -> Option<LoweringReusePlan> {
        let previous_top_level = previous_parse_result.program.statements.as_slice();
        if previous_top_level.is_empty() || changed_ranges.is_empty() {
            return None;
        }

        let new_root = new_tree.root_node();
        let new_top_level_nodes = Self::collect_direct_lowering_children(&new_root);
        if new_top_level_nodes.len() != previous_top_level.len() {
            return None;
        }

        let affected_top_level =
            Self::derive_affected_statement_mask(previous_top_level, changed_ranges)?;
        let affected_indices: Vec<_> = affected_top_level
            .iter()
            .enumerate()
            .filter_map(|(idx, affected)| affected.then_some(idx))
            .collect();
        let routine_reuse = if affected_indices.len() == 1 {
            let affected_idx = affected_indices[0];
            Self::derive_routine_body_reuse_decision(
                &previous_top_level[affected_idx],
                &new_top_level_nodes[affected_idx],
                changed_ranges,
            )
            .map(|decision| (affected_idx, decision))
        } else {
            None
        };
        let has_unaffected_top_level = affected_top_level.iter().any(|affected| !*affected);
        if !has_unaffected_top_level && routine_reuse.is_none() {
            return None;
        }

        let mut top_level_nodes = Vec::with_capacity(previous_top_level.len());
        let mut rebase_elapsed = Duration::ZERO;
        let mut rebase_statement_count = 0u64;
        for (idx, statement) in previous_top_level.iter().enumerate() {
            if affected_top_level[idx] {
                top_level_nodes.push(LoweringReuseNodePlan::Rebuild);
            } else {
                let started = std::time::Instant::now();
                top_level_nodes.push(LoweringReuseNodePlan::ReuseStatement(
                    Self::rebase_statement(statement, changed_ranges),
                ));
                rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                rebase_statement_count = rebase_statement_count.saturating_add(1);
            }
        }

        let mut outcome = if has_unaffected_top_level {
            LoweringReusePlanOutcome::TopLevelReuse
        } else {
            LoweringReusePlanOutcome::FullRebuild
        };

        if let Some((affected_idx, decision)) = routine_reuse {
            let routine_plan = Self::derive_routine_body_lowering_reuse_plan(
                &previous_top_level[affected_idx],
                decision,
                changed_ranges,
                &mut rebase_elapsed,
                &mut rebase_statement_count,
            );
            outcome = match decision {
                RoutineBodyReuseDecision::ReuseWholeStatement => {
                    LoweringReusePlanOutcome::TopLevelReuse
                }
                RoutineBodyReuseDecision::ReuseWindow(_) => {
                    LoweringReusePlanOutcome::RoutineBodyReuse
                }
            };
            if let Some(routine_plan) = routine_plan {
                top_level_nodes[affected_idx] = routine_plan;
            }
        }

        attribution.reuse_plan_rebase_ms = Some(duration_to_u64_ms(rebase_elapsed));
        attribution.reuse_plan_rebase_statement_count = Some(rebase_statement_count);

        Some(LoweringReusePlan {
            outcome,
            top_level_nodes,
        })
    }

    fn derive_exact_lowering_reuse_plan_owned(
        previous_parse_result: ParseResult,
        new_tree: &tree_sitter::Tree,
        changed_ranges: &[ParseChangedRange],
        attribution: &mut ParseSnapshotProgramLoweringAttribution,
    ) -> Result<LoweringReusePlan, ParseResult> {
        let previous_top_level = previous_parse_result.program.statements.as_slice();
        if previous_top_level.is_empty() || changed_ranges.is_empty() {
            return Err(previous_parse_result);
        }

        let new_root = new_tree.root_node();
        let new_top_level_nodes = Self::collect_direct_lowering_children(&new_root);
        if new_top_level_nodes.len() != previous_top_level.len() {
            return Err(previous_parse_result);
        }

        let affected_top_level =
            match Self::derive_affected_statement_mask(previous_top_level, changed_ranges) {
                Some(mask) => mask,
                None => return Err(previous_parse_result),
            };
        let affected_indices: Vec<_> = affected_top_level
            .iter()
            .enumerate()
            .filter_map(|(idx, affected)| affected.then_some(idx))
            .collect();
        let routine_reuse = if affected_indices.len() == 1 {
            let affected_idx = affected_indices[0];
            Self::derive_routine_body_reuse_decision(
                &previous_top_level[affected_idx],
                &new_top_level_nodes[affected_idx],
                changed_ranges,
            )
            .map(|decision| (affected_idx, decision))
        } else {
            None
        };
        let has_unaffected_top_level = affected_top_level.iter().any(|affected| !*affected);
        if !has_unaffected_top_level && routine_reuse.is_none() {
            return Err(previous_parse_result);
        }

        let mut outcome = if has_unaffected_top_level {
            LoweringReusePlanOutcome::TopLevelReuse
        } else {
            LoweringReusePlanOutcome::FullRebuild
        };
        if let Some((_, decision)) = routine_reuse {
            outcome = match decision {
                RoutineBodyReuseDecision::ReuseWholeStatement => {
                    LoweringReusePlanOutcome::TopLevelReuse
                }
                RoutineBodyReuseDecision::ReuseWindow(_) => {
                    LoweringReusePlanOutcome::RoutineBodyReuse
                }
            };
        }

        let mut rebase_elapsed = Duration::ZERO;
        let mut rebase_statement_count = 0u64;
        let top_level_nodes = previous_parse_result
            .program
            .statements
            .into_iter()
            .enumerate()
            .map(|(idx, mut statement)| {
                if !affected_top_level[idx] {
                    let started = std::time::Instant::now();
                    Self::rebase_statement_in_place(&mut statement, changed_ranges);
                    rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                    rebase_statement_count = rebase_statement_count.saturating_add(1);
                    LoweringReuseNodePlan::ReuseStatement(statement)
                } else if let Some((affected_idx, decision)) =
                    routine_reuse.filter(|(affected_idx, _)| *affected_idx == idx)
                {
                    let _ = affected_idx;
                    Self::build_owned_routine_body_lowering_reuse_plan(
                        statement,
                        decision,
                        changed_ranges,
                        &mut rebase_elapsed,
                        &mut rebase_statement_count,
                    )
                } else {
                    LoweringReuseNodePlan::Rebuild
                }
            })
            .collect();

        attribution.reuse_plan_rebase_ms = Some(duration_to_u64_ms(rebase_elapsed));
        attribution.reuse_plan_rebase_statement_count = Some(rebase_statement_count);

        Ok(LoweringReusePlan {
            outcome,
            top_level_nodes,
        })
    }

    fn derive_affected_statement_mask(
        statements: &[Statement],
        changed_ranges: &[ParseChangedRange],
    ) -> Option<Vec<bool>> {
        let mut affected = vec![false; statements.len()];
        for changed_range in changed_ranges {
            let mut matched = false;
            for (idx, statement) in statements.iter().enumerate() {
                if Self::changed_range_touches_statement(changed_range, statement) {
                    affected[idx] = true;
                    matched = true;
                }
            }
            if !matched {
                return None;
            }
        }
        Some(affected)
    }

    fn derive_routine_body_lowering_reuse_plan(
        previous_statement: &Statement,
        decision: RoutineBodyReuseDecision,
        changed_ranges: &[ParseChangedRange],
        rebase_elapsed: &mut Duration,
        rebase_statement_count: &mut u64,
    ) -> Option<LoweringReuseNodePlan> {
        match decision {
            RoutineBodyReuseDecision::ReuseWholeStatement => {
                let started = std::time::Instant::now();
                let statement = Self::rebase_statement(previous_statement, changed_ranges);
                *rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                *rebase_statement_count = rebase_statement_count.saturating_add(1);
                Some(LoweringReuseNodePlan::ReuseStatement(statement))
            }
            RoutineBodyReuseDecision::ReuseWindow(window) => {
                let body = match previous_statement {
                    Statement::FunctionDecl { body, .. }
                    | Statement::ProcedureDecl { body, .. } => body.as_slice(),
                    _ => return None,
                };
                Some(LoweringReuseNodePlan::RebuildRoutineBody(
                    RoutineBodyLoweringReusePlan {
                        original_body_len: window.original_body_len,
                        reused_prefix_len: window.reused_prefix_len,
                        reused_suffix_start: window.reused_suffix_start,
                        reused_body_prefix: body[..window.reused_prefix_len]
                            .iter()
                            .map(|statement| {
                                let started = std::time::Instant::now();
                                let rebased = Self::rebase_statement(statement, changed_ranges);
                                *rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                                *rebase_statement_count = rebase_statement_count.saturating_add(1);
                                rebased
                            })
                            .collect(),
                        reused_body_suffix: body[window.reused_suffix_start..]
                            .iter()
                            .map(|statement| {
                                let started = std::time::Instant::now();
                                let rebased = Self::rebase_statement(statement, changed_ranges);
                                *rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                                *rebase_statement_count = rebase_statement_count.saturating_add(1);
                                rebased
                            })
                            .collect(),
                    },
                ))
            }
        }
    }

    fn derive_routine_body_reuse_decision(
        previous_statement: &Statement,
        new_routine_node: &Node<'_>,
        changed_ranges: &[ParseChangedRange],
    ) -> Option<RoutineBodyReuseDecision> {
        let (body, expected_kind) = match previous_statement {
            Statement::FunctionDecl { body, .. } => (body.as_slice(), "function_definition"),
            Statement::ProcedureDecl { body, .. } => (body.as_slice(), "procedure_definition"),
            _ => return None,
        };

        if body.is_empty() || new_routine_node.kind() != expected_kind {
            return None;
        }

        let new_body_nodes = Self::collect_direct_lowering_children(new_routine_node);
        if new_body_nodes.len() != body.len() {
            return None;
        }

        let first_body_span = Self::statement_span(&body[0]);
        let last_body_span = Self::statement_span(body.last()?);
        let mut affected_body = vec![false; body.len()];
        let mut any_body_statement_affected = false;

        for changed_range in changed_ranges {
            if !Self::changed_range_within_bounds(
                changed_range,
                first_body_span.start,
                last_body_span.end,
            ) {
                return None;
            }

            let mut matched = false;
            for (idx, statement) in body.iter().enumerate() {
                if Self::changed_range_touches_statement(changed_range, statement) {
                    affected_body[idx] = true;
                    matched = true;
                    any_body_statement_affected = true;
                }
            }

            if !matched && changed_range.old_end_byte > changed_range.start_byte {
                // A replacement entirely between lowered siblings is too ambiguous.
                return None;
            }
        }

        if !any_body_statement_affected {
            return Some(RoutineBodyReuseDecision::ReuseWholeStatement);
        }

        let first_affected = affected_body.iter().position(|affected| *affected)?;
        let last_affected = affected_body.iter().rposition(|affected| *affected)?;
        if (first_affected..=last_affected)
            .any(|idx| !Self::supports_routine_body_window_reuse(&body[idx]))
        {
            return None;
        }

        if first_affected == 0 && last_affected + 1 == body.len() {
            return None;
        }

        Some(RoutineBodyReuseDecision::ReuseWindow(
            RoutineBodyReuseWindow {
                original_body_len: body.len(),
                reused_prefix_len: first_affected,
                reused_suffix_start: last_affected + 1,
            },
        ))
    }

    fn build_owned_routine_body_lowering_reuse_plan(
        mut previous_statement: Statement,
        decision: RoutineBodyReuseDecision,
        changed_ranges: &[ParseChangedRange],
        rebase_elapsed: &mut Duration,
        rebase_statement_count: &mut u64,
    ) -> LoweringReuseNodePlan {
        match decision {
            RoutineBodyReuseDecision::ReuseWholeStatement => {
                let started = std::time::Instant::now();
                Self::rebase_statement_in_place(&mut previous_statement, changed_ranges);
                *rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                *rebase_statement_count = rebase_statement_count.saturating_add(1);
                LoweringReuseNodePlan::ReuseStatement(previous_statement)
            }
            RoutineBodyReuseDecision::ReuseWindow(window) => {
                let body = match &mut previous_statement {
                    Statement::FunctionDecl { body, .. }
                    | Statement::ProcedureDecl { body, .. } => body,
                    _ => unreachable!(
                        "owned routine-body reuse must only receive routine statements"
                    ),
                };
                let mut reused_body_suffix = body.split_off(window.reused_suffix_start);
                let _reused_gap = body.split_off(window.reused_prefix_len);
                let reused_body_prefix = std::mem::take(body);
                let mut reused_body_prefix: VecDeque<_> = reused_body_prefix.into();
                let mut reused_body_suffix: VecDeque<_> = reused_body_suffix.drain(..).collect();
                for statement in reused_body_prefix.iter_mut() {
                    let started = std::time::Instant::now();
                    Self::rebase_statement_in_place(statement, changed_ranges);
                    *rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                    *rebase_statement_count = rebase_statement_count.saturating_add(1);
                }
                for statement in reused_body_suffix.iter_mut() {
                    let started = std::time::Instant::now();
                    Self::rebase_statement_in_place(statement, changed_ranges);
                    *rebase_elapsed = rebase_elapsed.saturating_add(started.elapsed());
                    *rebase_statement_count = rebase_statement_count.saturating_add(1);
                }
                LoweringReuseNodePlan::RebuildRoutineBody(RoutineBodyLoweringReusePlan {
                    original_body_len: window.original_body_len,
                    reused_prefix_len: window.reused_prefix_len,
                    reused_suffix_start: window.reused_suffix_start,
                    reused_body_prefix,
                    reused_body_suffix,
                })
            }
        }
    }

    fn collect_direct_lowering_children<'a>(node: &'a Node<'a>) -> Vec<Node<'a>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| Self::is_lowering_progress_unit_kind(child.kind()))
            .collect()
    }

    fn is_lowering_progress_unit_kind(kind: &str) -> bool {
        matches!(
            kind,
            "function_definition"
                | "procedure_definition"
                | "var_definition"
                | "var_statement"
                | "if_statement"
                | "for_statement"
                | "for_each_statement"
                | "while_statement"
                | "try_statement"
                | "rise_error_statement"
                | "assignment_statement"
                | "return_statement"
                | "call_statement"
                | "break_statement"
                | "continue_statement"
                | "goto_statement"
                | "label_statement"
                | "execute_statement"
                | "add_handler_statement"
                | "remove_handler_statement"
                | "await_statement"
        )
    }

    fn statement_span(statement: &Statement) -> bsl_shared::ir::Span {
        match statement {
            Statement::Assignment { span, .. }
            | Statement::VarDeclaration { span, .. }
            | Statement::FunctionDecl { span, .. }
            | Statement::ProcedureDecl { span, .. }
            | Statement::If { span, .. }
            | Statement::For { span, .. }
            | Statement::ForEach { span, .. }
            | Statement::While { span, .. }
            | Statement::Return { span, .. }
            | Statement::Try { span, .. }
            | Statement::Call { span, .. }
            | Statement::Break { span, .. }
            | Statement::Continue { span, .. }
            | Statement::Goto { span, .. }
            | Statement::Label { span, .. }
            | Statement::Execute { span, .. }
            | Statement::RaiseError { span, .. }
            | Statement::AddHandler { span, .. }
            | Statement::RemoveHandler { span, .. }
            | Statement::Await { span, .. } => *span,
        }
    }

    fn supports_routine_body_window_reuse(statement: &Statement) -> bool {
        matches!(
            statement,
            Statement::Assignment { .. }
                | Statement::If { .. }
                | Statement::For { .. }
                | Statement::ForEach { .. }
                | Statement::While { .. }
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
                | Statement::Await { .. }
        )
    }

    fn changed_range_touches_statement(
        changed_range: &ParseChangedRange,
        statement: &Statement,
    ) -> bool {
        Self::changed_range_touches_span(changed_range, Self::statement_span(statement))
    }

    fn changed_range_touches_span(
        changed_range: &ParseChangedRange,
        span: bsl_shared::ir::Span,
    ) -> bool {
        let start = changed_range.start_byte;
        let old_end = changed_range.old_end_byte;
        if old_end > start {
            start < span.end && old_end > span.start
        } else {
            span.start <= start && start <= span.end
        }
    }

    fn changed_range_within_bounds(
        changed_range: &ParseChangedRange,
        start_bound: u32,
        end_bound: u32,
    ) -> bool {
        let start = changed_range.start_byte;
        let old_end = changed_range.old_end_byte.max(start);
        start_bound <= start && old_end <= end_bound
    }

    fn summarize_program_lowering(
        parse_result: &ParseResult,
        options: ParseSnapshotExecutionOptions<'_>,
    ) -> ParseSnapshotProgramLoweringSummary {
        let total_lowering_units =
            Self::count_lowering_units_in_statements(&parse_result.program.statements);
        if let Some(lowering_reuse_summary) = options.lowering_reuse_summary {
            return Self::apply_program_lowering_attribution(
                Self::summarize_program_lowering_summary_plan(
                    lowering_reuse_summary,
                    &parse_result.program.statements,
                    total_lowering_units,
                ),
                options.lowering_reuse_attribution,
            );
        }
        if let Some(lowering_reuse_plan) = options.lowering_reuse_plan {
            return Self::apply_program_lowering_attribution(
                Self::summarize_program_lowering_reuse_plan(
                    lowering_reuse_plan,
                    &parse_result.program.statements,
                    total_lowering_units,
                ),
                options.lowering_reuse_attribution,
            );
        }
        if let Some(reused_program_prefix) = options
            .reused_program_prefix
            .filter(|prefix| !prefix.is_empty())
        {
            let reused_lowering_units =
                Self::count_lowering_units_in_statements(reused_program_prefix);
            let rebuilt_lowering_units = total_lowering_units.saturating_sub(reused_lowering_units);
            let reused_top_level_node_count = reused_program_prefix.len() as u64;
            let rebuilt_top_level_node_count = parse_result
                .program
                .statements
                .len()
                .saturating_sub(reused_program_prefix.len())
                as u64;
            return Self::apply_program_lowering_attribution(
                ParseSnapshotProgramLoweringSummary {
                    reuse_outcome: ParseSnapshotProgramLoweringReuseOutcome::ReusedPrefix,
                    reused_lowering_units,
                    rebuilt_lowering_units,
                    reused_window_count: 1,
                    rebuilt_window_count: u64::from(rebuilt_lowering_units > 0),
                    largest_rebuilt_window_lowering_units: rebuilt_lowering_units,
                    fully_reused_top_level_node_count: reused_top_level_node_count,
                    fully_rebuilt_top_level_node_count: rebuilt_top_level_node_count,
                    routine_body_reuse_node_count: 0,
                    fully_reused_top_level_lowering_units: reused_lowering_units,
                    fully_rebuilt_top_level_lowering_units: rebuilt_lowering_units,
                    routine_body_reused_prefix_lowering_units: 0,
                    routine_body_reused_suffix_lowering_units: 0,
                    routine_body_rebuilt_lowering_units: 0,
                    reuse_plan_build_source: None,
                    reuse_plan_take_if_unique_hit: None,
                    reuse_plan_borrowed_cache_hit: None,
                    reuse_plan_build_ms: None,
                    reuse_plan_owned_build_ms: None,
                    reuse_plan_borrowed_build_ms: None,
                    reuse_plan_rebase_ms: None,
                    reuse_plan_rebase_statement_count: None,
                    reused_progress_ms: None,
                    reused_progress_call_count: None,
                    rebuild_dispatch_ms: None,
                    rebuild_dispatch_call_count: None,
                    rebuild_dispatch_callable_ms: None,
                    rebuild_dispatch_callable_call_count: None,
                    rebuild_dispatch_callable_body_dispatch_ms: None,
                    rebuild_dispatch_callable_body_dispatch_call_count: None,
                    rebuild_dispatch_callable_non_body_dispatch_ms: None,
                    rebuild_dispatch_control_flow_ms: None,
                    rebuild_dispatch_control_flow_call_count: None,
                    rebuild_dispatch_simple_ms: None,
                    rebuild_dispatch_simple_call_count: None,
                    rebuild_dispatch_other_ms: None,
                    rebuild_dispatch_other_call_count: None,
                },
                options.lowering_reuse_attribution,
            );
        }
        Self::apply_program_lowering_attribution(
            ParseSnapshotProgramLoweringSummary {
                reuse_outcome: ParseSnapshotProgramLoweringReuseOutcome::FullRebuild,
                reused_lowering_units: 0,
                rebuilt_lowering_units: total_lowering_units,
                reused_window_count: 0,
                rebuilt_window_count: u64::from(total_lowering_units > 0),
                largest_rebuilt_window_lowering_units: total_lowering_units,
                fully_reused_top_level_node_count: 0,
                fully_rebuilt_top_level_node_count: parse_result.program.statements.len() as u64,
                routine_body_reuse_node_count: 0,
                fully_reused_top_level_lowering_units: 0,
                fully_rebuilt_top_level_lowering_units: total_lowering_units,
                routine_body_reused_prefix_lowering_units: 0,
                routine_body_reused_suffix_lowering_units: 0,
                routine_body_rebuilt_lowering_units: 0,
                reuse_plan_build_source: None,
                reuse_plan_take_if_unique_hit: None,
                reuse_plan_borrowed_cache_hit: None,
                reuse_plan_build_ms: None,
                reuse_plan_owned_build_ms: None,
                reuse_plan_borrowed_build_ms: None,
                reuse_plan_rebase_ms: None,
                reuse_plan_rebase_statement_count: None,
                reused_progress_ms: None,
                reused_progress_call_count: None,
                rebuild_dispatch_ms: None,
                rebuild_dispatch_call_count: None,
                rebuild_dispatch_callable_ms: None,
                rebuild_dispatch_callable_call_count: None,
                rebuild_dispatch_callable_body_dispatch_ms: None,
                rebuild_dispatch_callable_body_dispatch_call_count: None,
                rebuild_dispatch_callable_non_body_dispatch_ms: None,
                rebuild_dispatch_control_flow_ms: None,
                rebuild_dispatch_control_flow_call_count: None,
                rebuild_dispatch_simple_ms: None,
                rebuild_dispatch_simple_call_count: None,
                rebuild_dispatch_other_ms: None,
                rebuild_dispatch_other_call_count: None,
            },
            options.lowering_reuse_attribution,
        )
    }

    fn apply_program_lowering_attribution(
        mut summary: ParseSnapshotProgramLoweringSummary,
        attribution: Option<&ParseSnapshotProgramLoweringAttribution>,
    ) -> ParseSnapshotProgramLoweringSummary {
        let Some(attribution) = attribution else {
            return summary;
        };
        summary.reuse_plan_build_source = attribution.reuse_plan_build_source;
        summary.reuse_plan_take_if_unique_hit = attribution.reuse_plan_take_if_unique_hit;
        summary.reuse_plan_borrowed_cache_hit = attribution.reuse_plan_borrowed_cache_hit;
        summary.reuse_plan_build_ms = attribution.reuse_plan_build_ms;
        summary.reuse_plan_owned_build_ms = attribution.reuse_plan_owned_build_ms;
        summary.reuse_plan_borrowed_build_ms = attribution.reuse_plan_borrowed_build_ms;
        summary.reuse_plan_rebase_ms = attribution.reuse_plan_rebase_ms;
        summary.reuse_plan_rebase_statement_count = attribution.reuse_plan_rebase_statement_count;
        summary.reused_progress_ms = attribution.reused_progress_ms;
        summary.reused_progress_call_count = attribution.reused_progress_call_count;
        summary.rebuild_dispatch_ms = attribution.rebuild_dispatch_ms;
        summary.rebuild_dispatch_call_count = attribution.rebuild_dispatch_call_count;
        summary.rebuild_dispatch_callable_ms = attribution.rebuild_dispatch_callable_ms;
        summary.rebuild_dispatch_callable_call_count =
            attribution.rebuild_dispatch_callable_call_count;
        summary.rebuild_dispatch_callable_body_dispatch_ms =
            attribution.rebuild_dispatch_callable_body_dispatch_ms;
        summary.rebuild_dispatch_callable_body_dispatch_call_count =
            attribution.rebuild_dispatch_callable_body_dispatch_call_count;
        summary.rebuild_dispatch_callable_non_body_dispatch_ms =
            attribution.rebuild_dispatch_callable_non_body_dispatch_ms;
        summary.rebuild_dispatch_control_flow_ms = attribution.rebuild_dispatch_control_flow_ms;
        summary.rebuild_dispatch_control_flow_call_count =
            attribution.rebuild_dispatch_control_flow_call_count;
        summary.rebuild_dispatch_simple_ms = attribution.rebuild_dispatch_simple_ms;
        summary.rebuild_dispatch_simple_call_count = attribution.rebuild_dispatch_simple_call_count;
        summary.rebuild_dispatch_other_ms = attribution.rebuild_dispatch_other_ms;
        summary.rebuild_dispatch_other_call_count = attribution.rebuild_dispatch_other_call_count;
        summary
    }

    fn build_program_lowering_summary_plan(
        lowering_reuse_plan: &LoweringReusePlan,
    ) -> ParseSnapshotProgramLoweringSummaryPlan {
        let nodes = lowering_reuse_plan
            .top_level_nodes
            .iter()
            .map(|node_plan| match node_plan {
                LoweringReuseNodePlan::ReuseStatement(statement) => {
                    ParseSnapshotProgramLoweringSummaryNode::ReuseStatement {
                        reused_lowering_units: Self::count_lowering_units(statement),
                    }
                }
                LoweringReuseNodePlan::Rebuild => ParseSnapshotProgramLoweringSummaryNode::Rebuild,
                LoweringReuseNodePlan::RebuildRoutineBody(body_reuse) => {
                    ParseSnapshotProgramLoweringSummaryNode::RebuildRoutineBody {
                        reused_prefix_lowering_units: Self::count_lowering_units_in_iter(
                            body_reuse.reused_body_prefix.iter(),
                        ),
                        reused_suffix_lowering_units: Self::count_lowering_units_in_iter(
                            body_reuse.reused_body_suffix.iter(),
                        ),
                    }
                }
            })
            .collect();

        ParseSnapshotProgramLoweringSummaryPlan {
            reuse_outcome: match lowering_reuse_plan.outcome {
                LoweringReusePlanOutcome::FullRebuild => {
                    ParseSnapshotProgramLoweringReuseOutcome::FullRebuild
                }
                LoweringReusePlanOutcome::TopLevelReuse => {
                    ParseSnapshotProgramLoweringReuseOutcome::TopLevelReuse
                }
                LoweringReusePlanOutcome::RoutineBodyReuse => {
                    ParseSnapshotProgramLoweringReuseOutcome::RoutineBodyReuse
                }
            },
            nodes,
        }
    }

    fn summarize_program_lowering_summary_plan(
        lowering_reuse_summary: &ParseSnapshotProgramLoweringSummaryPlan,
        final_statements: &[Statement],
        total_lowering_units: u64,
    ) -> ParseSnapshotProgramLoweringSummary {
        let mut reused_lowering_units = 0u64;
        let mut rebuilt_lowering_units = 0u64;
        let mut reused_window_count = 0u64;
        let mut rebuilt_window_count = 0u64;
        let mut largest_rebuilt_window_lowering_units = 0u64;
        let mut fully_reused_top_level_node_count = 0u64;
        let mut fully_rebuilt_top_level_node_count = 0u64;
        let mut routine_body_reuse_node_count = 0u64;
        let mut fully_reused_top_level_lowering_units = 0u64;
        let mut fully_rebuilt_top_level_lowering_units = 0u64;
        let mut routine_body_reused_prefix_lowering_units = 0u64;
        let mut routine_body_reused_suffix_lowering_units = 0u64;
        let mut routine_body_rebuilt_lowering_units = 0u64;
        let mut previous_top_level_reused = false;
        let mut previous_top_level_rebuilt = false;

        for (idx, node_plan) in lowering_reuse_summary.nodes.iter().enumerate() {
            match node_plan {
                ParseSnapshotProgramLoweringSummaryNode::ReuseStatement {
                    reused_lowering_units: reused_units,
                } => {
                    reused_lowering_units = reused_lowering_units.saturating_add(*reused_units);
                    fully_reused_top_level_lowering_units =
                        fully_reused_top_level_lowering_units.saturating_add(*reused_units);
                    fully_reused_top_level_node_count =
                        fully_reused_top_level_node_count.saturating_add(1);
                    if !previous_top_level_reused {
                        reused_window_count = reused_window_count.saturating_add(1);
                    }
                    previous_top_level_reused = true;
                    previous_top_level_rebuilt = false;
                }
                ParseSnapshotProgramLoweringSummaryNode::Rebuild => {
                    let rebuilt_window_units = final_statements
                        .get(idx)
                        .map(Self::count_lowering_units)
                        .unwrap_or(0);
                    rebuilt_lowering_units =
                        rebuilt_lowering_units.saturating_add(rebuilt_window_units);
                    fully_rebuilt_top_level_lowering_units =
                        fully_rebuilt_top_level_lowering_units.saturating_add(rebuilt_window_units);
                    fully_rebuilt_top_level_node_count =
                        fully_rebuilt_top_level_node_count.saturating_add(1);
                    if !previous_top_level_rebuilt {
                        rebuilt_window_count = rebuilt_window_count.saturating_add(1);
                    }
                    largest_rebuilt_window_lowering_units =
                        largest_rebuilt_window_lowering_units.max(rebuilt_window_units);
                    previous_top_level_reused = false;
                    previous_top_level_rebuilt = true;
                }
                ParseSnapshotProgramLoweringSummaryNode::RebuildRoutineBody {
                    reused_prefix_lowering_units,
                    reused_suffix_lowering_units,
                } => {
                    let reused_body_lowering_units = (*reused_prefix_lowering_units)
                        .saturating_add(*reused_suffix_lowering_units);
                    let rebuilt_window_units = final_statements
                        .get(idx)
                        .map(Self::count_lowering_units)
                        .unwrap_or(0)
                        .saturating_sub(reused_body_lowering_units);
                    reused_lowering_units =
                        reused_lowering_units.saturating_add(reused_body_lowering_units);
                    rebuilt_lowering_units =
                        rebuilt_lowering_units.saturating_add(rebuilt_window_units);
                    routine_body_reuse_node_count = routine_body_reuse_node_count.saturating_add(1);
                    routine_body_reused_prefix_lowering_units =
                        routine_body_reused_prefix_lowering_units
                            .saturating_add(*reused_prefix_lowering_units);
                    routine_body_reused_suffix_lowering_units =
                        routine_body_reused_suffix_lowering_units
                            .saturating_add(*reused_suffix_lowering_units);
                    routine_body_rebuilt_lowering_units =
                        routine_body_rebuilt_lowering_units.saturating_add(rebuilt_window_units);
                    if *reused_prefix_lowering_units > 0 {
                        reused_window_count = reused_window_count.saturating_add(1);
                    }
                    if *reused_suffix_lowering_units > 0 {
                        reused_window_count = reused_window_count.saturating_add(1);
                    }
                    if !previous_top_level_rebuilt {
                        rebuilt_window_count = rebuilt_window_count.saturating_add(1);
                    }
                    largest_rebuilt_window_lowering_units =
                        largest_rebuilt_window_lowering_units.max(rebuilt_window_units);
                    previous_top_level_reused = false;
                    previous_top_level_rebuilt = true;
                }
            }
        }

        ParseSnapshotProgramLoweringSummary {
            reuse_outcome: lowering_reuse_summary.reuse_outcome,
            reused_lowering_units,
            rebuilt_lowering_units: if rebuilt_lowering_units == 0 {
                total_lowering_units.saturating_sub(reused_lowering_units)
            } else {
                rebuilt_lowering_units
            },
            reused_window_count,
            rebuilt_window_count,
            largest_rebuilt_window_lowering_units,
            fully_reused_top_level_node_count,
            fully_rebuilt_top_level_node_count,
            routine_body_reuse_node_count,
            fully_reused_top_level_lowering_units,
            fully_rebuilt_top_level_lowering_units,
            routine_body_reused_prefix_lowering_units,
            routine_body_reused_suffix_lowering_units,
            routine_body_rebuilt_lowering_units,
            reuse_plan_build_source: None,
            reuse_plan_take_if_unique_hit: None,
            reuse_plan_borrowed_cache_hit: None,
            reuse_plan_build_ms: None,
            reuse_plan_owned_build_ms: None,
            reuse_plan_borrowed_build_ms: None,
            reuse_plan_rebase_ms: None,
            reuse_plan_rebase_statement_count: None,
            reused_progress_ms: None,
            reused_progress_call_count: None,
            rebuild_dispatch_ms: None,
            rebuild_dispatch_call_count: None,
            rebuild_dispatch_callable_ms: None,
            rebuild_dispatch_callable_call_count: None,
            rebuild_dispatch_callable_body_dispatch_ms: None,
            rebuild_dispatch_callable_body_dispatch_call_count: None,
            rebuild_dispatch_callable_non_body_dispatch_ms: None,
            rebuild_dispatch_control_flow_ms: None,
            rebuild_dispatch_control_flow_call_count: None,
            rebuild_dispatch_simple_ms: None,
            rebuild_dispatch_simple_call_count: None,
            rebuild_dispatch_other_ms: None,
            rebuild_dispatch_other_call_count: None,
        }
    }

    fn summarize_program_lowering_reuse_plan(
        lowering_reuse_plan: &LoweringReusePlan,
        final_statements: &[Statement],
        total_lowering_units: u64,
    ) -> ParseSnapshotProgramLoweringSummary {
        let mut reused_lowering_units = 0u64;
        let mut rebuilt_lowering_units = 0u64;
        let mut reused_window_count = 0u64;
        let mut rebuilt_window_count = 0u64;
        let mut largest_rebuilt_window_lowering_units = 0u64;
        let mut fully_reused_top_level_node_count = 0u64;
        let mut fully_rebuilt_top_level_node_count = 0u64;
        let mut routine_body_reuse_node_count = 0u64;
        let mut fully_reused_top_level_lowering_units = 0u64;
        let mut fully_rebuilt_top_level_lowering_units = 0u64;
        let mut routine_body_reused_prefix_lowering_units = 0u64;
        let mut routine_body_reused_suffix_lowering_units = 0u64;
        let mut routine_body_rebuilt_lowering_units = 0u64;
        let mut previous_top_level_reused = false;
        let mut previous_top_level_rebuilt = false;

        for (idx, node_plan) in lowering_reuse_plan.top_level_nodes.iter().enumerate() {
            match node_plan {
                LoweringReuseNodePlan::ReuseStatement(statement) => {
                    let reused_units = Self::count_lowering_units(statement);
                    reused_lowering_units = reused_lowering_units.saturating_add(reused_units);
                    fully_reused_top_level_lowering_units =
                        fully_reused_top_level_lowering_units.saturating_add(reused_units);
                    fully_reused_top_level_node_count =
                        fully_reused_top_level_node_count.saturating_add(1);
                    if !previous_top_level_reused {
                        reused_window_count = reused_window_count.saturating_add(1);
                    }
                    previous_top_level_reused = true;
                    previous_top_level_rebuilt = false;
                }
                LoweringReuseNodePlan::Rebuild => {
                    let rebuilt_window_units = final_statements
                        .get(idx)
                        .map(Self::count_lowering_units)
                        .unwrap_or(0);
                    rebuilt_lowering_units =
                        rebuilt_lowering_units.saturating_add(rebuilt_window_units);
                    fully_rebuilt_top_level_lowering_units =
                        fully_rebuilt_top_level_lowering_units.saturating_add(rebuilt_window_units);
                    fully_rebuilt_top_level_node_count =
                        fully_rebuilt_top_level_node_count.saturating_add(1);
                    if !previous_top_level_rebuilt {
                        rebuilt_window_count = rebuilt_window_count.saturating_add(1);
                    }
                    largest_rebuilt_window_lowering_units =
                        largest_rebuilt_window_lowering_units.max(rebuilt_window_units);
                    previous_top_level_reused = false;
                    previous_top_level_rebuilt = true;
                }
                LoweringReuseNodePlan::RebuildRoutineBody(body_reuse) => {
                    let reused_prefix_lowering_units =
                        Self::count_lowering_units_in_iter(body_reuse.reused_body_prefix.iter());
                    let reused_suffix_lowering_units =
                        Self::count_lowering_units_in_iter(body_reuse.reused_body_suffix.iter());
                    let reused_body_lowering_units =
                        reused_prefix_lowering_units.saturating_add(reused_suffix_lowering_units);
                    let rebuilt_window_units = final_statements
                        .get(idx)
                        .map(Self::count_lowering_units)
                        .unwrap_or(0)
                        .saturating_sub(reused_body_lowering_units);
                    reused_lowering_units =
                        reused_lowering_units.saturating_add(reused_body_lowering_units);
                    rebuilt_lowering_units =
                        rebuilt_lowering_units.saturating_add(rebuilt_window_units);
                    routine_body_reuse_node_count = routine_body_reuse_node_count.saturating_add(1);
                    routine_body_reused_prefix_lowering_units =
                        routine_body_reused_prefix_lowering_units
                            .saturating_add(reused_prefix_lowering_units);
                    routine_body_reused_suffix_lowering_units =
                        routine_body_reused_suffix_lowering_units
                            .saturating_add(reused_suffix_lowering_units);
                    routine_body_rebuilt_lowering_units =
                        routine_body_rebuilt_lowering_units.saturating_add(rebuilt_window_units);
                    if !body_reuse.reused_body_prefix.is_empty() {
                        reused_window_count = reused_window_count.saturating_add(1);
                    }
                    if !body_reuse.reused_body_suffix.is_empty() {
                        reused_window_count = reused_window_count.saturating_add(1);
                    }
                    if !previous_top_level_rebuilt {
                        rebuilt_window_count = rebuilt_window_count.saturating_add(1);
                    }
                    largest_rebuilt_window_lowering_units =
                        largest_rebuilt_window_lowering_units.max(rebuilt_window_units);
                    previous_top_level_reused = false;
                    previous_top_level_rebuilt = true;
                }
            }
        }

        ParseSnapshotProgramLoweringSummary {
            reuse_outcome: match lowering_reuse_plan.outcome {
                LoweringReusePlanOutcome::FullRebuild => {
                    ParseSnapshotProgramLoweringReuseOutcome::FullRebuild
                }
                LoweringReusePlanOutcome::TopLevelReuse => {
                    ParseSnapshotProgramLoweringReuseOutcome::TopLevelReuse
                }
                LoweringReusePlanOutcome::RoutineBodyReuse => {
                    ParseSnapshotProgramLoweringReuseOutcome::RoutineBodyReuse
                }
            },
            reused_lowering_units,
            rebuilt_lowering_units: if rebuilt_lowering_units == 0 {
                total_lowering_units.saturating_sub(reused_lowering_units)
            } else {
                rebuilt_lowering_units
            },
            reused_window_count,
            rebuilt_window_count,
            largest_rebuilt_window_lowering_units,
            fully_reused_top_level_node_count,
            fully_rebuilt_top_level_node_count,
            routine_body_reuse_node_count,
            fully_reused_top_level_lowering_units,
            fully_rebuilt_top_level_lowering_units,
            routine_body_reused_prefix_lowering_units,
            routine_body_reused_suffix_lowering_units,
            routine_body_rebuilt_lowering_units,
            reuse_plan_build_source: None,
            reuse_plan_take_if_unique_hit: None,
            reuse_plan_borrowed_cache_hit: None,
            reuse_plan_build_ms: None,
            reuse_plan_owned_build_ms: None,
            reuse_plan_borrowed_build_ms: None,
            reuse_plan_rebase_ms: None,
            reuse_plan_rebase_statement_count: None,
            reused_progress_ms: None,
            reused_progress_call_count: None,
            rebuild_dispatch_ms: None,
            rebuild_dispatch_call_count: None,
            rebuild_dispatch_callable_ms: None,
            rebuild_dispatch_callable_call_count: None,
            rebuild_dispatch_callable_body_dispatch_ms: None,
            rebuild_dispatch_callable_body_dispatch_call_count: None,
            rebuild_dispatch_callable_non_body_dispatch_ms: None,
            rebuild_dispatch_control_flow_ms: None,
            rebuild_dispatch_control_flow_call_count: None,
            rebuild_dispatch_simple_ms: None,
            rebuild_dispatch_simple_call_count: None,
            rebuild_dispatch_other_ms: None,
            rebuild_dispatch_other_call_count: None,
        }
    }

    fn count_lowering_units_in_statements(statements: &[Statement]) -> u64 {
        Self::count_lowering_units_in_iter(statements.iter())
    }

    fn count_lowering_units_in_iter<'a>(statements: impl Iterator<Item = &'a Statement>) -> u64 {
        statements.map(Self::count_lowering_units).sum::<u64>()
    }

    fn count_lowering_units(statement: &Statement) -> u64 {
        let nested = match statement {
            Statement::FunctionDecl { body, .. } | Statement::ProcedureDecl { body, .. } => {
                Self::count_lowering_units_in_statements(body)
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => Self::count_lowering_units_in_statements(then_body).saturating_add(
                else_body
                    .as_ref()
                    .map(|body| Self::count_lowering_units_in_statements(body))
                    .unwrap_or(0),
            ),
            Statement::For { body, .. }
            | Statement::ForEach { body, .. }
            | Statement::While { body, .. } => Self::count_lowering_units_in_statements(body),
            Statement::Try {
                try_body,
                except_body,
                ..
            } => Self::count_lowering_units_in_statements(try_body)
                .saturating_add(Self::count_lowering_units_in_statements(except_body)),
            Statement::Assignment { .. }
            | Statement::VarDeclaration { .. }
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
            | Statement::Await { .. } => 0,
        };
        1u64.saturating_add(nested)
    }

    fn rebase_statement(statement: &Statement, changed_ranges: &[ParseChangedRange]) -> Statement {
        let mut statement = statement.clone();
        Self::rebase_statement_in_place(&mut statement, changed_ranges);
        statement
    }

    fn rebase_statement_in_place(statement: &mut Statement, changed_ranges: &[ParseChangedRange]) {
        match statement {
            Statement::Assignment {
                target,
                value,
                span,
            } => {
                Self::rebase_expression_in_place(target, changed_ranges);
                Self::rebase_expression_in_place(value, changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::VarDeclaration { span, .. }
            | Statement::Break { span }
            | Statement::Continue { span }
            | Statement::Goto { span, .. }
            | Statement::Label { span, .. } => {
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::FunctionDecl { body, span, .. }
            | Statement::ProcedureDecl { body, span, .. } => {
                for statement in body.iter_mut() {
                    Self::rebase_statement_in_place(statement, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
                span,
            } => {
                Self::rebase_expression_in_place(condition, changed_ranges);
                for statement in then_body.iter_mut() {
                    Self::rebase_statement_in_place(statement, changed_ranges);
                }
                if let Some(body) = else_body.as_mut() {
                    for statement in body.iter_mut() {
                        Self::rebase_statement_in_place(statement, changed_ranges);
                    }
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::For {
                start,
                end,
                body,
                span,
                ..
            } => {
                Self::rebase_expression_in_place(start, changed_ranges);
                Self::rebase_expression_in_place(end, changed_ranges);
                for statement in body.iter_mut() {
                    Self::rebase_statement_in_place(statement, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::ForEach {
                collection,
                body,
                span,
                ..
            } => {
                Self::rebase_expression_in_place(collection, changed_ranges);
                for statement in body.iter_mut() {
                    Self::rebase_statement_in_place(statement, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::While {
                condition,
                body,
                span,
            } => {
                Self::rebase_expression_in_place(condition, changed_ranges);
                for statement in body.iter_mut() {
                    Self::rebase_statement_in_place(statement, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::Return { value, span } => {
                if let Some(expression) = value.as_mut() {
                    Self::rebase_expression_in_place(expression, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::Try {
                try_body,
                except_body,
                span,
            } => {
                for statement in try_body.iter_mut() {
                    Self::rebase_statement_in_place(statement, changed_ranges);
                }
                for statement in except_body.iter_mut() {
                    Self::rebase_statement_in_place(statement, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::Call { expression, span }
            | Statement::Execute {
                code: expression,
                span,
            }
            | Statement::Await { expression, span } => {
                Self::rebase_expression_in_place(expression, changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::RaiseError { message, span } => {
                if let Some(expression) = message.as_mut() {
                    Self::rebase_expression_in_place(expression, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Statement::AddHandler {
                event,
                handler,
                span,
            }
            | Statement::RemoveHandler {
                event,
                handler,
                span,
            } => {
                Self::rebase_expression_in_place(event, changed_ranges);
                Self::rebase_expression_in_place(handler, changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
        }
    }

    fn rebase_expression_in_place(
        expression: &mut Expression,
        changed_ranges: &[ParseChangedRange],
    ) {
        match expression {
            Expression::Identifier { span, .. }
            | Expression::String { span, .. }
            | Expression::Number { span, .. }
            | Expression::Boolean { span, .. }
            | Expression::Date { span, .. } => {
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::Call {
                function,
                args,
                span,
            } => {
                Self::rebase_expression_in_place(function.as_mut(), changed_ranges);
                for expression in args.iter_mut() {
                    Self::rebase_expression_in_place(expression, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::Binary {
                left, right, span, ..
            } => {
                Self::rebase_expression_in_place(left.as_mut(), changed_ranges);
                Self::rebase_expression_in_place(right.as_mut(), changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::Unary { operand, span, .. } => {
                Self::rebase_expression_in_place(operand.as_mut(), changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
                span,
            } => {
                Self::rebase_expression_in_place(condition.as_mut(), changed_ranges);
                Self::rebase_expression_in_place(then_expr.as_mut(), changed_ranges);
                Self::rebase_expression_in_place(else_expr.as_mut(), changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::New { args, span, .. } => {
                for expression in args.iter_mut() {
                    Self::rebase_expression_in_place(expression, changed_ranges);
                }
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::PropertyAccess { object, span, .. } => {
                Self::rebase_expression_in_place(object.as_mut(), changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::IndexAccess {
                object,
                index,
                span,
            } => {
                Self::rebase_expression_in_place(object.as_mut(), changed_ranges);
                Self::rebase_expression_in_place(index.as_mut(), changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
            Expression::Await { expression, span } => {
                Self::rebase_expression_in_place(expression.as_mut(), changed_ranges);
                *span = Self::rebase_span(*span, changed_ranges);
            }
        }
    }

    fn rebase_span(
        span: bsl_shared::ir::Span,
        changed_ranges: &[ParseChangedRange],
    ) -> bsl_shared::ir::Span {
        bsl_shared::ir::Span::new(
            Self::rebase_offset(span.start, changed_ranges),
            Self::rebase_offset(span.end, changed_ranges),
        )
    }

    fn rebase_offset(old_offset: u32, changed_ranges: &[ParseChangedRange]) -> u32 {
        let rebased = changed_ranges
            .iter()
            .fold(i64::from(old_offset), |acc, range| {
                if old_offset >= range.old_end_byte {
                    acc + i64::from(range.new_end_byte) - i64::from(range.old_end_byte)
                } else {
                    acc
                }
            });
        rebased.max(0) as u32
    }

    fn run_exact_ready_snapshot_assembly_with_cancellation(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
        cancellation_flag: &AtomicBool,
        options: ParseSnapshotExecutionOptions<'_>,
        lowering_attribution: &mut ParseSnapshotProgramLoweringAttribution,
        lowering_reuse_plan: Option<&mut LoweringReusePlan>,
    ) -> Result<(ParseResult, bool), String> {
        Self::notify_parse_snapshot_core_build_checkpoint(
            &options,
            ParseSnapshotCoreBuildCheckpoint::ExactReadySnapshotAssembly,
        );
        Self::notify_parse_snapshot_assembly_checkpoint(
            &options,
            ParseSnapshotAssemblyCheckpoint::ProgramLowering,
        );
        let mut save_critical_during_lowering = options.save_critical_initial;
        let mut lowering_observer = |_, _| {
            maybe_inject_parse_snapshot_program_conversion_progress_delay_for_test();
            match Self::exact_ready_snapshot_control(cancellation_flag, options) {
                ParseSnapshotExactReadyControl::Continue => Ok(()),
                ParseSnapshotExactReadyControl::SaveCritical => {
                    save_critical_during_lowering = true;
                    Ok(())
                }
                ParseSnapshotExactReadyControl::Cancel => {
                    Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string())
                }
            }
        };
        let parse_result = if let Some(lowering_reuse_plan) = lowering_reuse_plan {
            let mut execution_attribution = LoweringExecutionAttribution::default();
            TreeSitterAdapter::convert_tree_fast_with_observer_and_reuse_plan(
                tree,
                content,
                lowering_reuse_plan,
                &mut execution_attribution,
                &mut lowering_observer,
            )
            .inspect(|_| {
                lowering_attribution.reused_progress_ms = Some(duration_to_u64_ms(
                    execution_attribution.reused_progress_elapsed,
                ));
                lowering_attribution.reused_progress_call_count =
                    Some(execution_attribution.reused_progress_call_count);
                lowering_attribution.rebuild_dispatch_ms = Some(duration_to_u64_ms(
                    execution_attribution.rebuild_dispatch_elapsed,
                ));
                lowering_attribution.rebuild_dispatch_call_count =
                    Some(execution_attribution.rebuild_dispatch_call_count);
                lowering_attribution.rebuild_dispatch_callable_ms = Some(duration_to_u64_ms(
                    execution_attribution.rebuild_dispatch_callable_elapsed,
                ));
                lowering_attribution.rebuild_dispatch_callable_call_count =
                    Some(execution_attribution.rebuild_dispatch_callable_call_count);
                lowering_attribution.rebuild_dispatch_callable_body_dispatch_ms =
                    Some(duration_to_u64_ms(
                        execution_attribution.rebuild_dispatch_callable_body_dispatch_elapsed,
                    ));
                lowering_attribution.rebuild_dispatch_callable_body_dispatch_call_count =
                    Some(execution_attribution.rebuild_dispatch_callable_body_dispatch_call_count);
                lowering_attribution.rebuild_dispatch_callable_non_body_dispatch_ms =
                    Some(duration_to_u64_ms(
                        execution_attribution.rebuild_dispatch_callable_non_body_dispatch_elapsed,
                    ));
                lowering_attribution.rebuild_dispatch_control_flow_ms = Some(duration_to_u64_ms(
                    execution_attribution.rebuild_dispatch_control_flow_elapsed,
                ));
                lowering_attribution.rebuild_dispatch_control_flow_call_count =
                    Some(execution_attribution.rebuild_dispatch_control_flow_call_count);
                lowering_attribution.rebuild_dispatch_simple_ms = Some(duration_to_u64_ms(
                    execution_attribution.rebuild_dispatch_simple_elapsed,
                ));
                lowering_attribution.rebuild_dispatch_simple_call_count =
                    Some(execution_attribution.rebuild_dispatch_simple_call_count);
                lowering_attribution.rebuild_dispatch_other_ms = Some(duration_to_u64_ms(
                    execution_attribution.rebuild_dispatch_other_elapsed,
                ));
                lowering_attribution.rebuild_dispatch_other_call_count =
                    Some(execution_attribution.rebuild_dispatch_other_call_count);
            })?
        } else if let Some(reused_program_prefix) = options
            .reused_program_prefix
            .filter(|prefix| !prefix.is_empty())
        {
            TreeSitterAdapter::convert_tree_fast_with_observer_and_reused_prefix(
                tree,
                content,
                reused_program_prefix,
                &mut lowering_observer,
            )?
        } else {
            TreeSitterAdapter::convert_tree_fast_with_observer(
                tree,
                content,
                &mut lowering_observer,
            )?
        };
        match Self::exact_ready_snapshot_control(cancellation_flag, options) {
            ParseSnapshotExactReadyControl::Continue => {}
            ParseSnapshotExactReadyControl::SaveCritical => {
                save_critical_during_lowering = true;
            }
            ParseSnapshotExactReadyControl::Cancel => {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
        }
        if save_critical_during_lowering {
            return Ok((parse_result, true));
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
                match Self::exact_ready_snapshot_control(cancellation_flag, options) {
                    ParseSnapshotExactReadyControl::Continue => {}
                    ParseSnapshotExactReadyControl::SaveCritical => {
                        return Ok((parse_result, true));
                    }
                    ParseSnapshotExactReadyControl::Cancel => {
                        return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        match Self::exact_ready_snapshot_control(cancellation_flag, options) {
            ParseSnapshotExactReadyControl::Continue => {}
            ParseSnapshotExactReadyControl::SaveCritical => {
                return Ok((parse_result, true));
            }
            ParseSnapshotExactReadyControl::Cancel => {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
        }

        Self::notify_parse_snapshot_assembly_checkpoint(
            &options,
            ParseSnapshotAssemblyCheckpoint::SyntaxErrorCollection,
        );
        if let Some(delay_ms) = maybe_inject_parse_snapshot_syntax_error_assembly_delay_for_test() {
            let deadline = std::time::Instant::now() + Duration::from_millis(delay_ms);
            while std::time::Instant::now() < deadline {
                match Self::exact_ready_snapshot_control(cancellation_flag, options) {
                    ParseSnapshotExactReadyControl::Continue => {}
                    ParseSnapshotExactReadyControl::SaveCritical => {
                        return Ok((parse_result, true));
                    }
                    ParseSnapshotExactReadyControl::Cancel => {
                        return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        match Self::exact_ready_snapshot_control(cancellation_flag, options) {
            ParseSnapshotExactReadyControl::Continue => {}
            ParseSnapshotExactReadyControl::SaveCritical => {
                return Ok((parse_result, true));
            }
            ParseSnapshotExactReadyControl::Cancel => {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
        }

        let syntax_errors = TreeSitterAdapter::collect_syntax_errors_only(tree, content);
        match Self::exact_ready_snapshot_control(cancellation_flag, options) {
            ParseSnapshotExactReadyControl::Continue => {}
            ParseSnapshotExactReadyControl::SaveCritical => {
                return Ok((parse_result, true));
            }
            ParseSnapshotExactReadyControl::Cancel => {
                return Err(PARSE_COORDINATOR_CANCELLED_ERROR.to_string());
            }
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
                &mut |i, _| {
                    if i < len {
                        &bytes[i..]
                    } else {
                        Default::default()
                    }
                },
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
                    &mut |i, _| {
                        if i < len {
                            &bytes[i..]
                        } else {
                            Default::default()
                        }
                    },
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
                    &mut |i, _| {
                        if i < len {
                            &bytes[i..]
                        } else {
                            Default::default()
                        }
                    },
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
