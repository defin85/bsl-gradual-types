#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/backend/tests/perf/reports"
CHANGE_ID="${CHANGE_ID:-refactor-completion-prepare-lightweight-exact-split}"
READINESS_REPORT="${REPORT_DIR}/${CHANGE_ID}-readiness-gate.json"
READINESS_SUMMARY="${REPORT_DIR}/${CHANGE_ID}-readiness-gate.md"
REAL_MODULE_PROFILE="p38_real_conf_big_revision_churn_completion_perf_report_live"
REAL_MODULE_REPORT="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.json"
REAL_MODULE_SUMMARY="${REPORT_DIR}/${CHANGE_ID}-real-conf-big-revision-churn-completion-perf-live.md"
OPENSPEC_LOG="${REPORT_DIR}/${CHANGE_ID}-openspec-validate.log"
PERF_PROFILES="${PERF_PROFILES:-small large churn}"

mkdir -p "${REPORT_DIR}"

if ! command -v openspec >/dev/null 2>&1; then
  echo "openspec CLI is required for strict change validation (command not found)." >&2
  exit 1
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

python3 - "${REPORT_DIR}" "${READINESS_REPORT}" "${READINESS_SUMMARY}" "${CHANGE_ID}" ${PERF_PROFILES} <<'PY'
import json
import pathlib
import sys

report_dir = pathlib.Path(sys.argv[1])
aggregate_path = pathlib.Path(sys.argv[2])
summary_path = pathlib.Path(sys.argv[3])
expected_change_id = sys.argv[4]
profiles = sys.argv[5:]

profile_rows = []
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
}
overall_pass = True

for profile in profiles:
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
        "report": str(report_path.relative_to(report_dir.parents[2])),
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

aggregate["pass"] = overall_pass
aggregate_path.write_text(json.dumps(aggregate, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

lines = [
    f"# {expected_change_id} readiness gates",
    "",
    "## Cross-adapter smoke",
    "- command: `./scripts/run-intellisense-tests.sh smoke`",
    "- pass: yes",
    "",
    "## Representative-matrix perf gate",
    f"- command: `CHANGE_ID={expected_change_id} BSL_V2_PERF_GATE_BLOCKING=1 ./scripts/run-intellisense-perf.sh`",
    f"- pass: {'yes' if overall_pass else 'no'}",
    "",
    "| profile | pass | matrix entries | failing entries | fail_closed budget failures | reason codes |",
    "|---|---|---:|---:|---:|---|",
]
for profile, pass_flag, entries_total, failing_total, fail_closed_total, reason_codes in profile_rows:
    lines.append(
        f"| {profile} | {pass_flag} | {entries_total} | {failing_total} | {fail_closed_total} | {reason_codes} |"
    )
lines.append("")
summary_path.write_text("\n".join(lines), encoding="utf-8")
print(f"[gate] Summary written to {summary_path}")
print(f"[gate] Aggregate written to {aggregate_path}")
PY

echo "[gate] Running real-module post-handoff readiness gate..."
BSL_V2_REAL_CONF_BIG_REVISION_CHURN_COMPLETION_PERF_REPORT="${REAL_MODULE_REPORT}" \
  cargo test -p bsl-backend --bin bsl-lsp-server "${REAL_MODULE_PROFILE}" -- --nocapture

python3 - "${REAL_MODULE_REPORT}" "${REAL_MODULE_SUMMARY}" "${CHANGE_ID}" "${REAL_MODULE_PROFILE}" <<'PY'
import json
import pathlib
import sys

report_path = pathlib.Path(sys.argv[1])
summary_path = pathlib.Path(sys.argv[2])
change_id = sys.argv[3]
profile = sys.argv[4]

data = json.loads(report_path.read_text(encoding="utf-8"))
summary = data.get("summary") or {}
lines = [
    f"# {change_id} real-module readiness gate",
    "",
    f"- profile: `{profile}`",
    f"- report: `{report_path}`",
    f"- report change_id: `{data.get('change_id', '-')}`",
    f"- measured samples: `{summary.get('measured_requests', summary.get('measured_trace_linked_samples', 0))}`",
    f"- head_hit traces: `{summary.get('measured_head_hit_traces', 0)}`",
    f"- exact_hit traces: `{summary.get('measured_exact_hit_traces', 0)}`",
    f"- prepare_timeout delta: `{summary.get('measured_prepare_timeout_total_delta', 0)}`",
    f"- exact_deadline delta: `{summary.get('measured_exact_deadline_total_delta', 0)}`",
]
summary_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"[gate] Real-module summary written to {summary_path}")
PY

echo "[gate] Running OpenSpec strict validation..."
openspec validate "${CHANGE_ID}" --strict --no-interactive \
  | tee "${OPENSPEC_LOG}"

echo "[gate] Done."
echo "[gate] Artifacts:"
echo "  - ${READINESS_REPORT}"
echo "  - ${READINESS_SUMMARY}"
echo "  - ${REAL_MODULE_REPORT}"
echo "  - ${REAL_MODULE_SUMMARY}"
echo "  - ${OPENSPEC_LOG}"
