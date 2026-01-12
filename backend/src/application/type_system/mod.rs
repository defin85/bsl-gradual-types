//! Type System Service - main service for BSL type system
//!
//! Phase 4: Unified API - replaces LspTypeService + WebTypeService + AnalysisService
//! with unified entrypoints for presentation layers.
//!
//! # Module Structure
//!
//! ```text
//! type_system/
//! ├── mod.rs                  # This file - re-exports
//! ├── services/               # Business logic services
//! │   ├── hover_service.rs    # LSP hover operations
//! │   ├── completion_service.rs # LSP completion operations
//! │   └── web_api_service.rs  # Web API operations
//! ├── formatters/             # Formatting utilities
//! │   ├── hover_formatters.rs # Hover tooltip formatting
//! │   └── type_formatters.rs  # Type name formatting
//! ├── extractors/             # Extraction utilities
//! │   ├── symbol_extractor.rs # Symbol extraction from source
//! │   └── type_extractor.rs   # Type extraction from AST
//! ```

mod extractors;
mod formatters;
mod services;

pub use services::web_api_service;
pub use services::completion_service::{CompletionContext, CompletionStats, get_completion};
pub use services::completion_service::get_completion_with_semantic_program;
pub use services::completion_service::get_completion_with_semantic_program_snapshot;
pub use services::completion_service::{
    build_call_snippet, CompletionResolveDetails, resolve_method_completion, resolve_type_details,
};
pub use services::hover_service::get_hover_info_with_semantic_program;
