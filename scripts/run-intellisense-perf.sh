#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/backend/tests/perf/reports"
BASELINE_DIR="${ROOT_DIR}/backend/tests/perf/baselines"
CONTRACT_PATH="${ROOT_DIR}/contracts/intellisense-perf-gate/v1/contract.json"
THRESHOLD_RESOURCE="${THRESHOLD_RESOURCE:-1.15}"
THRESHOLD_P95="${THRESHOLD_P95:-1.10}"
THRESHOLD_P99="${THRESHOLD_P99:-1.15}"
PERF_PROFILES="${PERF_PROFILES:-small large churn}"
PERF_WARMUP="${PERF_WARMUP:-20}"
PERF_ITERATIONS="${PERF_ITERATIONS:-200}"

BLOCKING_FLAG=""
if [[ "${BSL_V2_PERF_GATE_BLOCKING:-0}" == "1" ]]; then
  BLOCKING_FLAG="--blocking-mode"
fi

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
    ${BLOCKING_FLAG} \
    --warmup "${PERF_WARMUP}" \
    --iterations "${PERF_ITERATIONS}" \
    --threshold-p95 "${THRESHOLD_P95}" \
    --threshold-p99 "${THRESHOLD_P99}" \
    --threshold-resource "${THRESHOLD_RESOURCE}" \
    --contract-path "${CONTRACT_PATH}" \
    --max-error-rate 0.0 \
    --max-incomplete-rate 0.0 \
    --output "${report}" \
    --summary "${REPORT_DIR}/intellisense_${name}.md"
}

for profile in ${PERF_PROFILES}; do
  run_profile "${profile}"
done

echo "Perf reports saved in ${REPORT_DIR}"
