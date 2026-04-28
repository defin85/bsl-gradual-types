//! Application layer.

pub mod semantic_validation_visitor; // Milestone 3.7: Semantic Diagnostics MVP

pub mod intellisense_v2;

pub mod type_system;

pub use bsl_shared::domain::CompletionItem;

pub use intellisense_v2::{
    classify_optional_query, completion_missing_ir_policy_decision, cpu_work_class_for_operation,
    diagnostics_execution_plan, diagnostics_profiles_for_trigger, did_save_followup_lane_quota,
    scale_aware_document_is_large, should_query_parse_result, spawn_bounded_blocking,
    spawn_bounded_blocking_with_class, spawn_bounded_blocking_with_class_observed,
    spawn_bounded_blocking_with_class_observed_call_origin,
    spawn_bounded_blocking_with_class_observed_call_origin_dynamic_lane_hooks,
    spawn_bounded_blocking_with_class_observed_call_origin_hooks,
    spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks,
    spawn_bounded_blocking_with_class_observed_origin, AdmissionLane, CancellationPolicy,
    CompletionCurrentRevisionSnapshot, CompletionFirstResponseReadiness,
    CompletionFirstResponseSupport, CompletionMissingIrPolicyDecision, CompletionMode,
    CompletionPipelineKnobs, CompletionSupportBundle, CpuWorkClass, DeferredHeavyDiagnosticsReason,
    DiagnosticsDisposition, DiagnosticsExecutionPlan, DiagnosticsProfile, DiagnosticsTrigger,
    ExecutionContext, ExecutionSettings, IntellisenseV2Facade, ObservabilityMetricKind,
    ObservabilityOrigin, ObservabilityStage, ObservedBlockingCall, PrepareStatefulProgress,
    PrepareStatefulProgressSnapshot, PrepareTimeoutAttributionTrace, PrepareTimeoutSourceKind,
    PreparedCompletionFirstResponse, PreparedOperationSnapshot, RuntimePerfKnobs,
    ScaleAwareDiagnosticsKnobs, SemanticOperation, SemanticOutcome, SemanticSnapshot,
    SingleflightQueryError, SnapshotWithDepsRuntimeTrace, SnapshotWithDepsTimeoutResolutionKind,
    SnapshotWithDepsTimeoutRuntimeTrace, WaitForFileVersionResolutionKind,
    WaitForFileVersionRuntimeTrace,
};
pub use type_system::get_completion_with_semantic_program;
pub use type_system::get_completion_with_semantic_program_snapshot;
pub use type_system::get_completion_with_semantic_program_snapshot_v2;
pub use type_system::get_completion_with_semantic_program_snapshot_v2_with_trigger_hint;
pub use type_system::get_completion_with_semantic_program_snapshot_with_owner_hints;
pub use type_system::get_completion_with_semantic_program_snapshot_with_trigger_hint;
pub use type_system::get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints;
pub use type_system::get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids;
pub use type_system::get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints_with_snapshot_ids_and_global_context;
pub use type_system::get_completion_with_trigger_hint_and_owner_hints_without_ir;
pub use type_system::get_completion_with_trigger_hint_and_owner_hints_without_ir_with_snapshot_ids;
pub use type_system::get_completion_with_trigger_hint_and_owner_hints_without_ir_with_snapshot_ids_and_global_context;
pub use type_system::get_hover_info_with_semantic_program;
pub use type_system::CompletionCollectBreakdown;
pub use type_system::CompletionStats;
pub use type_system::{
    completion_member_access_owner_type_hint_from_analysis,
    completion_member_access_owner_type_hint_from_analysis_with_flow_sensitive,
    completion_member_access_owner_type_hints_from_analysis,
    completion_member_access_owner_type_hints_from_analysis_with_flow_sensitive,
    completion_member_access_owner_type_hints_from_completion_head,
    completion_member_access_owner_type_hints_from_completion_head_for_version,
    completion_member_access_owner_type_hints_from_head_receiver,
    completion_member_access_owner_type_hints_from_static_receiver,
    CompletionHeadTypeHintsForVersionRequest,
};
