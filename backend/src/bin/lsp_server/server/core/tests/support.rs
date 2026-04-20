use super::*;

pub(super) fn init_test_tracing() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    });
}

pub(super) static TEST_ENV_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
    std::sync::OnceLock::new();
pub(super) static PRECOMPUTE_DELAY_ENV_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
    std::sync::OnceLock::new();

pub(super) async fn lock_test_env() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_ENV_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

pub(super) fn lock_test_env_blocking() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_ENV_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .blocking_lock()
}

pub(super) async fn lock_test_env_mutex(
    mutex: &'static std::sync::OnceLock<tokio::sync::Mutex<()>>,
) -> tokio::sync::MutexGuard<'static, ()> {
    mutex
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

pub(super) fn parse_backend_tree_for_test(text: &str) -> Arc<Tree> {
    let mut parser = TreeSitterParser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("tree-sitter-bsl language");
    Arc::new(
        parser
            .parse(text, None)
            .expect("tree-sitter parse for snapshot"),
    )
}

pub(super) fn parse_snapshot_for_test(
    file_id: bsl_analysis_v2::FileId,
    file_version: i32,
    text: &str,
    changed_ranges: Vec<ParseChangedRange>,
    incremental: bool,
    fallback_reason: Option<&str>,
) -> ParseSnapshot {
    ParseSnapshot {
        file_id,
        file_version,
        parse_result: Arc::new(
            bsl_syntax::parse(text, &ParseOptions::default()).expect("snapshot parse"),
        ),
        line_index: Arc::new(LineIndex::new(text)),
        backend_tree: parse_backend_tree_for_test(text),
        changed_ranges: Arc::new(changed_ranges),
        produced_at_millis: 0,
        backend_tree_hash: 0,
        incremental,
        fallback_reason: fallback_reason.map(Arc::from),
    }
}

pub(super) const UNIFIED_STAGE_COUNTER_KEYS: &[&str] = &[
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
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_previous_tree",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_edits_provided",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_other",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_stale_parser_base",
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
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_open_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_change_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_did_save_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_other_reason_other",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_open",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_change",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_save",
    "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_other",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_open_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_open_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_open_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_open_phase_document_symbol_side_work",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_change_phase_document_symbol_side_work",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_save_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_save_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_save_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_did_save_phase_document_symbol_side_work",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_other_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_other_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_other_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_total_origin_lsp_source_other_phase_document_symbol_side_work",
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
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_not_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_timeout",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_relief_valve_outcome_other",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_helped",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_timed_out",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_engaged_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_not_exact_still_current",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_runtime_queue_wait",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_apply_lag",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_timeout_phase_unavailable",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_total_outcome_skipped_timeout_phase_waiting",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_continued_still_current",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_exhausted_continuation_proof",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_other_terminal",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_continuation_total_reason_other",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_pending_publish",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_runtime_queue_wait",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_semantic_work",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_apply_lag",
    "intellisense_v2_diagnostics_save_followup_wait_state_total_reason_other",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_ready_artifacts",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_detached_ready_artifacts",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_shadow_state",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_generic_pipeline",
    "intellisense_v2_diagnostics_save_followup_semantic_path_total_path_other",
];

pub(super) const UNIFIED_STAGE_HISTOGRAM_KEYS: &[&str] = &[
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
    "intellisense_v2_revision_lag_versions",
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
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_open_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_change_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_did_save_reason_other",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_superseded",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_retargeted_before_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_retargeted_during_parse",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_retargeted_before_materialization",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_latest_version_mismatch",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_build_snapshot_aborted",
    "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_other_reason_other",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_open",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_change",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_save",
    "intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_other",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_open_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_open_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_open_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_open_phase_document_symbol_side_work",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_change_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_change_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_change_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_change_phase_document_symbol_side_work",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_save_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_save_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_save_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_did_save_phase_document_symbol_side_work",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_other_phase_parse_exec",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_other_phase_post_parse_pre_materialization",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_other_phase_ready_install",
    "intellisense_v2_ready_parse_snapshot_phase_ms_origin_lsp_source_other_phase_document_symbol_side_work",
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
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_not_ready",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_timeout",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_relief_valve_outcome_other",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_engaged_helped",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_engaged_timed_out",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_engaged_version_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_engaged_generation_mismatch",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_engaged_cancelled",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_engaged_superseded",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_skipped_not_exact_still_current",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_skipped_runtime_queue_wait",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_skipped_apply_lag",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_skipped_timeout_phase_unavailable",
    "intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve_ms_outcome_skipped_timeout_phase_waiting",
];

