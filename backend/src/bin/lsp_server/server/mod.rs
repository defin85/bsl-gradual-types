//! BSL Language Server implementation
//!
//! Contains the main server struct and LanguageServer trait implementation.
//!
//! This module is split into submodules:
//! - `core`: Constructor and helper methods
//! - `language_server`: Full LanguageServer trait implementation
//! - `command_handlers`: Command-specific handlers

mod analysis_v2_runtime;
mod command_handlers;
mod core;
mod language_server;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tower_lsp::Client;

use bsl_analysis_v2::{DepsSnapshotId, FileId as V2FileId, SettingsId};
use bsl_backend::system::SystemCoordinator;

use crate::config::{BslSettings, LspConfig};

// Re-export Url for use in submodules
pub use tower_lsp::lsp_types::Url;

use self::analysis_v2_runtime::AnalysisV2Runtime;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum V2FileKey {
    /// Preferred key: filesystem path derived from `Url::to_file_path()`.
    Path(PathBuf),
    /// Fallback key for non-file documents.
    Url(String),
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FormattingCapabilityState {
    pub dynamic_document_formatting: bool,
    pub dynamic_range_formatting: bool,
    pub registered: bool,
    pub in_flight: bool,
    pub desired_enabled: bool,
}

/// BSL Language Server backend - CLEAN ARCHITECTURE
#[derive(Clone)]
pub struct BslLanguageServer {
    pub(crate) client: Client,
    pub(crate) diagnostics_counts: Arc<RwLock<HashMap<Url, usize>>>,
    pub(crate) config: Arc<RwLock<Option<LspConfig>>>,
    pub(crate) settings: Arc<RwLock<BslSettings>>,
    pub(crate) completion_snippet_support: Arc<RwLock<bool>>,
    pub(crate) auto_reindex_paused: Arc<RwLock<bool>>,
    pub(crate) coordinator: Arc<SystemCoordinator>,
    pub(crate) formatting_capability: Arc<RwLock<FormattingCapabilityState>>,

    pub(crate) analysis_v2: AnalysisV2Runtime,
    /// Serializes `didOpen/didChange/didClose` updates so that incremental changes are applied
    /// against a consistent base text (source of truth lives in `analysis-v2` inputs).
    pub(crate) text_sync_v2: Arc<Mutex<()>>,
    /// Session-stable mapping: once a `FileId` is assigned for a key, it is not revoked for the
    /// lifetime of the server process (even if the document is closed and re-opened).
    pub(crate) file_key_to_file_id_v2: Arc<RwLock<HashMap<V2FileKey, V2FileId>>>,
    pub(crate) next_file_id_v2: Arc<AtomicU32>,
    pub(crate) diagnostics_tasks_v2: Arc<Mutex<DiagnosticsTasksV2>>,
    pub(crate) latest_received_file_versions_v2: Arc<RwLock<HashMap<V2FileId, i32>>>,
    pub(crate) last_deps_id_v2: Arc<RwLock<Option<DepsSnapshotId>>>,
    pub(crate) last_settings_id_v2: Arc<RwLock<Option<SettingsId>>>,
}

pub(crate) struct DiagnosticsTaskV2 {
    pub requested_version: i32,
    pub debounce: bool,
    pub handle: JoinHandle<()>,
}

type DiagnosticsTasksV2 = HashMap<V2FileId, DiagnosticsTaskV2>;

fn parse_env_duration_ms(var: &str) -> Option<Duration> {
    let raw = std::env::var(var).ok()?;
    let parsed = raw.parse::<u64>().ok()?;
    if parsed == 0 {
        return None;
    }
    Some(Duration::from_millis(parsed))
}

fn parse_env_duration_ms_with_default(var: &str, default_ms: u64) -> Option<Duration> {
    match std::env::var(var) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(0) => None,
            Ok(ms) => Some(Duration::from_millis(ms)),
            Err(_) => Some(Duration::from_millis(default_ms)),
        },
        Err(_) => Some(Duration::from_millis(default_ms)),
    }
}

static INTELLISENSE_V2_SLOW_WAIT_WARN_THRESHOLD: LazyLock<Option<Duration>> =
    LazyLock::new(|| parse_env_duration_ms("BSL_INTELLISENSE_V2_SLOW_WAIT_WARN_MS"));

static INTELLISENSE_V2_SLOW_SNAPSHOT_WARN_THRESHOLD: LazyLock<Option<Duration>> =
    LazyLock::new(|| parse_env_duration_ms("BSL_INTELLISENSE_V2_SLOW_SNAPSHOT_WARN_MS"));

static INTELLISENSE_V2_SLOW_QUERY_WARN_THRESHOLD: LazyLock<Option<Duration>> =
    LazyLock::new(|| parse_env_duration_ms("BSL_INTELLISENSE_V2_SLOW_QUERY_WARN_MS"));

static INTELLISENSE_V2_SLOW_CLIENT_LOG_THRESHOLD: LazyLock<Option<Duration>> = LazyLock::new(|| {
    parse_env_duration_ms_with_default("BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS", 2000)
});

pub(crate) fn intellisense_v2_slow_wait_warn_threshold() -> Option<Duration> {
    *INTELLISENSE_V2_SLOW_WAIT_WARN_THRESHOLD
}

pub(crate) fn intellisense_v2_slow_snapshot_warn_threshold() -> Option<Duration> {
    *INTELLISENSE_V2_SLOW_SNAPSHOT_WARN_THRESHOLD
}

pub(crate) fn intellisense_v2_slow_query_warn_threshold() -> Option<Duration> {
    *INTELLISENSE_V2_SLOW_QUERY_WARN_THRESHOLD
}

pub(crate) fn intellisense_v2_slow_client_log_threshold() -> Option<Duration> {
    *INTELLISENSE_V2_SLOW_CLIENT_LOG_THRESHOLD
}
