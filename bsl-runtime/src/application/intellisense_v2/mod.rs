//! Unified IntelliSense v2 facade contract shared by LSP/Web/MCP adapters.
//!
//! This module is the canonical orchestration API surface for semantic operations.

mod facade;
mod policy;

pub use facade::{
    CancellationPolicy, ExecutionContext, ExecutionSettings, IntellisenseV2Facade,
    ObservabilityMetricKind, ObservabilityStage, PreparedOperationSnapshot, SemanticOperation,
    SemanticOutcome, SemanticSnapshot,
};
pub use policy::{
    classify_optional_query, should_query_parse_result, spawn_bounded_blocking, RuntimePerfKnobs,
};
