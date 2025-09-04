//! Presentation layer (flat structure)  
//! Protocol adapters and UI components

pub mod code_actions;
pub mod lsp_enhanced;
pub mod position;
pub mod type_hints;
pub mod web_ui;

/// Фильтры для поиска
#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub query: Option<String>,
    pub file_types: Vec<String>,
    pub include_deprecated: bool,
}
