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

run_smoke() {
  cargo test -p bsl-backend --lib completion_ranking
  cargo test -p bsl-backend --lib completion_service
  cargo test -p bsl-backend --test intellisense_testkit_smoke_test
  cargo test -p bsl-backend --test intellisense_golden_completion_test
  cargo test -p bsl-backend --test lsp_intellisense_tests
  cargo test -p bsl-backend --test m8_completion_matrix_golden_v2_test
  cargo test -p bsl-backend --test lsp_incremental_completion_test
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
