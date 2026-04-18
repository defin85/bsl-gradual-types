//! Basic Observability - простая замена сложного observability stack
//!
//! Structured logging + basic metrics вместо полного enterprise стека

use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Простая система наблюдения
pub struct BasicObservability {
    logger: StructuredLogger,
    metrics: SimpleMetrics,
    start_time: Instant,
}

/// Структурированные логи
pub struct StructuredLogger {
    // Wrapper для tracing с JSON форматированием
}

/// Простые метрики
pub struct SimpleMetrics {
    counters: Arc<Mutex<HashMap<String, u64>>>,
    gauges: Arc<Mutex<HashMap<String, f64>>>,
    histograms: Arc<Mutex<HashMap<String, Vec<f64>>>>,
    start_time: Instant,
}

/// Статус здоровья системы
pub struct HealthStatus {
    pub status: String,
    pub uptime: Duration,
    pub components: Vec<ComponentHealth>,
}

/// Здоровье отдельного компонента
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CompletionOwnerHintIndexFetchSalsaCounters {
    pub block_on_total: u64,
    pub block_on_type_index_total: u64,
    pub block_on_parse_result_total: u64,
    pub block_on_other_total: u64,
    pub will_execute_total: u64,
    pub will_execute_type_index_total: u64,
    pub will_execute_parse_result_total: u64,
    pub will_execute_other_total: u64,
    pub did_validate_memoized_total: u64,
    pub did_validate_memoized_type_index_total: u64,
    pub did_validate_memoized_parse_result_total: u64,
    pub did_validate_memoized_other_total: u64,
    pub will_check_cancellation_total: u64,
}

const UNIFIED_INTELLISENSE_V2_COUNTER_KEYS: &[&str] = &[
    "intellisense_v2_runtime_wait_for_file_version_queue_wait_total",
    "intellisense_v2_runtime_wait_for_file_version_exec_total",
    "intellisense_v2_runtime_snapshot_with_deps_queue_wait_total",
    "intellisense_v2_runtime_snapshot_with_deps_exec_total",
    "intellisense_v2_runtime_apply_changes_queue_wait_total",
    "intellisense_v2_runtime_apply_changes_exec_total",
    "intellisense_v2_runtime_apply_change_set_file_exec_total",
    "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_total",
    "intellisense_v2_runtime_apply_change_remove_file_exec_total",
    "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_total",
    "intellisense_v2_runtime_type_index_precompute_queue_wait_total",
    "intellisense_v2_runtime_type_index_precompute_exec_total",
    "intellisense_v2_runtime_type_index_precompute_build_exec_total",
    "intellisense_v2_runtime_type_index_precompute_ir_exec_total",
    "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_total",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_incremental",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_reused",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_full",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_other",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_edits_do_not_match_new_content",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_input_edit_conversion_failed",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_incremental_parse_failed",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_stale_parser_base",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_previous_tree",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_edits_provided",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_other",
    "intellisense_v2_wait_for_file_version_diagnostics_total",
    "intellisense_v2_snapshot_diagnostics_total",
    "intellisense_v2_ir_query_other_total",
    "intellisense_v2_syntax_diagnostics_query_total",
    "intellisense_v2_semantic_diagnostics_query_total",
    "intellisense_v2_semantic_diagnostics_materialization_path_total_path_diagnostics_only",
    "intellisense_v2_semantic_diagnostics_materialization_path_total_path_full_semantic_facts_fallback",
    "intellisense_v2_semantic_diagnostics_materialization_path_total_path_other",
    "intellisense_v2_parse_result_query_total",
    "intellisense_v2_ir_query_cancelled_total_other",
    "intellisense_v2_query_cancelled_total_syntax",
    "intellisense_v2_query_cancelled_total_semantic",
    "intellisense_v2_interactive_wait_budget_exhausted_total",
    "intellisense_v2_interactive_stale_served_total",
    "intellisense_v2_interactive_knob_clamped_total",
    "intellisense_v2_singleflight_leader_total",
    "intellisense_v2_singleflight_shared_total",
    "intellisense_v2_singleflight_key_unavailable_total",
    "intellisense_v2_runtime_queue_wait_interactive_total",
    "intellisense_v2_runtime_queue_wait_background_total",
    "intellisense_v2_runtime_exec_interactive_total",
    "intellisense_v2_runtime_exec_background_total",
    "intellisense_v2_completion_stale_fallback_total",
    "intellisense_v2_completion_fallback_unavailable_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch",
    "intellisense_v2_completion_exact_type_index_wait_promotion_total",
    "intellisense_v2_completion_exact_type_index_wait_join_total",
    "intellisense_v2_completion_exact_type_index_wait_ready_after_wait_total",
    "intellisense_v2_revision_lag_sample_total",
    "intellisense_v2_observability_contract_violation_total",
    "intellisense_v2_projection_missing_total",
    "intellisense_v2_runtime_saturation_sample_total",
    "intellisense_v2_current_context_parse_source_total_source_ready_snapshot",
    "intellisense_v2_current_context_parse_source_total_source_parser_coordinator",
    "intellisense_v2_current_context_parse_source_total_source_syntax_fallback",
    "intellisense_v2_current_context_parse_source_total_source_parse_unavailable",
    "intellisense_v2_current_context_parse_source_total_source_other",
    "intellisense_v2_current_context_role_total_role_ready_snapshot",
    "intellisense_v2_current_context_role_total_role_broker_leader",
    "intellisense_v2_current_context_role_total_role_broker_follower",
    "intellisense_v2_current_context_role_total_role_other",
    "intellisense_v2_current_context_terminal_total_outcome_resolved",
    "intellisense_v2_current_context_terminal_total_outcome_parse_unavailable",
    "intellisense_v2_current_context_terminal_total_outcome_superseded",
    "intellisense_v2_current_context_terminal_total_outcome_budget_exhausted",
    "intellisense_v2_current_context_terminal_total_outcome_other",
    "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_open",
    "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_change",
    "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_did_save",
    "intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_other",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_open",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_save",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_other",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_not_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_timeout",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_zero_budget_outcome_other",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_not_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_timeout",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_bounded_wait_outcome_other",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_pending_publish",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_runtime_queue_wait",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_semantic_work",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_apply_lag",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_other",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_ready_artifacts",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_shadow_state",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_generic_pipeline",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_other",
];

