#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"
PROFILE="${1:-smoke}"

report_m8() {
  local doc="${ROOT_DIR}/docs/roadmap/intellisense-v2-roadmap/m8-implementation-plan.md"
  local golden="${ROOT_DIR}/backend/tests/golden/m8_completion_matrix_v2.json"

  local doc_cases=""
  doc_cases="$(rg -c --no-filename '^\\| m8_' "${doc}" 2>/dev/null || true)"
  if [[ -n "${doc_cases}" ]]; then
    echo "M8: matrix cases in doc: ${doc_cases}"
  fi

  if [[ -f "${golden}" ]]; then
    local golden_cases=""
    golden_cases="$(rg -c --no-filename '\"case\":' "${golden}" 2>/dev/null || true)"
    if [[ -n "${golden_cases}" ]]; then
      echo "M8: matrix cases in golden: ${golden_cases}"
    fi
  fi
}

run_cross_adapter_smoke() {
  # Default-path acceptance slices that exercise shipped runtime wiring across adapters.
  cargo test -p bsl-backend --bin bsl-lsp-server p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p7_hover_and_type_at_position_revision_switch_do_not_report_stale_typed_structure_member -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p7_definition_revision_switch_does_not_return_stale_previous_revision_location_across_lsp_and_mcp -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test hover_endpoints_emit_type_index_reason_metrics -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test hover_endpoints_fail_closed_on_missing_canonical_artifacts -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test hover_endpoints_do_not_backfill_from_polluted_search_index -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test diagnostics_and_validate_do_not_backfill_from_polluted_search_index -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test file_path_for_module_context_bindings -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test hover_endpoints_use_file_path_for_module_context_bindings -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test diagnostics_and_validate_use_file_path_for_module_context_bindings -- --nocapture
  cargo test -p bsl-backend --test form_module_object_unified_contract_test bare_owner_members_without_canonical_binding_stay_undeclared -- --nocapture
  cargo test -p bsl-backend --test form_module_object_unified_contract_test diagnostics_hover_and_type_at_position_follow_unified_form_contract -- --nocapture
  cargo test -p bsl-backend --test form_module_object_unified_contract_test completion_and_resolve_follow_unified_form_contract -- --nocapture
  cargo test -p bsl-backend --test form_module_object_unified_contract_test recordset_module_resolves_system_members_and_manager_path_call -- --nocapture
  cargo test -p bsl-backend --test hover_property_access_test test_hover_on_property_name_uses_exact_semantic_index_when_runtime_ir_facts_are_missing -- --nocapture
  cargo test -p bsl-backend --test lsp_intellisense_tests lsp_signature_help_uses_exact_semantic_index_when_runtime_ir_facts_are_missing -- --nocapture
  cargo test -p bsl-backend --test goto_definition_common_module_test goto_definition_uses_exact_semantic_index_when_runtime_ir_facts_are_missing -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_completion_uses_shared_runtime_snapshot -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_completion_preserves_canonical_generic_owner_hint -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_completion_does_not_backfill_from_polluted_search_index -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_completion_preserves_object_module_binding_facets -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_type_info_uses_shared_runtime_snapshot -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_type_info_preserves_object_module_binding_facets -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_file_diagnostics_use_shared_runtime_snapshot -- --nocapture
  cargo test -p bsl-agent bsl_members_does_not_execute_parse_result_query_on_semantic_path -- --nocapture
  cargo test -p bsl-agent semantic_mcp_tools_do_not_backfill_from_polluted_search_index_on_default_path -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_semantic_tools_happy_path_uses_current_revision_overlay -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_members_fail_closed_on_current_revision_missing_owner_hint -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_type_at_position_returns_empty_on_current_revision_without_semantic_surface -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_definition_fail_closed_on_current_revision_unresolved_target -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_type_at_position_revision_switch_does_not_return_stale_previous_revision_type -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_definition_revision_switch_does_not_return_stale_previous_revision_location -- --nocapture
}

