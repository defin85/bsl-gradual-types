#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/backend/tests/perf/reports"
CHANGE_ID="${CHANGE_ID:-refactor-completion-front-edge-readiness-window}"
READINESS_REPORT="${REPORT_DIR}/${CHANGE_ID}-readiness-gate.json"
READINESS_SUMMARY="${REPORT_DIR}/${CHANGE_ID}-readiness-gate.md"
OPENSPEC_LOG="${REPORT_DIR}/${CHANGE_ID}-openspec-validate.log"
OPENSPEC_VALIDATE_SCOPE="change"
if [[ -z "${PERF_PROFILES:-}" ]]; then
  if [[ "${CHANGE_ID}" == "refactor-completion-front-edge-exact-deadline-removal" ]]; then
    PERF_PROFILES="large churn"
  else
    PERF_PROFILES="small large churn"
  fi
fi
if [[ -z "${REAL_MODULE_PROFILES:-}" ]]; then
  if [[ "${CHANGE_ID}" == "refactor-completion-prepare-lightweight-exact-split" ]]; then
    REAL_MODULE_PROFILES="warm churn"
  elif [[ "${CHANGE_ID}" == "refactor-completion-superseded-active-turn-release" ]]; then
    REAL_MODULE_PROFILES="churn overlap"
  elif [[ "${CHANGE_ID}" == "refactor-completion-turn-wait-slot-release" ]]; then
    REAL_MODULE_PROFILES="churn preactive_overlap"
  elif [[ "${CHANGE_ID}" == "refactor-completion-turn-wait-lifecycle" ]]; then
    REAL_MODULE_PROFILES="churn preactive_overlap"
  elif [[ "${CHANGE_ID}" == "refactor-document-symbol-interactive-isolation" ]]; then
    REAL_MODULE_PROFILES="outline"
  elif [[ "${CHANGE_ID}" == "isolate-completion-pre-dispatch-ingress" ]]; then
    REAL_MODULE_PROFILES="outline"
  elif [[ "${CHANGE_ID}" == "refactor-completion-front-edge-exact-deadline-removal" ]]; then
    REAL_MODULE_PROFILES="front_edge"
  elif [[ "${CHANGE_ID}" == "refactor-completion-front-edge-readiness-window" ]]; then
    REAL_MODULE_PROFILES="front_edge"
  else
    REAL_MODULE_PROFILES="churn"
  fi
fi

mkdir -p "${REPORT_DIR}"

if ! command -v openspec >/dev/null 2>&1; then
  echo "openspec CLI is required for strict change validation (command not found)." >&2
  exit 1
fi

