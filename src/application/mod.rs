//! Application layer (flat structure)
//! Specialized services for different use cases

pub mod analysis_service;
pub mod code_actions;
pub mod documentation_service;
pub mod lsp_enhanced;
pub mod lsp_service;
pub mod services;
pub mod type_system_service;
pub mod web_service;

// Re-export main services
pub use lsp_service::LspTypeService;
pub use services::AnalysisTypeService;
pub use type_system_service::{TypeSystemService, CompletionItem}; // Main unified service
pub use web_service::WebTypeService;