run_document_symbol_isolation_smoke() {
  cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_returns_unavailable_before_ready_outline_from_did_open_gap -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_returns_latest_ready_from_cache_during_parse_gap -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_supersedes_older_outstanding_refresh -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p33_document_symbol_burst_does_not_delay_hover_signature_help_or_definition_under_parse_gap -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p33_did_save_rearms_same_version_outline_refresh_on_default_path -- --nocapture
}

run_completion_timeline_drilldown_smoke() {
  cargo test -p bsl-runtime wait_for_file_version_runtime_trace_distinguishes_immediate_and_waiter_paths -- --nocapture
  cargo test -p bsl-runtime snapshot_with_deps_runtime_trace_exposes_queue_and_exec_latency -- --nocapture
  cargo test -p bsl-runtime interactive_wait_budget_timeout_can_still_report_timeout_attribution_on_success -- --nocapture
  cargo test -p bsl-runtime snapshot_with_deps_timeout_can_report_queue_wait_runtime_split_via_progress -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p22_get_completion_timeline_exposes_versioned_contract -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p22_get_completion_timeline_contains_completion_trace -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server dispatch_context_service_records_completion_context_for_position_lookup -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server server_edge_details_are_derived_from_transport_handler_and_response_timestamps -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server server_edge_details_include_pre_method_attribution_provenance -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server server_edge_details_use_outer_dispatch_timestamp_as_transport_anchor_when_available -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server request_context_service_records_first_poll_and_first_wake_for_pending_future -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server request_context_service_does_not_fabricate_first_wake_for_ready_first_poll -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server server_edge_details_derive_first_poll_and_first_wake_split_when_present -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server server_edge_details_do_not_fabricate_first_wake_split_when_first_poll_is_ready -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server prepare_runtime_drilldown_is_serialised_into_trace -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server prepare_timeout_attribution_is_serialised_into_trace -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server snapshot_timeout_runtime_is_serialised_into_trace -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server exact_wait_task_state_drilldown_is_serialised_into_trace -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server exact_wait_artifact_poll_is_serialised_into_trace -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server pre_method_attribution_provenance_stays_fail_closed_for_overlapping_completion -- --nocapture
  cargo test -p bsl-backend --bin bsl-lsp-server p33_same_file_completion_supersession_releases_active_turn_during_response_build -- --nocapture
}

run_extension_completion_observability_smoke() {
  # Focused extension-host slice for completion observability, including
  # request-centric incident bundle summary over authoritative timeline + probes.
  local grep_pattern='Completion Probe (Schema|Recorder|Runtime|Store) Test Suite|Completion Timeline (Clipboard|Drilldown|Model|Webview Provider) Test Suite|Client Options Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found|getObservabilityMetricsFetchResult should preserve unsupported capability until reset|getObservabilityMetricsFetchResult should return unavailable error on timeout'

  npm --prefix "${ROOT_DIR}/vscode-extension" run compile:fast
  (
    cd "${ROOT_DIR}/vscode-extension"
    BSL_TEST_GREP="${grep_pattern}" node ./out/test/runTest.js
  )
}

run_smoke() {
  cargo test -p bsl-backend --lib completion_ranking
  cargo test -p bsl-backend --lib completion_service
  cargo test -p bsl-backend --test intellisense_testkit_smoke_test
  cargo test -p bsl-backend --test intellisense_golden_completion_test
  cargo test -p bsl-backend --test lsp_intellisense_tests
  cargo test -p bsl-backend --test m8_completion_matrix_golden_v2_test
  cargo test -p bsl-backend --test lsp_incremental_completion_test
  run_cross_adapter_smoke
  run_document_symbol_isolation_smoke
  run_completion_timeline_drilldown_smoke
  run_extension_completion_observability_smoke
  report_m8
}

run_full() {
  run_smoke
  # Дополнительные интеграционные тесты, которые используют репозиторные фикстуры
  # (Syntax Helper + fixture конфигурации).
  cargo test -p bsl-backend --test metadata_completion_fixture_test
  cargo test -p bsl-backend --test property_type_inference_real_data_test
}

case "${PROFILE}" in
  smoke)
    run_smoke
    ;;
  full)
    run_full
    ;;
  *)
    echo "Usage: $(basename "$0") [smoke|full]" >&2
    exit 1
    ;;
esac