const UNIFIED_INTELLISENSE_V2_HISTOGRAM_KEYS: &[&str] = &[
    "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
    "intellisense_v2_runtime_wait_for_file_version_exec_ms",
    "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
    "intellisense_v2_runtime_snapshot_with_deps_exec_ms",
    "intellisense_v2_runtime_apply_changes_queue_wait_ms",
    "intellisense_v2_runtime_apply_changes_exec_ms",
    "intellisense_v2_runtime_apply_change_set_file_exec_ms",
    "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms",
    "intellisense_v2_runtime_apply_change_remove_file_exec_ms",
    "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_queue_wait_ms",
    "intellisense_v2_runtime_type_index_precompute_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
    "intellisense_v2_runtime_apply_changes_batch_size",
    "intellisense_v2_runtime_apply_changes_changed_files_count",
    "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_revision_start",
    "intellisense_v2_completion_owner_hint_index_fetch_revision_end",
    "intellisense_v2_completion_owner_hint_index_fetch_revision_delta",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_incremental",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_reused",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_full",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_other",
    "intellisense_v2_parse_snapshot_changed_ranges_count_origin_lsp",
    "intellisense_v2_parse_snapshot_changed_ranges_bytes_origin_lsp",
    "intellisense_v2_wait_for_file_version_diagnostics_ms",
    "intellisense_v2_snapshot_diagnostics_ms",
    "intellisense_v2_ir_query_other_ms",
    "intellisense_v2_syntax_diagnostics_query_ms",
    "intellisense_v2_semantic_diagnostics_query_ms",
    "intellisense_v2_parse_result_query_ms",
    "intellisense_v2_singleflight_wait_ms",
    "intellisense_v2_runtime_queue_wait_interactive_ms",
    "intellisense_v2_runtime_queue_wait_background_ms",
    "intellisense_v2_runtime_exec_interactive_ms",
    "intellisense_v2_runtime_exec_background_ms",
    "completion_stage_prepare_apply_age_at_start_ms",
    "completion_stage_prepare_apply_age_at_terminal_ms",
    "completion_stage_exact_wait_apply_age_at_start_ms",
    "completion_stage_exact_wait_apply_age_at_terminal_ms",
    "intellisense_v2_semantic_diagnostics_query_inputs_ms",
    "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
    "intellisense_v2_semantic_diagnostics_query_ir_ms",
    "intellisense_v2_semantic_diagnostics_query_collect_ms",
    "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_seed_module_context_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_prep_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_snapshot_build_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_body_infer_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_function_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_scc_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_fixed_point_iteration_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_singleton_fast_path_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summaries_recursive_scc_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_visit_statements_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_visit_callable_body_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_visit_callable_body_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_merge_control_flow_env_ms",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_merge_control_flow_env_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_statement_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_local_function_summary_count",
    "intellisense_v2_semantic_diagnostics_diagnostics_only_semantic_facts_index_entry_count",
    "intellisense_v2_revision_lag_versions",
    "completion_stage_collect_member_owner_resolve_ms",
    "completion_stage_collect_member_methods_ms",
    "completion_stage_collect_member_properties_ms",
    "completion_stage_collect_member_metadata_ms",
    "completion_stage_collect_non_member_local_symbols_ms",
    "completion_stage_collect_non_member_contextual_symbols_ms",
    "completion_stage_collect_non_member_module_routines_ms",
    "completion_stage_collect_non_member_global_functions_ms",
    "completion_stage_collect_non_member_metadata_items_ms",
    "completion_stage_collect_non_member_repository_types_ms",
    "completion_stage_collect_non_member_keywords_ms",
    "intellisense_v2_current_context_parse_ms_source_ready_snapshot",
    "intellisense_v2_current_context_parse_ms_source_parser_coordinator",
    "intellisense_v2_current_context_parse_ms_source_syntax_fallback",
    "intellisense_v2_current_context_parse_ms_source_parse_unavailable",
    "intellisense_v2_current_context_parse_ms_source_other",
    "intellisense_v2_current_context_wall_ms_source_ready_snapshot",
    "intellisense_v2_current_context_wall_ms_source_parser_coordinator",
    "intellisense_v2_current_context_wall_ms_source_syntax_fallback",
    "intellisense_v2_current_context_wall_ms_source_parse_unavailable",
    "intellisense_v2_current_context_wall_ms_source_other",
    "intellisense_v2_current_context_parse_ms_role_ready_snapshot",
    "intellisense_v2_current_context_parse_ms_role_broker_leader",
    "intellisense_v2_current_context_parse_ms_role_broker_follower",
    "intellisense_v2_current_context_parse_ms_role_other",
    "intellisense_v2_current_context_wall_ms_role_ready_snapshot",
    "intellisense_v2_current_context_wall_ms_role_broker_leader",
    "intellisense_v2_current_context_wall_ms_role_broker_follower",
    "intellisense_v2_current_context_wall_ms_role_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_other",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_open",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_change",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_save",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_other",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_not_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_timeout",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_zero_budget_outcome_other",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_not_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_timeout",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_bounded_wait_outcome_other",
];

