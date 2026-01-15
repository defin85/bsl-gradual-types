#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/backend/tests/perf/reports"
BASELINE_DIR="${ROOT_DIR}/backend/tests/perf/baselines"

UPDATE_FLAG=""
if [[ "${UPDATE_BASELINE:-0}" == "1" ]]; then
  UPDATE_FLAG="--update-baseline"
fi

mkdir -p "${REPORT_DIR}" "${BASELINE_DIR}"

run_profile() {
  local name="$1"
  local scenario="${ROOT_DIR}/backend/tests/perf/scenarios/intellisense_${name}.json"
  local baseline="${BASELINE_DIR}/intellisense_${name}.json"
  local report="${REPORT_DIR}/intellisense_${name}.json"

  RAYON_NUM_THREADS=1 \
  cargo run -p bsl-backend --bin intellisense_perf -- \
    --scenario "${scenario}" \
    --baseline "${baseline}" \
    ${UPDATE_FLAG} \
    --threshold-p95 1.10 \
    --threshold-p99 1.15 \
    --max-error-rate 0.0 \
    --max-incomplete-rate 0.0 \
    --output "${report}" \
    --summary "${REPORT_DIR}/intellisense_${name}.md"
}

run_profile "small"
run_profile "medium"

echo "Perf reports saved in ${REPORT_DIR}"
