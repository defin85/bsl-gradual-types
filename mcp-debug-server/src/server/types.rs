//! Parameter types for MCP Debug Server tools
//!
//! Contains all parameter structs used by debug tools.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Параметры для debug_create_session
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSessionParams {
    pub binary_path: String,
    #[serde(default)]
    pub adapter_type: Option<String>,
}

/// Параметры для debug_set_breakpoint
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetBreakpointParams {
    pub session_id: String,
    pub file: String,
    pub line: u32,
}

/// Параметры для debug_launch
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LaunchParams {
    pub session_id: String,
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

/// Параметры для stepping operations (next, step_in, continue, step_out)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepParams {
    pub session_id: String,
}

/// Параметры для debug_eval
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvalParams {
    pub session_id: String,
    pub expression: String,
}

/// Параметры для debug_backtrace
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BacktraceParams {
    pub session_id: String,
}

/// Параметры для debug_set_conditional_breakpoint
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConditionalBreakpointParams {
    pub session_id: String,
    pub file: String,
    pub line: u32,
    pub condition: String,
}

/// Параметры для debug_terminate
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerminateParams {
    pub session_id: String,
}

/// Параметры для debug_poll_events
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PollEventsParams {
    pub session_id: String,
}
