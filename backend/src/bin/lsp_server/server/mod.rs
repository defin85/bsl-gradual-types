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
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;

use bsl_backend::system::SystemCoordinator;

use crate::config::{BslSettings, LspConfig};

// Re-export Url for use in submodules
pub use tower_lsp::lsp_types::Url;

/// BSL Language Server backend - CLEAN ARCHITECTURE
#[derive(Clone)]
pub struct BslLanguageServer {
    pub(crate) client: Client,
    pub(crate) documents: Arc<RwLock<HashMap<Url, String>>>,
    pub(crate) config: Arc<RwLock<Option<LspConfig>>>,
    pub(crate) settings: Arc<RwLock<BslSettings>>,
    pub(crate) coordinator: Arc<SystemCoordinator>,
}