const UNIFIED_INTELLISENSE_V2_GAUGE_KEYS: &[&str] = &[
    "intellisense_v2_runtime_saturation_waiters_interactive",
    "intellisense_v2_runtime_saturation_waiters_background",
    "intellisense_v2_runtime_saturation_permits_interactive",
    "intellisense_v2_runtime_saturation_permits_background",
    "intellisense_v2_runtime_saturation_permits_shared",
    "intellisense_v2_runtime_saturation_queue_depth_total",
    "intellisense_v2_completion_owner_hint_index_fetch_active",
];

const ALLOWED_ORIGINS: &[&str] = &["lsp", "web", "agent", "runtime"];
const ALLOWED_OPERATIONS: &[&str] = &[
    "completion",
    "hover",
    "signature_help",
    "definition",
    "document_symbol",
    "rename",
    "diagnostics",
    "members",
    "type_at_position",
    "symbol_search",
    "references",
    "wait_for_file_version",
    "snapshot_with_deps",
    "apply_changes_batch",
    "apply_change_set_file",
    "apply_change_set_file_with_snapshot",
    "apply_change_remove_file",
    "apply_change_set_settings_snapshot",
    "type_index_precompute",
    "type_index_precompute_build",
    "type_index_precompute_ir",
    "type_index_precompute_ast_to_ir",
    "type_index_precompute_semantic_facts",
    "type_index_precompute_semantic_facts_seed_module_context",
    "type_index_precompute_semantic_facts_local_function_summaries",
    "type_index_precompute_semantic_facts_visit_statements",
    "other",
];
const ALLOWED_STAGES: &[&str] = &[
    "runtime_wait_for_file_version",
    "runtime_snapshot_with_deps",
    "runtime_queue_wait",
    "runtime_exec",
    "ir_query",
    "syntax_diagnostics_query",
    "semantic_diagnostics_query",
    "parse_result_query",
];
const ALLOWED_PARSE_MODES: &[&str] = &["incremental", "reused", "full", "other"];
const ALLOWED_OUTCOMES: &[&str] = &[
    "success",
    "empty",
    "cancelled",
    "error",
    "stale_version",
    "missing_deps",
    "leader",
    "shared",
    "key_unavailable",
];
const ALLOWED_REASONS: &[&str] = &["syntax", "semantic", "other", "queue_wait", "exec"];
const ALLOWED_QUERY_KINDS: &[&str] = &["parse_result", "syntax_diagnostics", "ir", "other"];
const ALLOWED_WORK_CLASSES: &[&str] = &["interactive", "background"];
const ALLOWED_COMPLETION_MODES: &[&str] = &["legacy", "event_driven", "shadow"];
const ALLOWED_SATURATION_METRICS: &[&str] = &[
    "waiters_interactive",
    "waiters_background",
    "permits_interactive",
    "permits_background",
    "permits_shared",
    "queue_depth_total",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanonicalFamily {
    StageTotal,
    StageLatencyMs,
    StageReasonTotal,
    SingleflightEffectivenessTotal,
    SaturationSampleTotal,
    SaturationSampleLatencyMs,
    SaturationGauge,
}

impl CanonicalFamily {
    fn as_str(self) -> &'static str {
        match self {
            CanonicalFamily::StageTotal => "stage_total",
            CanonicalFamily::StageLatencyMs => "stage_latency_ms",
            CanonicalFamily::StageReasonTotal => "stage_reason_total",
            CanonicalFamily::SingleflightEffectivenessTotal => "singleflight_effectiveness_total",
            CanonicalFamily::SaturationSampleTotal => "saturation_sample_total",
            CanonicalFamily::SaturationSampleLatencyMs => "saturation_sample_latency_ms",
            CanonicalFamily::SaturationGauge => "saturation_gauge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanonicalValueKind {
    Counter,
    HistogramMs,
    Gauge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LegacyMetricKind {
    Counter,
    HistogramMs,
    Gauge,
}

#[derive(Debug, Clone, Copy)]
struct LegacyMetricTarget<'a> {
    key: &'a str,
    kind: LegacyMetricKind,
}

#[derive(Debug, Clone, Copy)]
struct CanonicalEvent<'a> {
    family: CanonicalFamily,
    origin: &'a str,
    mode: Option<&'a str>,
    operation: Option<&'a str>,
    stage: Option<&'a str>,
    outcome: Option<&'a str>,
    reason: Option<&'a str>,
    query_kind: Option<&'a str>,
    work_class: Option<&'a str>,
    saturation_metric: Option<&'a str>,
    value_kind: CanonicalValueKind,
    value: f64,
    requires_legacy_projection: bool,
}

impl Default for BasicObservability {
    fn default() -> Self {
        let metrics = SimpleMetrics::new();
        register_unified_intellisense_v2_contract_metrics(&metrics);
        Self {
            logger: StructuredLogger::new(),
            metrics,
            start_time: Instant::now(),
        }
    }
}

fn register_unified_intellisense_v2_contract_metrics(metrics: &SimpleMetrics) {
    for key in UNIFIED_INTELLISENSE_V2_COUNTER_KEYS {
        metrics.register_counter(key);
    }
    for key in UNIFIED_INTELLISENSE_V2_HISTOGRAM_KEYS {
        metrics.register_histogram(key);
    }
    for key in UNIFIED_INTELLISENSE_V2_GAUGE_KEYS {
        metrics.register_gauge(key);
    }
}

#[path = "basic_observability/completion_metrics.rs"]
mod completion_metrics;
#[path = "basic_observability/core_metrics.rs"]
mod core_metrics;
#[path = "basic_observability/labels.rs"]
mod labels;
#[path = "basic_observability/metrics_backend.rs"]
mod metrics_backend;
#[path = "basic_observability/query_metrics.rs"]
mod query_metrics;
#[path = "basic_observability/runtime_metrics.rs"]
mod runtime_metrics;

#[cfg(test)]
#[path = "basic_observability/comparison_notes.rs"]
mod comparison_notes;
#[cfg(test)]
#[path = "basic_observability/tests.rs"]
mod observability_contract_tests;
use self::labels::*;

fn compute_rate(
    counters: &HashMap<String, u64>,
    numerator: &str,
    denominator: &str,
) -> Option<f64> {
    let numerator = *counters.get(numerator)? as f64;
    let denominator = *counters.get(denominator)? as f64;
    if denominator == 0.0 {
        return None;
    }
    Some(numerator / denominator)
}

fn sum_counters_with_all_substrings(counters: &HashMap<String, u64>, parts: &[&str]) -> u64 {
    counters
        .iter()
        .filter(|(key, _)| parts.iter().all(|part| key.contains(part)))
        .map(|(_, value)| *value)
        .sum()
}

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let clamped = percentile.clamp(0.0, 1.0);
    let rank = ((values.len() - 1) as f64 * clamped).round() as usize;
    values[rank]
}
