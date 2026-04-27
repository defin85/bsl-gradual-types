//! Custom types and DTOs for BSL Language Server
//!
//! Contains request/response types for custom LSP commands.

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::notification::Notification;

use bsl_shared::api::dtos::SnapshotReadinessDto;

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

/// Custom bsl/snapshotStatus notification type
pub enum SnapshotStatusNotification {}

impl Notification for SnapshotStatusNotification {
    type Params = SnapshotReadinessDto;
    const METHOD: &'static str = "bsl/snapshotStatus";
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSnapshotStatusRequest {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimeExactTypeIndexRequest {
    pub uri: String,
    #[serde(default)]
    pub requested_version: Option<i64>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimeExactTypeIndexResponse {
    pub accepted: bool,
    pub already_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_version: Option<i64>,
    pub action: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did_change_parse_snapshot_evidence: Option<DidChangeParseSnapshotEvidenceResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeParseSnapshotEvidenceTrace {
    pub evidence_id: String,
    pub uri: String,
    pub requested_version: i32,
    pub started_at_ms: u64,
    pub parse_mode: String,
    pub base_text_source: String,
    pub change_shape: String,
    pub content_changes_count: usize,
    pub replay_order: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_document_version: Option<i32>,
    pub changed_ranges_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser_base_root_cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_document_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_ready_document_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matching_ready_snapshot_for_shadow_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_snapshot_prime_attempted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_cache_matches_shadow_text_after_prime: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeParseSnapshotEvidenceResponse {
    pub version: u32,
    pub entries: Vec<DidChangeParseSnapshotEvidenceTrace>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_wait_entered_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_wait_resolved_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake_after_turn_resolution_at_ms: Option<u64>,
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
pub struct CompletionTimelinePrepareRuntimeTrace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matching_task_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_poll: Option<CompletionTimelineExactArtifactPollTrace>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionTimelineExactArtifactPollTrace {
    pub poll_count: u64,
    pub poll_elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_file_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_ready: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelinePrepareTimeoutAttributionTrace {
    pub source: String,
    pub phase: String,
    pub budget_ms: u64,
    pub elapsed_ms: u64,
    pub overshoot_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionTimelineFirstPollContentionAttributionTrace {
    pub contender_class: String,
    pub uri_scope: String,
    pub inflight_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_inflight_age_ms: Option<u64>,
    pub concurrency_level: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionTimelineFirstPollContentionContenderTrace {
    pub request_class: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    pub age_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionTimelinePrepareDetailsTrace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
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
    pub progress: Option<CompletionTimelinePrepareProgressTrace>,
    pub wait_for_file_version_runtime: Option<CompletionTimelinePrepareRuntimeTrace>,
    pub snapshot_with_deps_runtime: Option<CompletionTimelinePrepareRuntimeTrace>,
    pub snapshot_with_deps_timeout_runtime: Option<CompletionTimelinePrepareRuntimeTrace>,
    pub timeout_attribution: Option<CompletionTimelinePrepareTimeoutAttributionTrace>,
    pub exact_wait: Option<CompletionTimelineExactWaitDetailsTrace>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletionTimelineCollectBreakdownTrace {
    pub member_owner_resolve_ms: u64,
    pub member_methods_ms: u64,
    pub member_properties_ms: u64,
    pub member_metadata_ms: u64,
    pub non_member_local_symbols_ms: u64,
    pub non_member_contextual_symbols_ms: u64,
    pub non_member_module_routines_ms: u64,
    pub non_member_global_functions_ms: u64,
    pub non_member_metadata_items_ms: u64,
    pub non_member_repository_types_ms: u64,
    pub non_member_keywords_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelineServerEdgeDetailsTrace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_read_started_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_read_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_parse_completed_at_ms: Option<u64>,
    pub transport_received_at_ms: u64,
    pub transport_received_at_ms_provenance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc_dispatch_received_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_slot_released_at_ms: Option<u64>,
    pub pre_method_attribution_provenance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_future_created_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_future_first_poll_entered_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_future_first_poll_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_future_first_wake_scheduled_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_scope_entered_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_entered_at_ms: Option<u64>,
    pub handler_entered_at_ms: u64,
    pub response_sent_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_handoff_started_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_handoff_enqueued_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_enqueue_completed_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_encode_started_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_write_started_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_encode_completed_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_flush_completed_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_observed_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_loop_wait_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_loop_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_completion_spillover_depth: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_general_request_staged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_try_enqueue_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_lane: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_lane_depth_before: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_lane_depth_after: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_enqueue_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_spillover_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_enqueued_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_woke_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_poll_ready_entered_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_poll_ready_resolved_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_dequeued_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_barrier_active_at_dequeue: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_barrier_generation: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_barrier_owner_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_barrier_owner_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_barrier_owner_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_barrier_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_sync_first_poll_exec_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_sync_first_poll_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_sync_first_poll_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_sync_first_poll_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_sync_first_poll_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_file_ingress_token_required_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_file_ingress_token_published_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_file_ingress_token_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_file_ingress_token_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_service_call_started_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_service_call_returned_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatch_to_request_context_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_to_dispatch_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_queue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_poll_ready_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_service_call_sync_exec_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler_ready_to_dispatch_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_to_slot_release_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_to_service_future_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_future_to_scope_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_future_to_first_poll_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_poll_to_first_wake_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_poll_contention_attribution:
        Option<CompletionTimelineFirstPollContentionAttributionTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_poll_contention_contenders:
        Option<Vec<CompletionTimelineFirstPollContentionContenderTrace>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_to_service_scope_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_scope_to_method_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_to_method_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_prelude_exec_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_release_to_handler_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_release_to_response_wait_ms: Option<u64>,
    pub transport_to_handler_wait_ms: u64,
    pub server_handler_exec_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_ready_to_output_handoff_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_handoff_send_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_handoff_to_writer_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_ready_to_output_enqueue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_queue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_encode_exec_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_output_write_and_flush_exec_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_ready_to_flush_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_observed_after_handler_enter_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTimelineTrace {
    pub trace_id: String,
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_probe_id: Option<String>,
    pub uri: String,
    pub trigger_mode: String,
    pub outcome: String,
    pub started_at_ms: u64,
    pub total_duration_ms: u64,
    pub dominant_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepare_details: Option<CompletionTimelinePrepareDetailsTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collect_breakdown: Option<CompletionTimelineCollectBreakdownTrace>,
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

/// Custom request: bsl/getCurrentContextTimeline - per-request current-context traces
#[derive(Debug, Deserialize, Default)]
pub struct CurrentContextTimelineRequest {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentContextTimelineTrace {
    pub trace_id: String,
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub started_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_version: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_generation: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness_wait_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_snapshot_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_snapshot_wait_budget_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker_wait_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker_wait_budget_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_ms: Option<u64>,
    pub wall_ms: u64,
    pub supersession_outcome: String,
    pub final_status: String,
}

#[derive(Debug, Serialize)]
pub struct CurrentContextTimelineResponse {
    pub version: u32,
    pub traces: Vec<CurrentContextTimelineTrace>,
}

/// Custom request: bsl/getDiagnosticsSaveTimeline - per-save diagnostics refresh traces
#[derive(Debug, Deserialize, Default)]
pub struct DiagnosticsSaveTimelineRequest {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticsSaveTimelinePublishTrace {
    pub profile: String,
    pub publish_kind: String,
    pub outcome: String,
    pub elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syntax_work_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_parse_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_ir_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_materialization_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_queue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_lag_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking_queue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_for_file_version_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_with_deps_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syntax_diagnostics_query_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_query_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_inputs_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_parse_result_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_collect_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_flow_sensitive_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_ast_to_ir_convert_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_materialize_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_seed_module_context_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_prep_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_snapshot_build_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_body_infer_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_function_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_scc_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_fixed_point_iteration_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_singleton_fast_path_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summaries_recursive_scc_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_visit_statements_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_visit_callable_body_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_visit_callable_body_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_merge_control_flow_env_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_source_incomplete_member_access_recovery_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_syntax_incomplete_member_access_recovery_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_incomplete_call_target_recovery_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_statement_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_local_function_summary_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_semantic_facts_index_entry_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_seed_module_context_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_prep_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_snapshot_build_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_body_infer_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_function_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_scc_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_iteration_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_singleton_fast_path_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summaries_recursive_scc_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_statements_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_visit_callable_body_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_merge_control_flow_env_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_statement_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_local_function_summary_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics_ir_diagnostics_only_semantic_facts_index_entry_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_wait_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticsSaveTimelinePhaseMark {
    pub phase: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticsSaveTimelineTrace {
    pub trace_id: String,
    pub uri: String,
    pub requested_version: i32,
    pub save_cycle_sequence: u64,
    pub diagnostics_generation: u64,
    pub trigger: String,
    pub started_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_publish: Option<DiagnosticsSaveTimelinePublishTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_publish: Option<DiagnosticsSaveTimelinePublishTrace>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub followup_profile_phase_marks: Vec<DiagnosticsSaveTimelinePhaseMark>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_fastlane_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_heavy_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_syntax_work_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_semantic_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_semantic_parse_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_semantic_ir_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_semantic_materialization_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_save_fastlane_gate_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_save_fastlane_gate_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_admission_queue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_zero_probe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_wait_probe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_bounded_wait_winner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_bounded_wait_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_task_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_did_save_exact_producer_lifecycle_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_did_save_exact_producer_lifecycle_state_at_timeout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_did_save_exact_producer_final_lifecycle_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_timeout_phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_timeout_phase_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_timeout_leaf: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_timeout_leaf_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_timeout_subphase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_parse_build_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint:
        Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome:
        Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_largest_rebuilt_window_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_node_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_node_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reuse_node_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_reused_top_level_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_fully_rebuilt_top_level_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_prefix_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_reused_suffix_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_routine_body_rebuilt_lowering_units:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source:
        Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit:
        Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit:
        Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_owned_build_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_build_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_rebase_statement_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_call_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_call_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_call_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_body_dispatch_call_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_callable_non_body_dispatch_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_control_flow_call_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_simple_call_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_other_call_count:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint:
        Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms:
        Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_dominant_subphase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_parse_exec_dominant_subphase_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_post_parse_pre_materialization_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_ready_install_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_document_symbol_side_work_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_dominant_phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_dominant_phase_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_relief_valve_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_relief_valve_budget_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_relief_valve_elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_ready_snapshot_continuation_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_shadow_state_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_wait_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_blocker_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_runtime_queue_wait_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_apply_lag_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_wait_for_file_version_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_snapshot_with_deps_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_readiness_blocker_bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followup_unclassified_readiness_residual_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_outcome: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticsSaveTimelineResponse {
    pub version: u32,
    pub traces: Vec<DiagnosticsSaveTimelineTrace>,
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
#[serde(rename_all = "camelCase")]
pub struct GetCurrentContextParams {
    pub uri: String,
    pub line: u32,
    pub character: u32,
    #[serde(default)]
    pub editor_session_id: Option<String>,
    #[serde(default)]
    pub request_generation: Option<u64>,
}
