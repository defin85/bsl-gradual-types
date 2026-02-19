#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/backend/tests/perf/reports"
GATE_REPORT="${REPORT_DIR}/improve-v2-completion-interactive-reliability-gate.json"
GATE_SUMMARY="${REPORT_DIR}/improve-v2-completion-interactive-reliability-gate.md"
OPENSPEC_LOG="${REPORT_DIR}/improve-v2-completion-interactive-reliability-openspec-validate.log"

mkdir -p "${REPORT_DIR}"

if ! command -v openspec >/dev/null 2>&1; then
  echo "openspec CLI is required for strict change validation (command not found)." >&2
  exit 1
fi

echo "[gate] Running interactive acceptance test with artifact output..."
BSL_V2_COMPLETION_GATE_REPORT="${GATE_REPORT}" \
  cargo test -p bsl-backend p27_interactive_completion_acceptance_gates_emit_artifact -- --nocapture

python3 - "${GATE_REPORT}" "${GATE_SUMMARY}" <<'PY'
import json
import pathlib
import sys

report_path = pathlib.Path(sys.argv[1])
summary_path = pathlib.Path(sys.argv[2])

data = json.loads(report_path.read_text(encoding="utf-8"))
thresholds = data.get("thresholds", {})
results = data.get("results", {})
lines = [
    "# improve-v2 completion acceptance gates",
    "",
    f"- pass: {'yes' if data.get('pass') else 'no'}",
    f"- iterations: {data.get('iterations', 0)}",
    "",
    "| metric | value | threshold |",
    "|---|---:|---:|",
    (
        f"| completion p95 (ms) | {results.get('completion_p95_ms', 0):.3f} | "
        f"<= {thresholds.get('completion_p95_ms_max', 0):.3f} |"
    ),
    (
        f"| completion p99 (ms) | {results.get('completion_p99_ms', 0):.3f} | "
        f"<= {thresholds.get('completion_p99_ms_max', 0):.3f} |"
    ),
    (
        f"| first-trigger success rate | {results.get('first_trigger_success_rate', 0):.4f} | "
        f">= {thresholds.get('first_trigger_success_rate_min', 0):.4f} |"
    ),
    (
        f"| terminal-empty (missing_ir) rate | "
        f"{results.get('terminal_empty_missing_ir_rate', 0):.4f} | "
        f"<= {thresholds.get('terminal_empty_missing_ir_rate_max', 0):.4f} |"
    ),
    (
        f"| parity mismatch rate | {results.get('parity_mismatch_rate', 0):.4f} | "
        f"<= {thresholds.get('parity_mismatch_rate_max', 0):.4f} |"
    ),
    "",
]
summary_path.write_text("\n".join(lines), encoding="utf-8")
print(f"[gate] Summary written to {summary_path}")
PY

echo "[gate] Running OpenSpec strict validation..."
openspec validate improve-v2-completion-interactive-reliability --strict --no-interactive \
  | tee "${OPENSPEC_LOG}"

echo "[gate] Done."
echo "[gate] Artifacts:"
echo "  - ${GATE_REPORT}"
echo "  - ${GATE_SUMMARY}"
echo "  - ${OPENSPEC_LOG}"
