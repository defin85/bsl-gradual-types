//! Unified IntelliSense v2 facade contract shared by LSP/Web/MCP adapters.
//!
//! This module is the canonical orchestration API surface for semantic operations.

mod facade;
mod policy;

pub use facade::{
    CancellationPolicy, CompletionCurrentRevisionSnapshot, CompletionFirstResponseReadiness,
    CompletionFirstResponseSupport, CompletionSupportBundle, ExecutionContext, ExecutionSettings,
    IntellisenseV2Facade, ObservabilityMetricKind, ObservabilityOrigin, ObservabilityStage,
    PrepareStatefulProgress, PrepareStatefulProgressSnapshot, PrepareTimeoutAttributionTrace,
    PrepareTimeoutSourceKind, PreparedCompletionFirstResponse, PreparedOperationSnapshot,
    SemanticOperation, SemanticOutcome, SemanticSnapshot, SingleflightQueryError,
    SnapshotWithDepsRuntimeTrace, SnapshotWithDepsTimeoutResolutionKind,
    SnapshotWithDepsTimeoutRuntimeTrace, WaitForFileVersionResolutionKind,
    WaitForFileVersionRuntimeTrace,
};
pub use policy::{
    classify_optional_query, completion_fastpath_preconditions,
    completion_missing_ir_policy_decision, cpu_work_class_for_operation,
    diagnostics_execution_plan, diagnostics_profiles_for_trigger, interactive_freshness_knobs,
    scale_aware_document_is_large, should_query_parse_result, spawn_bounded_blocking,
    spawn_bounded_blocking_with_class, spawn_bounded_blocking_with_class_observed,
    spawn_bounded_blocking_with_class_observed_call_origin,
    spawn_bounded_blocking_with_class_observed_origin, CompletionFastpathPreconditions,
    CompletionMissingIrPolicyDecision, CompletionMode, CompletionPipelineKnobs, CpuWorkClass,
    DeferredHeavyDiagnosticsReason, DiagnosticsDisposition, DiagnosticsExecutionPlan,
    DiagnosticsProfile, DiagnosticsTrigger, InteractiveFreshnessKnobs, ObservedBlockingCall,
    RuntimePerfKnobs, ScaleAwareDiagnosticsKnobs,
};
