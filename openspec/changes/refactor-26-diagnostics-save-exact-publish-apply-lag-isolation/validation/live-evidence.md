# Live Evidence

## Commands

- `cargo fmt --all`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24_diagnostics_save_timeline_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p26_diagnostics_save_timeline_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p26_did_save_followup_relief_valve_publishes_ready_artifacts_despite_delayed_apply -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_diagnostics_save_timeline_marks_apply_lag_for_inflight_idle_heavy_without_ready_artifacts -- --nocapture`
- `BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite' npm test`
- `CHANGE_ID=refactor-26-diagnostics-save-exact-publish-apply-lag-isolation cargo test -p bsl-backend --bin bsl-lsp-server p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live -- --nocapture`

## Findings

- Timeline/export now carry a separate bounded blocker label for the exact-ready path:
  `followup_blocker_reason=post_ready_publish_gate` can coexist with factual
  `followup_apply_lag_ms`, so operator-facing attribution no longer has to collapse everything
  back into generic `apply_lag`.
- Relief-valve behavior changed exactly where `refactor-26` intended:
  - late exact phase + `apply_lag` now yields `engaged_timed_out` / `engaged_helped`,
    not `skipped_apply_lag`;
  - waiting-only exact phase still truthfully reports `skipped_apply_lag`.
- Synthetic mixed same-file regression
  `p26_did_save_followup_relief_valve_publishes_ready_artifacts_despite_delayed_apply`
  proves the new runtime path: base bounded wait times out, the extra bounded relief window catches
  the same exact producer, and follow-up still publishes through `ready_artifacts`.
- Checked-in live `conf_big` evidence in
  [refactor-26-real-conf-big-lagging-shadow-recovery-save-followup-live.json](./refactor-26-real-conf-big-lagging-shadow-recovery-save-followup-live.json)
  shows that the mixed load still does not return to `ready_artifacts`, but the residual blocker is
  now different and truthful:
  - `followup_semantic_path=shadow_state`
  - `followup_ready_snapshot_zero_probe=not_ready`
  - `followup_ready_snapshot_wait_probe=timeout`
  - `followup_ready_snapshot_timeout_phase=parse_exec`
  - `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`

## Interpretation

- `refactor-26` achieved its scoped goal: `apply_lag` is no longer the primary reason the exact
  path gives up on representative late exact workers, and the operator-facing surfaces now show
  that distinction explicitly.
- The remaining `conf_big` bottleneck is still exact `parse_exec` duration. In other words, the
  writer/apply gate is no longer the dominant residual for this path; the exact worker simply does
  not finish within `3500ms + 500ms`.
