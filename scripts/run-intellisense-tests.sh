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

ensure_embedded_ui_assets() {
  local site_index="${ROOT_DIR}/target/site/index.html"
  if [[ -f "${site_index}" ]]; then
    return 0
  fi

  if ! command -v trunk >/dev/null 2>&1; then
    echo "default smoke path requires embedded bsl-agent UI assets, but trunk is not available." >&2
    echo "Install trunk and build the frontend, e.g.:" >&2
    echo "  cargo install trunk --locked" >&2
    echo "  rustup target add wasm32-unknown-unknown" >&2
    echo "  (cd frontend && NO_COLOR=true trunk build --release)" >&2
    exit 1
  fi

  echo "target/site/index.html is missing; rebuilding embedded bsl-agent UI assets via trunk..."
  (
    cd "${ROOT_DIR}/frontend"
    NO_COLOR=true trunk build --release
  )
}

resolve_cargo_test_binary() {
  local build_output="$1"
  local target_kind="$2"
  local target_name="${3:-}"

  python3 - "${build_output}" "${target_kind}" "${target_name}" <<'PY'
import json
import sys

build_output, target_kind, target_name = sys.argv[1:4]
matches = []

with open(build_output, encoding="utf-8") as handle:
    for raw_line in handle:
        raw_line = raw_line.strip()
        if not raw_line or not raw_line.startswith("{"):
            continue
        try:
            payload = json.loads(raw_line)
        except json.JSONDecodeError:
            continue

        if payload.get("reason") != "compiler-artifact":
            continue

        executable = payload.get("executable")
        if not executable:
            continue

        target = payload.get("target") or {}
        kinds = target.get("kind") or []
        name = target.get("name") or ""

        if target_kind == "lib":
            if "lib" not in kinds:
                continue
        else:
            if target_kind not in kinds or name != target_name:
                continue

        matches.append(executable)

if not matches:
    raise SystemExit(
        f"resolve_cargo_test_binary: no executable matched target_kind={target_kind!r} "
        f"target_name={target_name!r}"
    )

unique_matches = sorted(set(matches))
if len(unique_matches) != 1:
    raise SystemExit(
        "resolve_cargo_test_binary: executable resolution was ambiguous: "
        + ", ".join(unique_matches)
    )

print(unique_matches[0])
PY
}

