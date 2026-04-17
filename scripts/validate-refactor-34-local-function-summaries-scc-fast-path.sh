#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHANGE_ID="refactor-34-local-function-summaries-scc-fast-path"
VALIDATION_DIR="${ROOT_DIR}/openspec/changes/${CHANGE_ID}/validation"
READY_SNAPSHOT_REPORT="${VALIDATION_DIR}/refactor-34-real-conf-big-diagnostics-ready-snapshot-leaf-live.json"

mkdir -p "${VALIDATION_DIR}"

echo "[refactor-34] Running targeted analysis-v2 parity regressions..."
cargo test -p bsl-analysis-v2 \
  singleton_non_recursive_local_summaries_skip_fixed_point_and_preserve_local_call_semantics
cargo test -p bsl-analysis-v2 \
  self_recursive_singleton_stays_on_convergence_path_and_preserves_local_call_semantics
cargo test -p bsl-analysis-v2 \
  mutually_recursive_local_summaries_reuse_stable_out_of_scc_semantics
cargo test -p bsl-analysis-v2 \
  semantic_diagnostics_profiled_report_snapshot_parse_and_ir_sources

echo "[refactor-34] Running backend observability regressions..."
cargo test -p bsl-backend --bin bsl-lsp-server \
  p24c_diagnostics_save_timeline_exports_semantic_diagnostics_query_breakdown
CHANGE_ID="${CHANGE_ID}" \
BSL_V2_REAL_CONF_BIG_READY_SNAPSHOT_LEAF_REPORT="${READY_SNAPSHOT_REPORT}" \
  cargo test -p bsl-backend --bin bsl-lsp-server \
    p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live -- --nocapture

echo "[refactor-34] Running OpenSpec validation..."
openspec validate "${CHANGE_ID}" --strict --no-interactive
