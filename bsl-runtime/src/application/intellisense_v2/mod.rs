//! Unified IntelliSense v2 facade contract shared by LSP/Web/MCP adapters.
//!
//! This module is the canonical orchestration API surface for semantic operations.

mod facade;
mod policy;

pub use facade::{
    CancellationPolicy, ExecutionContext, ExecutionSettings, IntellisenseV2Facade,
    ObservabilityMetricKind, ObservabilityOrigin, ObservabilityStage, PreparedOperationSnapshot,
    SemanticOperation, SemanticOutcome, SemanticSnapshot, SingleflightQueryError,
};
pub use policy::{
    classify_optional_query, completion_missing_ir_policy_decision, cpu_work_class_for_operation,
    diagnostics_execution_plan, diagnostics_profiles_for_trigger, interactive_freshness_knobs,
    should_query_parse_result, spawn_bounded_blocking, spawn_bounded_blocking_with_class,
    spawn_bounded_blocking_with_class_observed, spawn_bounded_blocking_with_class_observed_origin,
    CompletionMissingIrPolicyDecision, CompletionMode, CompletionPipelineKnobs, CpuWorkClass,
    DiagnosticsDisposition, DiagnosticsExecutionPlan, DiagnosticsProfile, DiagnosticsTrigger,
    InteractiveFreshnessKnobs, RuntimePerfKnobs,
};