run_cargo_exact_bundle() {
  local label="$1"
  shift

  local -a cargo_args=()
  while [[ $# -gt 0 && "$1" != "--" ]]; do
    cargo_args+=("$1")
    shift
  done

  if [[ $# -eq 0 ]]; then
    echo "run_cargo_exact_bundle(${label}): missing selector separator" >&2
    exit 1
  fi
  shift

  local -a selectors=("$@")
  if [[ ${#selectors[@]} -eq 0 ]]; then
    echo "run_cargo_exact_bundle(${label}): no selectors provided" >&2
    exit 1
  fi

  echo "[smoke] ${label}: running ${#selectors[@]} exact selectors"
  local target_kind=""
  local target_name=""
  local arg_index=0
  while [[ ${arg_index} -lt ${#cargo_args[@]} ]]; do
    case "${cargo_args[${arg_index}]}" in
      --lib)
        target_kind="lib"
        ;;
      --bin)
        arg_index=$((arg_index + 1))
        if [[ ${arg_index} -ge ${#cargo_args[@]} ]]; then
          echo "run_cargo_exact_bundle(${label}): --bin is missing a target name" >&2
          exit 1
        fi
        target_kind="bin"
        target_name="${cargo_args[${arg_index}]}"
        ;;
      --test)
        arg_index=$((arg_index + 1))
        if [[ ${arg_index} -ge ${#cargo_args[@]} ]]; then
          echo "run_cargo_exact_bundle(${label}): --test is missing a target name" >&2
          exit 1
        fi
        target_kind="test"
        target_name="${cargo_args[${arg_index}]}"
        ;;
    esac
    arg_index=$((arg_index + 1))
  done

  if [[ -z "${target_kind}" ]]; then
    echo "run_cargo_exact_bundle(${label}): unsupported target spec ${cargo_args[*]}" >&2
    exit 1
  fi

  # Build once per bundle, then reuse the emitted test binary for exact selectors.
  local build_output=""
  build_output="$(mktemp)"
  cargo test "${cargo_args[@]}" --no-run --message-format=json >"${build_output}"

  local test_binary=""
  test_binary="$(resolve_cargo_test_binary "${build_output}" "${target_kind}" "${target_name}")"
  rm -f "${build_output}"

  local -a available_tests=()
  mapfile -t available_tests < <("${test_binary}" --list | sed -n 's/: test$//p')
  if [[ ${#available_tests[@]} -eq 0 ]]; then
    echo "run_cargo_exact_bundle(${label}): test binary --list returned no tests" >&2
    exit 1
  fi

  local selector=""
  local available_test=""
  local resolved_selector=""
  local -a matching_tests=()
  for selector in "${selectors[@]}"; do
    matching_tests=()
    for available_test in "${available_tests[@]}"; do
      if [[ "${available_test}" == "${selector}" || "${available_test}" == *::"${selector}" ]]; then
        matching_tests+=("${available_test}")
      fi
    done

    if [[ ${#matching_tests[@]} -eq 0 ]]; then
      echo "run_cargo_exact_bundle(${label}): selector '${selector}' did not resolve via cargo --list" >&2
      exit 1
    fi
    if [[ ${#matching_tests[@]} -ne 1 ]]; then
      echo "run_cargo_exact_bundle(${label}): selector '${selector}' resolved ambiguously:" >&2
      printf '  - %s\n' "${matching_tests[@]}" >&2
      exit 1
    fi

    resolved_selector="${matching_tests[0]}"
    "${test_binary}" "${resolved_selector}" --exact --nocapture
  done
}

run_cross_adapter_smoke() {
  local -a lsp_server_cross_adapter_selectors=(
    "p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics"
    "p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics"
    "p7_hover_and_type_at_position_revision_switch_do_not_report_stale_typed_structure_member"
    "p7_definition_revision_switch_does_not_return_stale_previous_revision_location_across_lsp_and_mcp"
  )
  local -a web_api_selectors=(
    "hover_endpoints_emit_type_index_reason_metrics"
    "hover_endpoints_fail_closed_on_missing_canonical_artifacts"
    "hover_endpoints_do_not_backfill_from_polluted_search_index"
    "diagnostics_and_validate_do_not_backfill_from_polluted_search_index"
    "hover_endpoints_use_file_path_for_module_context_bindings"
    "diagnostics_and_validate_use_file_path_for_module_context_bindings"
  )
  local -a form_contract_selectors=(
    "bare_owner_members_without_canonical_binding_stay_undeclared"
    "diagnostics_hover_and_type_at_position_follow_unified_form_contract"
    "completion_and_resolve_follow_unified_form_contract"
    "recordset_module_resolves_system_members_and_manager_path_call"
  )
  local -a cli_exact_selectors=(
    "cli_inline_completion_uses_shared_runtime_snapshot"
    "cli_inline_completion_preserves_canonical_generic_owner_hint"
    "cli_inline_completion_does_not_backfill_from_polluted_search_index"
    "cli_inline_completion_preserves_object_module_binding_facets"
    "cli_inline_type_info_uses_shared_runtime_snapshot"
    "cli_type_info_preserves_object_module_binding_facets"
    "cli_file_diagnostics_use_shared_runtime_snapshot"
  )
  local -a agent_lib_exact_selectors=(
    "bsl_members_does_not_execute_parse_result_query_on_semantic_path"
    "semantic_mcp_tools_do_not_backfill_from_polluted_search_index_on_default_path"
  )
  local -a stdio_integration_selectors=(
    "stdio_semantic_tools_happy_path_uses_current_revision_overlay"
    "stdio_members_fail_closed_on_current_revision_missing_owner_hint"
    "stdio_type_at_position_returns_empty_on_current_revision_without_semantic_surface"
    "stdio_definition_fail_closed_on_current_revision_unresolved_target"
    "stdio_type_at_position_revision_switch_does_not_return_stale_previous_revision_type"
    "stdio_definition_revision_switch_does_not_return_stale_previous_revision_location"
  )

  # Default-path acceptance slices that exercise shipped runtime wiring across adapters.
  run_cargo_exact_bundle "bsl-lsp-server cross-adapter" -p bsl-backend --bin bsl-lsp-server -- "${lsp_server_cross_adapter_selectors[@]}"
  run_cargo_exact_bundle "flow-sensitive web API" -p bsl-backend --test flow_sensitive_web_api_test -- "${web_api_selectors[@]}"
  run_cargo_exact_bundle "form module unified contract" -p bsl-backend --test form_module_object_unified_contract_test -- "${form_contract_selectors[@]}"
  cargo test -p bsl-backend --test hover_property_access_test test_hover_on_property_name_uses_exact_semantic_index_when_runtime_ir_facts_are_missing -- --nocapture
  cargo test -p bsl-backend --test lsp_intellisense_tests lsp_signature_help_uses_exact_semantic_index_when_runtime_ir_facts_are_missing -- --nocapture
  cargo test -p bsl-backend --test goto_definition_common_module_test goto_definition_uses_exact_semantic_index_when_runtime_ir_facts_are_missing -- --nocapture
  run_cargo_exact_bundle "bsl-cli cross-surface" -p bsl-cli --bin bsl-cli -- "${cli_exact_selectors[@]}"
  run_cargo_exact_bundle "bsl-agent semantic session" -p bsl-agent --lib -- "${agent_lib_exact_selectors[@]}"
  run_cargo_exact_bundle "bsl-agent stdio integration" -p bsl-agent --test stdio_integration -- "${stdio_integration_selectors[@]}"
}

run_document_symbol_isolation_smoke() {
  local -a document_symbol_selectors=(
    "p33_document_symbol_returns_unavailable_before_ready_outline_from_did_open_gap"
    "p33_document_symbol_returns_latest_ready_from_cache_during_parse_gap"
    "p33_document_symbol_supersedes_older_outstanding_refresh"
    "p33_document_symbol_burst_does_not_delay_completion_first_poll_under_parse_gap"
    "p33_document_symbol_burst_does_not_delay_hover_signature_help_or_definition_under_parse_gap"
    "p33_did_save_rearms_same_version_outline_refresh_on_default_path"
  )

  run_cargo_exact_bundle "bsl-lsp-server document-symbol isolation" -p bsl-backend --bin bsl-lsp-server -- "${document_symbol_selectors[@]}"
}

run_completion_timeline_drilldown_smoke() {
  local -a runtime_trace_selectors=(
    "wait_for_file_version_runtime_trace_distinguishes_immediate_and_waiter_paths"
    "snapshot_with_deps_runtime_trace_exposes_queue_and_exec_latency"
    "interactive_wait_budget_timeout_can_still_report_timeout_attribution_on_success"
    "snapshot_with_deps_timeout_can_report_queue_wait_runtime_split_via_progress"
  )
  local -a backend_lib_trace_selectors=(
    "server::completion_dispatcher::tests::turn_waiter_preserves_non_zero_absolute_lifecycle_after_observed_wait"
    "server::language_server::impl_completion::tests::turn_attribution_trace_preserves_turn_wait_resolution_timestamps"
  )
  local -a lsp_server_timeline_selectors=(
    "p22_get_completion_timeline_exposes_versioned_contract"
    "p22_get_completion_timeline_contains_completion_trace"
    "dispatch_context_service_records_completion_context_for_position_lookup"
    "server_edge_details_are_derived_from_transport_handler_and_response_timestamps"
    "server_edge_details_include_pre_method_attribution_provenance"
    "server_edge_details_use_outer_dispatch_timestamp_as_transport_anchor_when_available"
    "request_context_service_records_first_poll_and_first_wake_for_pending_future"
    "request_context_service_does_not_fabricate_first_wake_for_ready_first_poll"
    "server_edge_details_derive_first_poll_and_first_wake_split_when_present"
    "server_edge_details_do_not_fabricate_first_wake_split_when_first_poll_is_ready"
    "prepare_runtime_drilldown_is_serialised_into_trace"
    "prepare_timeout_attribution_is_serialised_into_trace"
    "snapshot_timeout_runtime_is_serialised_into_trace"
    "exact_wait_task_state_drilldown_is_serialised_into_trace"
    "exact_wait_artifact_poll_is_serialised_into_trace"
    "overlapping_completion_request_context_can_be_taken_by_request_id_out_of_order"
    "pre_method_attribution_provenance_stays_fail_closed_for_overlapping_completion"
    "p33_same_file_completion_supersession_releases_pre_active_turn_wait_before_active_registration"
    "p28_cancel_request_releases_pre_active_turn_wait_before_active_registration"
    "p33_same_file_completion_supersession_releases_active_turn_during_response_build"
    "p33_same_file_completion_supersession_releases_active_turn_at_format_checkpoint"
    "p33_same_version_exact_wait_keeps_completed_task_observable_until_cleanup"
    "p33_shutdown_cleans_retained_same_version_exact_task_entry"
    "p33_same_version_invoked_completion_keeps_completed_task_visible_on_default_path"
  )

  run_cargo_exact_bundle "bsl-runtime drilldown trace" -p bsl-runtime --lib -- "${runtime_trace_selectors[@]}"
  run_cargo_exact_bundle "bsl-lsp-server turn-wait drilldown trace" -p bsl-backend --bin bsl-lsp-server -- "${backend_lib_trace_selectors[@]}"
  run_cargo_exact_bundle "bsl-lsp-server completion timeline drilldown" -p bsl-backend --bin bsl-lsp-server -- "${lsp_server_timeline_selectors[@]}"
}

ensure_extension_release_lsp_binary() {
  # compile:fast uses copy-binaries:release:skip-build, so the release server
  # binary must already exist and be current on a fresh CI runner.
  cargo build -p bsl-backend --release --bin bsl-lsp-server
}

run_extension_completion_observability_tests() {
  local grep_pattern="$1"

  (
    export BSL_TEST_GREP="${grep_pattern}"
    node "${ROOT_DIR}/scripts/run-vscode-extension-tests.js"
  )
}

run_extension_completion_observability_smoke() {
  # Focused extension-host slice for completion observability, including
  # request-centric incident bundle summary over authoritative timeline + probes.
  local grep_pattern='Completion Probe (Schema|Recorder|Runtime|Store) Test Suite|Completion Timeline (Clipboard|Drilldown|Model|Webview Provider) Test Suite|Client Options Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found|getObservabilityMetricsFetchResult should preserve unsupported capability until reset|getObservabilityMetricsFetchResult should return unavailable error on timeout'

  ensure_extension_release_lsp_binary
  npm --prefix "${ROOT_DIR}/vscode-extension" run compile:fast
  run_extension_completion_observability_tests "${grep_pattern}"
}

run_smoke() {
  ensure_embedded_ui_assets
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