active_change_dir="${ROOT_DIR}/openspec/changes/${CHANGE_ID}"
archive_glob=("${ROOT_DIR}"/openspec/changes/archive/*-"${CHANGE_ID}")
if [[ ! -d "${active_change_dir}" ]]; then
  archive_matches=()
  for candidate in "${archive_glob[@]}"; do
    if [[ -d "${candidate}" ]]; then
      archive_matches+=("${candidate}")
    fi
  done
  if (( ${#archive_matches[@]} > 1 )); then
    printf 'Ambiguous archived OpenSpec change for %s:\n' "${CHANGE_ID}" >&2
    printf '  %s\n' "${archive_matches[@]}" >&2
    exit 1
  fi
  if (( ${#archive_matches[@]} == 1 )); then
    OPENSPEC_VALIDATE_SCOPE="all"
  fi
fi

echo "[gate] Running shipped cross-adapter smoke..."
python3 -m unittest \
  scripts/test-intellisense-smoke-gate.py \
  scripts/test-intellisense-readiness-assets.py
"${ROOT_DIR}/scripts/run-intellisense-tests.sh" smoke

echo "[gate] Running authoritative representative-matrix perf gate..."
CHANGE_ID="${CHANGE_ID}" \
BSL_V2_PERF_GATE_BLOCKING=1 \
PERF_PROFILES="${PERF_PROFILES}" \
  "${ROOT_DIR}/scripts/run-intellisense-perf.sh"

real_module_specs=()
real_module_artifacts=()
for profile in ${REAL_MODULE_PROFILES}; do
  case "${profile}" in
    warm)
      profile_title="same-revision warm"
      test_name="p37_real_conf_big_warm_cache_completion_perf_report_live"
      report_var="BSL_V2_REAL_CONF_BIG_WARM_CACHE_COMPLETION_PERF_REPORT"
      report_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-warm-cache-completion-perf-live.json"
      summary_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-warm-cache-completion-perf-live.md"
      ;;
    churn)
      profile_title="revision-churn/post-handoff readiness"
      test_name="p38_real_conf_big_revision_churn_completion_perf_report_live"
      report_var="BSL_V2_REAL_CONF_BIG_REVISION_CHURN_COMPLETION_PERF_REPORT"
      report_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.json"
      summary_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.md"
      ;;
    outline)
      profile_title="documentSymbol mixed-load isolation"
      test_name="p39_real_conf_big_document_symbol_mixed_load_gate_live"
      report_var="BSL_V2_REAL_CONF_BIG_DOCUMENT_SYMBOL_MIXED_LOAD_REPORT"
      report_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-document-symbol-mixed-load-live.json"
      summary_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-document-symbol-mixed-load-live.md"
      ;;
    overlap)
      profile_title="same-file overlap supersession"
      test_name="p40_real_conf_big_same_file_overlap_completion_perf_report_live"
      report_var="BSL_V2_REAL_CONF_BIG_OVERLAP_COMPLETION_PERF_REPORT"
      report_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-overlap-completion-perf-live.json"
      summary_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-overlap-completion-perf-live.md"
      ;;
    preactive_overlap)
      profile_title="same-file pre-active turn_wait overlap"
      test_name="p41_real_conf_big_pre_active_turn_wait_overlap_completion_perf_report_live"
      report_var="BSL_V2_REAL_CONF_BIG_PRE_ACTIVE_OVERLAP_COMPLETION_PERF_REPORT"
      report_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-pre-active-overlap-completion-perf-live.json"
      summary_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-pre-active-overlap-completion-perf-live.md"
      ;;
    front_edge)
      profile_title="same-file front-edge readiness window"
      test_name="p42_real_conf_big_front_edge_completion_perf_report_live"
      report_var="BSL_V2_REAL_CONF_BIG_FRONT_EDGE_COMPLETION_PERF_REPORT"
      report_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-front-edge-completion-perf-live.json"
      summary_path="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-front-edge-completion-perf-live.md"
      ;;
    *)
      echo "Unsupported REAL_MODULE_PROFILES entry: ${profile}" >&2
      exit 1
      ;;
  esac

  echo "[gate] Running real-module ${profile_title} gate..."
  env CHANGE_ID="${CHANGE_ID}" "${report_var}=${report_path}" \
    cargo test -p bsl-backend --bin bsl-lsp-server "${test_name}" -- --nocapture
  real_module_specs+=("${profile}::${profile_title}::${test_name}::${report_path}::${summary_path}")
  real_module_artifacts+=("${report_path}" "${summary_path}")
done

echo "[gate] Running OpenSpec strict validation..."
if [[ "${OPENSPEC_VALIDATE_SCOPE}" == "all" ]]; then
  openspec validate --all --strict --no-interactive \
    | tee "${OPENSPEC_LOG}"
else
  openspec validate "${CHANGE_ID}" --strict --no-interactive \
    | tee "${OPENSPEC_LOG}"
fi

python3 - "${REPORT_DIR}" "${READINESS_REPORT}" "${READINESS_SUMMARY}" "${OPENSPEC_LOG}" "${CHANGE_ID}" "${PERF_PROFILES}" "${REAL_MODULE_PROFILES}" "${OPENSPEC_VALIDATE_SCOPE}" "${real_module_specs[@]}" <<'PY'
import json
import pathlib
import sys

report_dir = pathlib.Path(sys.argv[1])
aggregate_path = pathlib.Path(sys.argv[2])
summary_path = pathlib.Path(sys.argv[3])
openspec_log_path = pathlib.Path(sys.argv[4])
expected_change_id = sys.argv[5]
perf_profiles = sys.argv[6].split()
real_module_profiles = sys.argv[7].split()
openspec_validate_scope = sys.argv[8]
real_module_specs = {}
for raw_spec in sys.argv[9:]:
    profile, profile_title, test_name, report_path, profile_summary_path = raw_spec.split("::", 4)
    real_module_specs[profile] = {
        "title": profile_title,
        "test_name": test_name,
        "report_path": pathlib.Path(report_path),
        "summary_path": pathlib.Path(profile_summary_path),
    }

repo_root = report_dir.parents[3]
backend_root = report_dir.parents[2]
profile_rows = []
real_module_rows = []
aggregate = {
    "change_id": expected_change_id,
    "authoritative_perf_gate": True,
    "cross_adapter_smoke": {
        "command": "./scripts/run-intellisense-tests.sh smoke",
        "pass": True,
    },
    "perf_gate": {
        "command": f"CHANGE_ID={expected_change_id} BSL_V2_PERF_GATE_BLOCKING=1 ./scripts/run-intellisense-perf.sh",
        "profiles": {},
    },
    "real_module_gates": {},
    "openspec_validation": {
        "command": (
            "openspec validate --all --strict --no-interactive"
            if openspec_validate_scope == "all"
            else f"openspec validate {expected_change_id} --strict --no-interactive"
        ),
        "log": str(openspec_log_path.relative_to(repo_root)),
        "pass": True,
    },
}
overall_pass = True

for profile in perf_profiles:
    report_path = report_dir / f"intellisense_{profile}.json"
    data = json.loads(report_path.read_text(encoding="utf-8"))
    comparison = data.get("comparison") or {}
    entries = comparison.get("entries", [])
    failing_entries = [entry for entry in entries if not entry.get("pass", False)]
    fail_closed_failures = [
        entry
        for entry in failing_entries
        if "fail_closed_budget_exceeded" in entry.get("reason_codes", [])
    ]
    pass_flag = bool(data.get("pass")) and bool(comparison.get("pass", True))
    overall_pass = overall_pass and pass_flag
    aggregate["perf_gate"]["profiles"][profile] = {
        "report": str(report_path.relative_to(backend_root)),
        "pass": pass_flag,
        "verdict": comparison.get("verdict", data.get("verdict", "fail")),
        "reason_codes": comparison.get("reason_codes", data.get("reason_codes", [])),
        "reported_matrix_entries": data.get("coverage", {}).get("reported_matrix_entries", 0),
        "failing_entries": len(failing_entries),
        "fail_closed_budget_failures": len(fail_closed_failures),
    }
    profile_rows.append(
        (
            profile,
            "yes" if pass_flag else "no",
            aggregate["perf_gate"]["profiles"][profile]["reported_matrix_entries"],
            len(failing_entries),
            len(fail_closed_failures),
            ", ".join(aggregate["perf_gate"]["profiles"][profile]["reason_codes"]) or "-",
        )
    )

for profile in real_module_profiles:
    spec = real_module_specs[profile]
    report_path = spec["report_path"]
    data = json.loads(report_path.read_text(encoding="utf-8"))
    summary = data.get("summary") or {}
    report_change_id = data.get("change_id", "-")
    pass_flag = report_change_id == expected_change_id
    overall_pass = overall_pass and pass_flag
    aggregate["real_module_gates"][profile] = {
        "profile_title": spec["title"],
        "test_name": spec["test_name"],
        "report": str(report_path.relative_to(backend_root)),
        "summary": str(spec["summary_path"].relative_to(backend_root)),
        "pass": pass_flag,
        "report_change_id": report_change_id,
        "measured_samples": len(data.get("measured_samples") or [])
        or summary.get("measured_requests")
        or summary.get("measured_trace_linked_samples")
        or summary.get("measured_non_empty_samples", 0),
        "successful_traces": summary.get("measured_successful_traces", 0),
        "fail_closed_traces": summary.get("measured_fail_closed_traces", 0),
        "head_hit_traces": summary.get("measured_head_hit_traces", 0),
        "exact_hit_traces": summary.get("measured_exact_hit_traces", 0),
        "prepare_timeout_delta": summary.get("measured_prepare_timeout_total_delta", 0),
        "exact_deadline_delta": summary.get("measured_exact_deadline_total_delta", 0),
        "prepare_timeout_wait_for_file_version_samples": summary.get(
            "measured_prepare_timeout_wait_for_file_version_samples", 0
        ),
        "prepare_timeout_snapshot_with_deps_samples": summary.get(
            "measured_prepare_timeout_snapshot_with_deps_samples", 0
        ),
        "cold_query_bundle_pool_wait_samples": summary.get(
            "measured_cold_query_bundle_pool_wait_samples", 0
        ),
    }
    real_module_rows.append(
        (
            spec["title"],
            "yes" if pass_flag else "no",
            report_change_id,
            aggregate["real_module_gates"][profile]["measured_samples"],
            aggregate["real_module_gates"][profile]["successful_traces"],
            aggregate["real_module_gates"][profile]["fail_closed_traces"],
            aggregate["real_module_gates"][profile]["head_hit_traces"],
            aggregate["real_module_gates"][profile]["exact_hit_traces"],
            aggregate["real_module_gates"][profile]["prepare_timeout_delta"],
            aggregate["real_module_gates"][profile]["exact_deadline_delta"],
            aggregate["real_module_gates"][profile]["cold_query_bundle_pool_wait_samples"],
        )
    )
    profile_lines = [
        f"# {expected_change_id} real-module readiness gate",
        "",
        f"- profile: `{spec['test_name']}`",
        f"- profile title: `{spec['title']}`",
        f"- report: `{report_path}`",
        f"- report change_id: `{report_change_id}`",
        f"- measured samples: `{aggregate['real_module_gates'][profile]['measured_samples']}`",
        f"- successful traces: `{aggregate['real_module_gates'][profile]['successful_traces']}`",
        f"- fail_closed traces: `{aggregate['real_module_gates'][profile]['fail_closed_traces']}`",
        f"- head_hit traces: `{aggregate['real_module_gates'][profile]['head_hit_traces']}`",
        f"- exact_hit traces: `{aggregate['real_module_gates'][profile]['exact_hit_traces']}`",
        f"- prepare_timeout delta: `{aggregate['real_module_gates'][profile]['prepare_timeout_delta']}`",
        f"- exact_deadline delta: `{aggregate['real_module_gates'][profile]['exact_deadline_delta']}`",
    ]
    if "measured_document_symbol_latest_ready_total_delta" in summary:
        profile_lines.extend(
            [
                f"- documentSymbol latest_ready delta: `{summary.get('measured_document_symbol_latest_ready_total_delta', 0)}`",
                f"- documentSymbol current_ready delta: `{summary.get('measured_document_symbol_current_ready_total_delta', 0)}`",
                f"- documentSymbol unavailable delta: `{summary.get('measured_document_symbol_unavailable_total_delta', 0)}`",
                f"- documentSymbol superseded delta: `{summary.get('measured_document_symbol_superseded_total_delta', 0)}`",
                f"- documentSymbol present responses: `{summary.get('measured_document_symbol_present_responses_total', 0)}`",
                f"- documentSymbol null responses: `{summary.get('measured_document_symbol_null_responses_total', 0)}`",
                f"- legacy ingress-regression samples: `{summary.get('measured_ingress_regression_samples', 0)}`",
            ]
        )
    if "measured_adapter_to_dispatch_wait_ms" in summary:
        profile_lines.extend(
            [
                (
                    "- pre-dispatch samples over budget: "
                    f"`{summary.get('measured_pre_dispatch_wait_over_budget_samples', 0)}`"
                ),
                (
                    "- pre-dispatch samples over hard cap: "
                    f"`{summary.get('measured_pre_dispatch_wait_over_hard_cap_samples', 0)}`"
                ),
                (
                    "`p95(adapter_to_dispatch_wait_ms)="
                    f"{summary.get('measured_adapter_to_dispatch_wait_ms', {}).get('p95', 0):g}ms`"
                ),
                (
                    "`max(adapter_to_dispatch_wait_ms)="
                    f"{summary.get('measured_adapter_to_dispatch_wait_max_ms', 0)}ms`"
                ),
                (
                    "`p95(service_future_to_first_poll_wait_ms)="
                    f"{summary.get('measured_service_future_to_first_poll_wait_ms', {}).get('p95', 0):g}ms`"
                ),
                (
                    "`max(service_future_to_first_poll_wait_ms)="
                    f"{summary.get('measured_service_future_to_first_poll_wait_max_ms', 0)}ms`"
                ),
                (
                    "`p95(transport_to_handler_wait_ms)="
                    f"{summary.get('measured_transport_to_handler_wait_ms', {}).get('p95', 0):g}ms`"
                ),
                (
                    "`max(transport_to_handler_wait_ms)="
                    f"{summary.get('measured_transport_to_handler_wait_max_ms', 0)}ms`"
                ),
            ]
        )
    if "measured_first_cancelled_or_superseded_traces" in summary:
        profile_lines.extend(
            [
                f"- first cancelled/superseded traces: `{summary.get('measured_first_cancelled_or_superseded_traces', 0)}`",
                f"- first empty responses: `{summary.get('measured_first_empty_response_samples', 0)}`",
                f"- first registry cleared: `{summary.get('measured_first_registry_cleared_samples', 0)}`",
                f"- second non-empty responses: `{summary.get('measured_second_non_empty_samples', 0)}`",
                (
                    "`p95(service_future_to_first_poll_wait_ms)="
                    f"{summary.get('measured_service_future_to_first_poll_wait_ms', {}).get('p95', 0):g}ms`"
                ),
                (
                    "`max(service_future_to_first_poll_wait_ms)="
                    f"{summary.get('measured_service_future_to_first_poll_wait_max_ms', 0)}ms`"
                ),
            ]
        )
    if "measured_first_pre_active_turn_wait_ready_traces" in summary:
        profile_lines.extend(
            [
                (
                    "- first pre-active `turn_wait` ready traces: "
                    f"`{summary.get('measured_first_pre_active_turn_wait_ready_traces', 0)}`"
                ),
                (
                    "- stranded pre-active `turn_wait` samples: "
                    f"`{summary.get('measured_stranded_pre_active_turn_wait_samples', 0)}`"
                ),
                (
                    "- trace-linked samples: "
                    f"`{summary.get('measured_trace_linked_samples', 0)}`"
                ),
            ]
        )
    if "measured_prepare_timeout_wait_for_file_version_samples" in summary:
        profile_lines.extend(
            [
                (
                    "- prepare_timeout@wait_for_file_version samples: "
                    f"`{summary.get('measured_prepare_timeout_wait_for_file_version_samples', 0)}`"
                ),
                (
                    "- prepare_timeout@snapshot_with_deps samples: "
                    f"`{summary.get('measured_prepare_timeout_snapshot_with_deps_samples', 0)}`"
                ),
            ]
        )
    if "measured_cold_query_bundle_pool_wait_samples" in summary:
        profile_lines.extend(
            [
                (
                    "- cold `query_bundle_pool_wait` samples: "
                    f"`{summary.get('measured_cold_query_bundle_pool_wait_samples', 0)}`"
                ),
                (
                    "`p95(client_to_transport_wait_ms)="
                    f"{summary.get('measured_client_to_transport_wait_ms', {}).get('p95', 0):g}ms`"
                ),
                (
                    "`max(client_to_transport_wait_ms)="
                    f"{summary.get('measured_client_to_transport_wait_max_ms', 0)}ms`"
                ),
                (
                    "`p95(service_future_to_first_poll_wait_ms)="
                    f"{summary.get('measured_service_future_to_first_poll_wait_ms', {}).get('p95', 0):g}ms`"
                ),
                (
                    "`max(service_future_to_first_poll_wait_ms)="
                    f"{summary.get('measured_service_future_to_first_poll_wait_max_ms', 0)}ms`"
                ),
                (
                    "`p95(response_output_handoff_send_wait_ms)="
                    f"{summary.get('measured_response_output_handoff_send_wait_ms', {}).get('p95', 0):g}ms`"
                ),
                (
                    "`max(response_output_handoff_send_wait_ms)="
                    f"{summary.get('measured_response_output_handoff_send_wait_max_ms', 0)}ms`"
                ),
            ]
        )
    spec["summary_path"].write_text("\n".join(profile_lines) + "\n", encoding="utf-8")
    print(f"[gate] Real-module summary written to {spec['summary_path']}")

aggregate["pass"] = overall_pass
aggregate_path.write_text(
    json.dumps(aggregate, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
)

lines = [
    f"# {expected_change_id} readiness gates",
    "",
    "## Cross-adapter smoke",
    "- command: `./scripts/run-intellisense-tests.sh smoke`",
    "- pass: yes",
    "",
    "## Representative-matrix perf gate",
    f"- command: `CHANGE_ID={expected_change_id} BSL_V2_PERF_GATE_BLOCKING=1 ./scripts/run-intellisense-perf.sh`",
    f"- pass: {'yes' if all(row[1] == 'yes' for row in profile_rows) else 'no'}",
    "",
    "| profile | pass | matrix entries | failing entries | fail_closed budget failures | reason codes |",
    "|---|---|---:|---:|---:|---|",
]
for profile, pass_flag, entries_total, failing_total, fail_closed_total, reason_codes in profile_rows:
    lines.append(
        f"| {profile} | {pass_flag} | {entries_total} | {failing_total} | {fail_closed_total} | {reason_codes} |"
    )

lines.extend(
    [
        "",
        "## Real-module representative gates",
        "| profile | pass | report change_id | measured samples | successful traces | fail_closed traces | head_hit traces | exact_hit traces | prepare_timeout delta | exact_deadline delta | cold query_bundle_pool_wait samples |",
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
)
for (
    profile_title,
    pass_flag,
    report_change_id,
    measured_samples,
    successful_traces,
    fail_closed_traces,
    head_hit_traces,
    exact_hit_traces,
    prepare_timeout_delta,
    exact_deadline_delta,
    cold_query_bundle_pool_wait_samples,
) in real_module_rows:
    lines.append(
        f"| {profile_title} | {pass_flag} | {report_change_id} | {measured_samples} | {successful_traces} | {fail_closed_traces} | {head_hit_traces} | {exact_hit_traces} | {prepare_timeout_delta} | {exact_deadline_delta} | {cold_query_bundle_pool_wait_samples} |"
    )

lines.extend(
    [
        "",
        "## OpenSpec",
        f"- command: `openspec validate {expected_change_id} --strict --no-interactive`",
        "- pass: yes",
        f"- log: `{openspec_log_path}`",
        "",
    ]
)
summary_path.write_text("\n".join(lines), encoding="utf-8")
print(f"[gate] Summary written to {summary_path}")
print(f"[gate] Aggregate written to {aggregate_path}")
PY

echo "[gate] Done."
echo "[gate] Artifacts:"
echo "  - ${READINESS_REPORT}"
echo "  - ${READINESS_SUMMARY}"
for artifact in "${real_module_artifacts[@]}"; do
  echo "  - ${artifact}"
done
echo "  - ${OPENSPEC_LOG}"
