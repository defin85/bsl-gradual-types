#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/backend/tests/perf/reports"
CHANGE_ID="add-bounded-stale-completion-fastpath"
OPENSPEC_LOG="${REPORT_DIR}/${CHANGE_ID}-openspec-validate.log"

mkdir -p "${REPORT_DIR}"

if ! command -v openspec >/dev/null 2>&1; then
  echo "openspec CLI is required for strict change validation (command not found)." >&2
  exit 1
fi

echo "[gate] Running scale-aware gate live test for ${CHANGE_ID}..."
cargo test -p bsl-backend p31_scale_aware_large_small_completion_gate_live -- --nocapture

echo "[gate] Running OpenSpec strict validation for ${CHANGE_ID}..."
openspec validate "${CHANGE_ID}" --strict --no-interactive | tee "${OPENSPEC_LOG}"

echo "[gate] Done."
echo "[gate] Artifacts:"
echo "  - ${REPORT_DIR}/${CHANGE_ID}-live.json"
echo "  - ${OPENSPEC_LOG}"
