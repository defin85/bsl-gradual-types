#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"
PROFILE="${1:-smoke}"

run_smoke() {
  cargo test -p bsl-backend --lib completion_ranking
  cargo test -p bsl-backend --lib completion_service
  cargo test -p bsl-backend --test intellisense_testkit_smoke_test
  cargo test -p bsl-backend --test intellisense_golden_completion_test
  cargo test -p bsl-backend --test lsp_intellisense_tests
}

run_full() {
  run_smoke
  cargo test -p bsl-backend --test shared_test_fixtures_test
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
