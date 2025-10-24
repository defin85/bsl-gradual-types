//! Presentation layer - organized by interface type
//! Protocol adapters and UI components

// LSP interface components
pub mod lsp;

// Web interface components
pub mod web;

// CLI interface components
pub mod cli;

// Semantic HTML generation (MILESTONE E2 - Task 2.1)
pub mod semantic_html_generator;

// Semantic visualization API (MILESTONE 2.16)
pub mod semantic_routes;

// Re-exports for backward compatibility
pub use lsp::{/* enhanced, */ code_actions, position, type_hints};

/// Фильтры для поиска
#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub query: Option<String>,
    pub file_types: Vec<String>,
    pub include_deprecated: bool,
}