pub(super) const UNIFIED_STAGE_GAUGE_KEYS: &[&str] = &[
    "intellisense_v2_runtime_saturation_waiters_interactive",
    "intellisense_v2_runtime_saturation_waiters_background",
    "intellisense_v2_runtime_saturation_permits_interactive",
    "intellisense_v2_runtime_saturation_permits_background",
    "intellisense_v2_runtime_saturation_permits_shared",
    "intellisense_v2_runtime_saturation_queue_depth_total",
    "intellisense_v2_completion_owner_hint_index_fetch_active",
];

pub(super) fn assert_unified_intellisense_v2_stage_contract(payload: &serde_json::Value) {
    let metrics = payload.get("metrics").expect("metrics field");
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let gauges = metrics
        .get("gauges")
        .and_then(|value| value.as_object())
        .expect("metrics.gauges object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    for key in UNIFIED_STAGE_COUNTER_KEYS {
        assert!(
            counters.contains_key(*key),
            "missing counter key {key}, got keys={:?}",
            counters.keys().collect::<Vec<_>>()
        );
    }

    for key in UNIFIED_STAGE_HISTOGRAM_KEYS {
        assert!(
            histograms.contains_key(*key),
            "missing histogram key {key}, got keys={:?}",
            histograms.keys().collect::<Vec<_>>()
        );
    }

    for key in UNIFIED_STAGE_GAUGE_KEYS {
        assert!(
            gauges.contains_key(*key),
            "missing gauge key {key}, got keys={:?}",
            gauges.keys().collect::<Vec<_>>()
        );
    }

    assert!(
        !counters.contains_key(
            "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_incremental_failed"
        ),
        "legacy generic incremental_failed fallback bucket must not remain in the exported contract"
    );
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct NormalizedSemanticDiagnostic {
    pub(super) message: String,
    pub(super) severity: String,
    pub(super) start_line: u32,
    pub(super) start_character: u32,
    pub(super) end_line: u32,
    pub(super) end_character: u32,
}

pub(super) async fn initialize_lsp_service(service: &mut LspService<BslLanguageServer>) {
    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let initialize_response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(
        initialize_response.is_some(),
        "initialize should return a response"
    );

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );
}

pub(super) struct LiveLspTransportHarness {
    pub(super) reader: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    pub(super) writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    pub(super) server_task: tokio::task::JoinHandle<()>,
}

impl LiveLspTransportHarness {
    pub(super) async fn send_notification<P>(&mut self, method: &str, params: P)
    where
        P: serde::Serialize,
    {
        self.write_message(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .await;
    }

    pub(super) async fn send_request<P>(
        &mut self,
        id: i64,
        method: &str,
        params: P,
    ) -> serde_json::Value
    where
        P: serde::Serialize,
    {
        self.write_message(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await;
        self.wait_for_response(id).await
    }

    pub(super) async fn shutdown(mut self) {
        drop(self.writer);
        self.server_task.abort();
        let _ = tokio::time::timeout(Duration::from_secs(1), &mut self.server_task).await;
    }

    pub(super) async fn write_message(&mut self, message: &serde_json::Value) {
        let _ = self.write_message_and_record_flush_at_ms(message).await;
    }

    pub(super) async fn write_message_and_record_flush_at_ms(
        &mut self,
        message: &serde_json::Value,
    ) -> u64 {
        let body = serde_json::to_vec(message).expect("serialize LSP transport message");
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.writer
            .write_all(header.as_bytes())
            .await
            .expect("write LSP Content-Length header");
        self.writer
            .write_all(&body)
            .await
            .expect("write LSP message body");
        self.writer.flush().await.expect("flush LSP client stream");
        crate::server::unix_timestamp_ms()
    }

    pub(super) async fn wait_for_response(&mut self, expected_id: i64) -> serde_json::Value {
        tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                let message = self.read_message().await;
                if message.get("method").is_some() {
                    continue;
                }
                if message.get("id").and_then(|value| value.as_i64()) == Some(expected_id) {
                    return message;
                }
            }
        })
        .await
        .expect("timed out waiting for LSP transport response")
    }

    pub(super) async fn read_message(&mut self) -> serde_json::Value {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let bytes = self
                .reader
                .read_line(&mut line)
                .await
                .expect("read LSP header line");
            assert!(bytes > 0, "unexpected EOF while reading LSP header");
            if line == "\r\n" {
                break;
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if let Some(raw_len) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    raw_len
                        .trim()
                        .parse::<usize>()
                        .expect("parse Content-Length header"),
                );
            }
        }
        let body_len = content_length.expect("Content-Length header must be present");
        let mut body = vec![0; body_len];
        self.reader
            .read_exact(&mut body)
            .await
            .expect("read LSP message body");
        serde_json::from_slice(&body).expect("parse framed LSP JSON message")
    }
}

