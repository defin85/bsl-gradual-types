//! Application layer.

pub mod semantic_validation_visitor; // Milestone 3.7: Semantic Diagnostics MVP

pub mod intellisense_v2;

pub mod type_system;

pub use bsl_shared::domain::CompletionItem;

pub use intellisense_v2::{
    classify_optional_query, should_query_parse_result, spawn_bounded_blocking, CancellationPolicy,
    ExecutionContext, ExecutionSettings, IntellisenseV2Facade, ObservabilityMetricKind,
    ObservabilityStage, RuntimePerfKnobs, SemanticOperation, SemanticOutcome, SemanticSnapshot,
};
pub use type_system::get_completion_with_semantic_program;
pub use type_system::get_completion_with_semantic_program_snapshot;
pub use type_system::get_completion_with_semantic_program_snapshot_v2;
pub use type_system::get_hover_info_with_semantic_program;
pub use type_system::{get_completion, CompletionContext, CompletionStats};
