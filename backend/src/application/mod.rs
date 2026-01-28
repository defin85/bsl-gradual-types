//! Application layer.

pub mod semantic_validation_visitor; // Milestone 3.7: Semantic Diagnostics MVP
pub mod type_inference_service;

pub mod type_system;

pub use bsl_shared::domain::CompletionItem;
pub use type_inference_service::TypeInferenceService;

pub use type_system::get_completion_with_semantic_program;
pub use type_system::get_completion_with_semantic_program_snapshot;
pub use type_system::get_completion_with_semantic_program_snapshot_v2;
pub use type_system::get_hover_info_with_semantic_program;
pub use type_system::{get_completion, CompletionContext, CompletionStats};