pub(super) async fn spawn_live_lsp_transport_harness(
    coordinator: Arc<SystemCoordinator>,
) -> (LiveLspTransportHarness, BslLanguageServer) {
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");
    let service = crate::server::request_context::DispatchContextService::new(
        crate::server::request_context::RequestContextService::new(service),
    );
    let (client_stream, server_stream) = tokio::io::duplex(1024 * 1024);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (server_read, server_write) = tokio::io::split(server_stream);
    let server_task = tokio::spawn(async move {
        crate::server::serve_with_completion_handoff(
            server_read,
            server_write,
            socket,
            service,
            crate::DEFAULT_LSP_TRANSPORT_CONCURRENCY_LEVEL,
        )
        .await;
    });
    (
        LiveLspTransportHarness {
            reader: BufReader::new(client_read),
            writer: client_write,
            server_task,
        },
        server,
    )
}

pub(super) async fn initialize_live_lsp_transport(harness: &mut LiveLspTransportHarness) {
    let initialize_response = harness
        .send_request(
            1,
            "initialize",
            InitializeParams {
                capabilities: ClientCapabilities::default(),
                ..Default::default()
            },
        )
        .await;
    assert!(
        initialize_response.get("result").is_some(),
        "initialize should return a response"
    );
    harness
        .send_notification("initialized", InitializedParams {})
        .await;
}

pub(super) async fn live_transport_append_text_change(
    harness: &mut LiveLspTransportHarness,
    uri: &Url,
    current_text: &str,
    version: i32,
    appended_text: &str,
) {
    let end_position = utf16_end_position(current_text);
    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: end_position,
                        end: end_position,
                    }),
                    range_length: None,
                    text: appended_text.to_string(),
                }],
            },
        )
        .await;
}

pub(super) async fn live_transport_ranged_did_change(
    harness: &mut LiveLspTransportHarness,
    uri: &Url,
    version: i32,
    content_changes: Vec<TextDocumentContentChangeEvent>,
) {
    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes,
            },
        )
        .await;
}

pub(super) async fn live_transport_close_document(
    harness: &mut LiveLspTransportHarness,
    uri: &Url,
) {
    harness
        .send_notification(
            "textDocument/didClose",
            DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            },
        )
        .await;
}

pub(super) async fn live_transport_save_document(harness: &mut LiveLspTransportHarness, uri: &Url) {
    harness
        .send_notification(
            "textDocument/didSave",
            DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                text: None,
            },
        )
        .await;
}

pub(super) async fn live_transport_completion_labels_with_request(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    uri: &Url,
    position: Position,
    context: Option<CompletionContext>,
) -> Vec<String> {
    let completion_response = harness
        .send_request(
            request_id,
            "textDocument/completion",
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context,
            },
        )
        .await;
    let completion_result = completion_response
        .get("result")
        .cloned()
        .expect("completion result field");
    let completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");

    normalize_lsp_member_labels(&completion.expect("completion result present"))
}

pub(super) async fn live_transport_completion_response_with_request(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    uri: &Url,
    position: Position,
    context: Option<CompletionContext>,
) -> serde_json::Value {
    harness
        .send_request(
            request_id,
            "textDocument/completion",
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context,
            },
        )
        .await
}

pub(super) async fn live_transport_write_document_symbol_request(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    uri: &Url,
) {
    harness
        .write_message(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "textDocument/documentSymbol",
            "params": DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        }))
        .await;
}

pub(super) async fn live_transport_write_completion_request(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    uri: &Url,
    position: Position,
    context: Option<CompletionContext>,
) -> u64 {
    harness
        .write_message_and_record_flush_at_ms(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "textDocument/completion",
            "params": CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context,
            },
        }))
        .await
}

pub(super) async fn live_transport_write_execute_command_request(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    command: &str,
    arguments: Vec<serde_json::Value>,
) -> u64 {
    harness
        .write_message_and_record_flush_at_ms(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "workspace/executeCommand",
            "params": {
                "command": command,
                "arguments": arguments,
            },
        }))
        .await
}

pub(super) async fn live_transport_get_completion_timeline(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    limit: usize,
) -> serde_json::Value {
    let execute_response = harness
        .send_request(
            request_id,
            "workspace/executeCommand",
            serde_json::json!({
                "command": "bsl.getCompletionTimeline",
                "arguments": [{ "limit": limit }],
            }),
        )
        .await;
    execute_response
        .get("result")
        .cloned()
        .expect("result field")
}

