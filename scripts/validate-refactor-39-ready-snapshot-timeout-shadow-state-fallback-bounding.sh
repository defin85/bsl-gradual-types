#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHANGE_ID="refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding"
VALIDATION_DIR="${ROOT_DIR}/openspec/changes/${CHANGE_ID}/validation"
REPRESENTATIVE_REPORT="${VALIDATION_DIR}/refactor-39-real-conf-big-diagnostics-representative-save-followup-bundle-live.json"
SHADOW_TIMEOUT_REPORT="${VALIDATION_DIR}/refactor-39-real-conf-big-diagnostics-shadow-state-timeout-live.json"

mkdir -p "${VALIDATION_DIR}"

echo "[refactor-39] Running targeted backend/runtime regressions..."
cargo test -p bsl-backend --bin bsl-lsp-server \
  p24_diagnostics_save_timeline_ -- --nocapture
cargo test -p bsl-backend --bin bsl-lsp-server \
  p32_diagnostics_save_timeline_ -- --nocapture

echo "[refactor-39] Refreshing representative conf_big save-followup bundle..."
CHANGE_ID="${CHANGE_ID}" \
BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT="${REPRESENTATIVE_REPORT}" \
  cargo test -p bsl-backend --bin bsl-lsp-server \
    p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture

echo "[refactor-39] Refreshing truthful exhausted-continuation timeout sidecar..."
CHANGE_ID="${CHANGE_ID}" \
BSL_V2_REAL_CONF_BIG_SHADOW_STATE_TIMEOUT_REPORT="${SHADOW_TIMEOUT_REPORT}" \
  cargo test -p bsl-backend --bin bsl-lsp-server \
    p54_real_conf_big_diagnostics_shadow_state_timeout_report_live -- --nocapture

echo "[refactor-39] Running OpenSpec validation..."
openspec validate "${CHANGE_ID}" --strict --no-interactive
