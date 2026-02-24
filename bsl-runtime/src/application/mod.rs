//! Application layer.

pub mod semantic_validation_visitor; // Milestone 3.7: Semantic Diagnostics MVP

pub mod intellisense_v2;

pub mod type_system;

pub use bsl_shared::domain::CompletionItem;

pub use intellisense_v2::{
    classify_optional_query, completion_missing_ir_policy_decision, cpu_work_class_for_operation,
    diagnostics_execution_plan, diagnostics_profiles_for_trigger, scale_aware_document_is_large,
    should_query_parse_result, spawn_bounded_blocking, spawn_bounded_blocking_with_class,
    spawn_bounded_blocking_with_class_observed, spawn_bounded_blocking_with_class_observed_origin,
    CancellationPolicy, CompletionMissingIrPolicyDecision, CompletionMode, CompletionPipelineKnobs,
    CpuWorkClass, DeferredHeavyDiagnosticsReason, DiagnosticsDisposition, DiagnosticsExecutionPlan,
    DiagnosticsProfile, DiagnosticsTrigger, ExecutionContext, ExecutionSettings,
    IntellisenseV2Facade, ObservabilityMetricKind, ObservabilityOrigin, ObservabilityStage,
    PreparedOperationSnapshot, RuntimePerfKnobs, ScaleAwareDiagnosticsKnobs, SemanticOperation,
    SemanticOutcome, SemanticSnapshot, SingleflightQueryError,
};
pub use type_system::get_completion_with_semantic_hint_snapshot_with_trigger_hint;
pub use type_system::get_completion_with_semantic_program;
pub use type_system::get_completion_with_semantic_program_snapshot;
pub use type_system::get_completion_with_semantic_program_snapshot_v2;
pub use type_system::get_completion_with_semantic_program_snapshot_v2_with_trigger_hint;
pub use type_system::get_completion_with_semantic_program_snapshot_with_trigger_hint;
pub use type_system::get_hover_info_with_semantic_program;
pub use type_system::{get_completion, CompletionContext, CompletionStats};