pub(super) async fn take_test_request_server_edge_trace(
    request_id: i64,
) -> crate::server::request_context::TestRequestServerEdgeTrace {
    tokio::time::timeout(Duration::from_secs(5), async move {
        loop {
            if let Some(trace) =
                crate::server::request_context::take_request_server_edge_trace_for_testing(
                    &request_id.to_string(),
                )
            {
                break trace;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("test request server-edge trace must appear")
}

pub(super) fn assert_request_first_poll_budget(
    trace: &crate::server::request_context::TestRequestServerEdgeTrace,
    expected_method: &str,
    budget_ms: u64,
) {
    assert_eq!(
        trace.method, expected_method,
        "unexpected method in test request trace: {trace:?}"
    );
    let first_poll_wait_ms = trace
        .server_edge_details
        .service_future_to_first_poll_wait_ms
        .expect("service_future_to_first_poll_wait_ms");
    assert!(
        first_poll_wait_ms <= budget_ms,
        "{expected_method} must keep first poll within budget under outline burst, trace={trace:?}"
    );
    assert!(
        !trace.uri.is_empty(),
        "{expected_method} trace must retain URI for debugging, trace={trace:?}"
    );
    assert_eq!(
        trace.server_edge_details.pre_method_attribution_provenance, "same_request_authoritative",
        "{expected_method} must expose authoritative pre-method attribution on live path, trace={trace:?}"
    );
}

pub(super) async fn live_transport_get_observability_metrics(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
) -> serde_json::Value {
    live_transport_get_observability_metrics_response(harness, request_id)
        .await
        .get("metrics")
        .cloned()
        .expect("result.metrics field")
}

pub(super) async fn live_transport_get_observability_metrics_response(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
) -> serde_json::Value {
    let execute_response = harness
        .send_request(
            request_id,
            "workspace/executeCommand",
            serde_json::json!({
                "command": "bsl.getObservabilityMetrics",
                "arguments": [],
            }),
        )
        .await;
    execute_response
        .get("result")
        .cloned()
        .expect("result field")
}

pub(super) async fn live_transport_get_diagnostics_save_timeline(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    limit: usize,
) -> serde_json::Value {
    let execute_response = harness
        .send_request(
            request_id,
            "workspace/executeCommand",
            serde_json::json!({
                "command": "bsl.getDiagnosticsSaveTimeline",
                "arguments": [{ "limit": limit }],
            }),
        )
        .await;
    execute_response
        .get("result")
        .cloned()
        .expect("result field")
}

pub(super) async fn live_transport_wait_publish_diagnostics(
    harness: &mut LiveLspTransportHarness,
    uri: &Url,
    version: i32,
    timeout_duration: Duration,
) -> PublishDiagnosticsParams {
    tokio::time::timeout(timeout_duration, async {
        loop {
            let message = harness.read_message().await;
            if message.get("method").and_then(|value| value.as_str())
                != Some("textDocument/publishDiagnostics")
            {
                continue;
            }
            let Some(params) = message.get("params").cloned() else {
                continue;
            };
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            if parsed.uri == *uri && parsed.version == Some(version) {
                break parsed;
            }
        }
    })
    .await
    .expect("timed out waiting for live publishDiagnostics")
}

pub(super) async fn shutdown_lsp_service(
    service: &mut LspService<BslLanguageServer>,
    close_uri: Option<&Url>,
) {
    if let Some(uri) = close_uri {
        let did_close = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        };
        let did_close_req = Request::build("textDocument/didClose")
            .params(serde_json::to_value(did_close).expect("DidCloseTextDocumentParams"))
            .finish();
        let did_close_response = service
            .ready()
            .await
            .unwrap()
            .call(did_close_req)
            .await
            .expect("didClose notification");
        assert!(did_close_response.is_none(), "didClose is a notification");
    }

    let shutdown_req = Request::build("shutdown").id(2).finish();
    let shutdown_response = service
        .ready()
        .await
        .unwrap()
        .call(shutdown_req)
        .await
        .expect("shutdown request");
    assert!(
        shutdown_response.is_some(),
        "shutdown should return a response"
    );

    let exit_req = Request::build("exit").finish();
    let exit_response = service
        .ready()
        .await
        .unwrap()
        .call(exit_req)
        .await
        .expect("exit notification");
    assert!(exit_response.is_none(), "exit is a notification");
}
