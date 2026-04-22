# Live Evidence

## Commands

- `./scripts/validate-refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding.sh`
- `CHANGE_ID=refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=openspec/changes/refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding/validation/refactor-39-real-conf-big-diagnostics-representative-save-followup-bundle-live.json cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture`
- `CHANGE_ID=refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding BSL_V2_REAL_CONF_BIG_SHADOW_STATE_TIMEOUT_REPORT=openspec/changes/refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding/validation/refactor-39-real-conf-big-diagnostics-shadow-state-timeout-live.json cargo test -p bsl-backend --bin bsl-lsp-server p54_real_conf_big_diagnostics_shadow_state_timeout_report_live -- --nocapture`

## Result

- Representative `conf_big` save-followup family is now captured by a single repo-owned bundle:
  [refactor-39-real-conf-big-diagnostics-representative-save-followup-bundle-live.json](./refactor-39-real-conf-big-diagnostics-representative-save-followup-bundle-live.json)
  - Baseline bundle at `2026-04-17T14:06:03Z`: `ready_artifacts=1`, `shadow_state=3`
  - Refreshed representative bundle: `cycle_count=4`, `ready_artifacts=4`, `shadow_state=0`
  - Every representative cycle stayed on the same family surface:
    `followup_ready_snapshot_zero_probe=not_ready`, `followup_ready_snapshot_wait_probe=ready`

- Truthful exhausted-continuation fallback remains covered by a separate non-representative sidecar:
  [refactor-39-real-conf-big-diagnostics-shadow-state-timeout-live.json](./refactor-39-real-conf-big-diagnostics-shadow-state-timeout-live.json)
  - `shadow_path_delta=2`
  - `ready_path_delta=0`
  - `continuation_exhausted_delta=1`
  - `cycle_2.followup_ready_snapshot_continuation_reason=exhausted_continuation_proof`

## Interpretation

- `p56` is the canonical representative bundle for this change. It measures four consecutive real
  `conf_big` same-file save cycles from the same bounded follow-up family and records the incidence
  summary directly in one report.
- `p54` remains useful, but only as a truthful timeout/exhausted-proof sidecar. It intentionally
  forces fallback pressure and must not be treated as the representative acceptance bundle.
