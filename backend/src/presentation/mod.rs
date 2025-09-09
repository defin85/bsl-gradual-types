//! Presentation layer - organized by interface type
//! Protocol adapters and UI components

// LSP interface components
pub mod lsp;

// Web interface components  
pub mod web;

// CLI interface components
pub mod cli;

// Re-exports for backward compatibility
pub use lsp::{enhanced, code_actions, type_hints, position};

/// Фильтры для поиска
#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub query: Option<String>,
    pub file_types: Vec<String>,
    pub include_deprecated: bool,
}
