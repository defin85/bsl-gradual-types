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

pub mod extractors;
mod formatters;
mod services;

pub use services::completion_service::get_completion_with_semantic_program;
pub use services::completion_service::get_completion_with_semantic_program_snapshot;
pub use services::completion_service::get_completion_with_semantic_program_snapshot_v2;
pub use services::completion_service::get_completion_with_semantic_program_snapshot_v2_with_trigger_hint;
pub use services::completion_service::get_completion_with_semantic_program_snapshot_with_owner_hints;
pub use services::completion_service::get_completion_with_semantic_program_snapshot_with_trigger_hint;
pub use services::completion_service::get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints;
pub use services::completion_service::get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids;
pub use services::completion_service::get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids_and_global_context;
pub use services::completion_service::get_completion_with_trigger_hint_and_owner_hints_without_ir;
pub use services::completion_service::get_completion_with_trigger_hint_and_owner_hints_without_ir_with_snapshot_ids;
pub use services::completion_service::get_completion_with_trigger_hint_and_owner_hints_without_ir_with_snapshot_ids_and_global_context;
pub use services::completion_service::CompletionCollectBreakdown;
pub use services::completion_service::CompletionStats;
pub use services::completion_service::{
    build_call_snippet, resolve_method_completion, resolve_type_details,
    CompletionHeadTypeHintsForVersionRequest, CompletionResolveDetails,
};
pub use services::completion_service::{
    completion_member_access_owner_type_hint_from_analysis,
    completion_member_access_owner_type_hint_from_analysis_with_flow_sensitive,
    completion_member_access_owner_type_hints_from_analysis,
    completion_member_access_owner_type_hints_from_analysis_with_flow_sensitive,
    completion_member_access_owner_type_hints_from_completion_head,
    completion_member_access_owner_type_hints_from_completion_head_for_version,
    completion_member_access_owner_type_hints_from_head_receiver,
    completion_member_access_owner_type_hints_from_static_receiver,
};
pub use services::definition_service::{
    definition_exact_type_index_available_at_position, goto_definition_v2,
    goto_definition_v2_with_source, goto_definition_v2_with_source_and_analysis, DefinitionRequest,
    DefinitionTarget,
};
pub use services::hover_service::{
    get_hover_info_with_semantic_program, hover_exact_type_index_available_at_position,
};
pub use services::signature_help_service::{
    get_signature_help_v2, get_signature_help_v2_with_analysis,
    signature_help_exact_type_index_available_at_position, signature_help_query, SignatureHelpData,
    SignatureHelpQuery, SignatureHelpRequest,
};
pub use services::web_api_service;
