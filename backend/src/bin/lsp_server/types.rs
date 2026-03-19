//! Custom types and DTOs for BSL Language Server
//!
//! Contains request/response types for custom LSP commands.

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::notification::Notification;

// ============================================================================
// MILESTONE 2.20.3: Server Status notification
// ============================================================================

/// Custom bsl/serverStatus notification type
pub enum ServerStatus {}

impl Notification for ServerStatus {
    type Params = ServerStatusParams;
    const METHOD: &'static str = "bsl/serverStatus";
}

/// Parameters for bsl/serverStatus notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatusParams {
    pub loading: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ServerStatusParams {
    pub fn loading(message: impl Into<String>) -> Self {
        Self {
            loading: true,
            message: Some(message.into()),
        }
    }

    pub fn ready() -> Self {
        Self {
            loading: false,
            message: None,
        }
    }
}

// ============================================================================
// Custom Request/Response Types (deprecated stubs)
// ============================================================================

/// Custom request: bsl/buildIndex - building type index
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BuildIndexParams {
    pub workspace_path: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct BuildIndexResponse {
    pub success: bool,
    pub types_count: usize,
    pub message: String,
}

/// Custom request: bsl/getIndexState - machine-readable full-index state
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct GetIndexStateParams {}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct GetIndexStateResponse {
    pub version: u32,
    pub state: String,
    pub ready: bool,
    pub active_operation: Option<String>,
    pub operation_id: Option<String>,
    pub message: Option<String>,
    pub updated_at_ms: u64,
}

/// Custom request: bsl/getWorkspaceStats - workspace stats for Overview panel
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStatsResponse {
    pub bsl_files: usize,
    pub diagnostics: usize,
}

/// Custom request: bsl/getObservabilityMetrics - observability metrics snapshot
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityMetricsRequest {
    #[serde(default)]
    pub shape: Option<String>,
}

/// Custom request: bsl/getObservabilityMetrics - observability metrics snapshot
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityMetricsResponse {
    pub metrics: serde_json::Value,
}

/// Custom request: bsl/getCompletionTimeline - per-request completion timeline traces
#[derive(Debug, Deserialize, Default)]
pub struct CompletionTimelineRequest {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelineStageTrace {
    pub name: String,
    pub status: String,
    pub started_offset_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelineTurnHolderTrace {
    pub request_id: Option<String>,
    pub file_seq: u64,
    pub request_epoch: u64,
    pub trigger_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_hint: Option<i32>,
    pub age_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelineTurnAttributionTrace {
    pub request_file_seq: u64,
    pub request_epoch: u64,
    pub queue_outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_wait_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatcher_resolution_latency_ms: Option<u64>,
    pub queue_capacity: usize,
    pub queue_depth_before_enqueue: usize,
    pub queue_depth_after_enqueue: usize,
    pub queued_completion_ahead_count: usize,
    pub did_change_ahead_count: usize,
    pub active_completion_count: usize,
    #[serde(default)]
    pub dropped_completion_file_seq: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_holder: Option<CompletionTimelineTurnHolderTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_completion_ahead: Option<CompletionTimelineTurnHolderTrace>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionTimelinePrepareProgressTrace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_started_offset_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_completed_offset_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_completed_offset_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionTimelineExactWaitDetailsTrace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_ready_before_wait: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_ready_before_wait: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision_head_owner_hints_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_wait_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_index_wait_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_index_waiter_action: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionTimelinePrepareDetailsTrace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_budget_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    pub route: Option<String>,
    pub fail_closed_cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_file_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_version_at_start: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_file_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_age_at_start_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_age_at_terminal_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<CompletionTimelinePrepareProgressTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_wait: Option<CompletionTimelineExactWaitDetailsTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelineServerEdgeDetailsTrace {
    pub transport_received_at_ms: u64,
    pub handler_entered_at_ms: u64,
    pub response_sent_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_observed_at_ms: Option<u64>,
    pub transport_to_handler_wait_ms: u64,
    pub server_handler_exec_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_observed_after_handler_enter_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelineTrace {
    pub trace_id: String,
    pub request_id: Option<String>,
    pub uri: String,
    pub trigger_mode: String,
    pub outcome: String,
    pub started_at_ms: u64,
    pub total_duration_ms: u64,
    pub dominant_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_details: Option<CompletionTimelinePrepareDetailsTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_edge_details: Option<CompletionTimelineServerEdgeDetailsTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_attribution: Option<CompletionTimelineTurnAttributionTrace>,
    pub stages: Vec<CompletionTimelineStageTrace>,
}

#[derive(Debug, Serialize)]
pub struct CompletionTimelineResponse {
    pub version: u32,
    pub traces: Vec<CompletionTimelineTrace>,
}

/// Custom request: bsl/validateMethod - method call validation
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ValidateMethodParams {
    pub object_type: String,
    pub method_name: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ValidateMethodResponse {
    pub valid: bool,
    pub message: String,
}

/// Custom request: bsl/checkTypeCompatibility - type compatibility check
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CheckCompatibilityParams {
    pub source_type: String,
    pub target_type: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct CheckCompatibilityResponse {
    pub compatible: bool,
    pub message: String,
}

/// Custom request: bsl/incrementalUpdate - incremental index update
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct IncrementalUpdateParams {
    pub config_path: String,
    pub platform_version: String,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub is_auto: bool,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct IncrementalUpdateResponse {
    pub success: bool,
    pub message: String,
}

/// Custom request: bsl/pauseAutoReindex and bsl/resumeAutoReindex
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AutoReindexCommandParams {}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct AutoReindexStateResponse {
    pub success: bool,
    pub paused: bool,
    pub message: String,
}

/// Custom request: bsl/extractPlatformDocs - platform documentation extraction
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ExtractPlatformDocsParams {
    pub archive_path: String,
    pub platform_version: String,
    pub force: bool,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ExtractPlatformDocsResponse {
    pub success: bool,
    pub types_count: usize,
    pub message: String,
}

/// Custom request: bsl/renderTypeHtml - render HTML for type (uses TypeVisualization)
/// Reserved for future implementation
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RenderTypeHtmlParams {
    pub type_name: String,
    pub theme: Option<String>, // "light", "dark", "high-contrast"
}

/// Reserved for future implementation
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RenderTypeHtmlResponse {
    pub html: String,
    pub success: bool,
    pub message: Option<String>,
}

// ============================================================================
// GetCurrentContext types
// ============================================================================

/// Custom command: bsl.getCurrentContext - determine current function/procedure
#[derive(Debug, Deserialize)]
pub struct GetCurrentContextParams {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}
