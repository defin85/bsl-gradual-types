//! Application layer (flat structure)
//! Specialized services for different use cases

pub mod lsp_service;
pub mod web_service;
pub mod analysis_service;
pub mod documentation_service;
pub mod lsp_enhanced;
pub mod code_actions;
pub mod services;

// Re-export main services
pub use services::{AnalysisTypeService, LspTypeService, WebTypeService};
pub use analysis_service::AnalysisService;
