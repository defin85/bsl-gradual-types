//! Type System Service - main service for BSL type system
//!
//! Phase 4: Unified API - replaces LspTypeService + WebTypeService + AnalysisService
//! with single unified API for all presentation layers.
//!
//! # Module Structure
//!
//! ```text
//! type_system/
//! ├── mod.rs                  # This file - re-exports
//! ├── service.rs              # TypeSystemService struct + core methods
//! ├── services/               # Business logic services
//! │   ├── hover_service.rs    # LSP hover operations
//! │   ├── completion_service.rs # LSP completion operations
//! │   ├── validation_service.rs # Code validation
//! │   ├── file_analysis_service.rs # File analysis
//! │   └── web_api_service.rs  # Web API operations
//! ├── formatters/             # Formatting utilities
//! │   ├── hover_formatters.rs # Hover tooltip formatting
//! │   └── type_formatters.rs  # Type name formatting
//! ├── extractors/             # Extraction utilities
//! │   ├── symbol_extractor.rs # Symbol extraction from source
//! │   └── type_extractor.rs   # Type extraction from AST
//! └── loaders/                # Loading utilities
//!     └── configuration_loader.rs # 1C configuration loading
//! ```

mod extractors;
mod formatters;
mod loaders;
mod service;
mod services;

// Main re-export
pub use service::{CompletionContext, TypeSystemService};
pub use services::completion_service::{get_completion, CompletionStats};
pub use services::completion_service::get_completion_with_semantic_program;
pub use services::completion_service::get_completion_with_semantic_program_snapshot;
pub use services::hover_service::get_hover_info_with_semantic_program;

// Re-export completion context for external usage
pub use services::completion_service::CompletionContext as CompletionContextDetails;
