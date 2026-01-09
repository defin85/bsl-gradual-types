//! BSL Language Server implementation
//!
//! Contains the main server struct and LanguageServer trait implementation.
//!
//! This module is split into submodules:
//! - `core`: Constructor and helper methods
//! - `language_server`: Full LanguageServer trait implementation
//! - `command_handlers`: Command-specific handlers

mod command_handlers;
mod core;
mod language_server;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tower_lsp::Client;

use bsl_analysis_v2::{AnalysisHostV2, FileId as V2FileId};
use bsl_backend::system::SystemCoordinator;

use crate::config::{BslSettings, LspConfig};

// Re-export Url for use in submodules
pub use tower_lsp::lsp_types::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum V2FileKey {
    /// Preferred key: filesystem path derived from `Url::to_file_path()`.
    Path(PathBuf),
    /// Fallback key for non-file documents.
    Url(String),
}

/// BSL Language Server backend - CLEAN ARCHITECTURE
#[derive(Clone)]
pub struct BslLanguageServer {
    pub(crate) client: Client,
    pub(crate) documents: Arc<RwLock<HashMap<Url, String>>>,
    pub(crate) diagnostics_counts: Arc<RwLock<HashMap<Url, usize>>>,
    pub(crate) config: Arc<RwLock<Option<LspConfig>>>,
    pub(crate) settings: Arc<RwLock<BslSettings>>,
    pub(crate) completion_snippet_support: Arc<RwLock<bool>>,
    pub(crate) auto_reindex_paused: Arc<RwLock<bool>>,
    pub(crate) coordinator: Arc<SystemCoordinator>,

    pub(crate) use_salsa_v2: bool,
    pub(crate) analysis_host_v2: Arc<Mutex<AnalysisHostV2>>,
    /// Session-stable mapping: once a `FileId` is assigned for a key, it is not revoked for the
    /// lifetime of the server process (even if the document is closed and re-opened).
    pub(crate) file_key_to_file_id_v2: Arc<RwLock<HashMap<V2FileKey, V2FileId>>>,
    pub(crate) next_file_id_v2: Arc<AtomicU32>,
    pub(crate) diagnostics_tasks_v2: Arc<Mutex<HashMap<V2FileId, (i32, JoinHandle<()>)>>>,
}
