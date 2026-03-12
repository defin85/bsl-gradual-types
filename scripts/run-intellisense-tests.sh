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
  cargo test -p bsl-backend --test flow_sensitive_web_api_test hover_endpoints_emit_type_index_reason_metrics -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test hover_endpoints_fail_closed_on_missing_canonical_artifacts -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test hover_endpoints_do_not_backfill_from_polluted_search_index -- --nocapture
  cargo test -p bsl-backend --test flow_sensitive_web_api_test file_path_for_module_context_bindings -- --nocapture
  cargo test -p bsl-backend --test form_module_object_unified_contract_test diagnostics_hover_and_type_at_position_follow_unified_form_contract -- --nocapture
  cargo test -p bsl-backend --test form_module_object_unified_contract_test completion_and_resolve_follow_unified_form_contract -- --nocapture
  cargo test -p bsl-backend --test form_module_object_unified_contract_test recordset_module_resolves_system_members_and_manager_path_call -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_completion_uses_shared_runtime_snapshot -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_completion_does_not_backfill_from_polluted_search_index -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_inline_type_info_uses_shared_runtime_snapshot -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_type_info_preserves_object_module_binding_facets -- --nocapture
  cargo test -p bsl-cli --bin bsl-cli cli_file_diagnostics_use_shared_runtime_snapshot -- --nocapture
  cargo test -p bsl-agent bsl_members_does_not_execute_parse_result_query_on_semantic_path -- --nocapture
  cargo test -p bsl-agent semantic_mcp_tools_do_not_backfill_from_polluted_search_index_on_default_path -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_semantic_tools_happy_path_uses_current_revision_overlay -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_members_fail_closed_on_current_revision_missing_owner_hint -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_type_at_position_returns_empty_on_current_revision_without_semantic_surface -- --nocapture
  cargo test -p bsl-agent --test stdio_integration stdio_definition_fail_closed_on_current_revision_unresolved_target -- --nocapture
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
