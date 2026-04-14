# Live Evidence

## Commands

- `cargo test -p bsl-backend --bin bsl-lsp-server p24_diagnostics_save_timeline_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_diagnostics_save_timeline_marks_apply_lag_for_inflight_idle_heavy_without_ready_artifacts -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_skips_bounded_wait_after_exact_producer_is_retargeted_away -- --nocapture`
- `cargo test -p bsl-runtime did_save_followup_ready_snapshot_metrics_are_exported -- --nocapture`
- `BSL_TEST_GREP='Observability Incident Bundle' npm test`
- `CHANGE_ID=refactor-24-diagnostics-save-followup-budget-valve BSL_V2_REAL_CONF_TEST_READY_SNAPSHOT_RELIEF_VALVE_REPORT=/home/egor/code/bsl-gradual-types/openspec/changes/refactor-24-diagnostics-save-followup-budget-valve/validation/refactor-24-real-conf-test-ready-snapshot-relief-valve-live.json cargo test -p bsl-backend --bin bsl-lsp-server p51_real_conf_test_ready_snapshot_relief_valve_report_live -- --nocapture`

## Findings

- Synthetic regressions prove all three required valve outcomes without raw logs:
  - `engaged_timed_out`
  - `skipped_not_exact_still_current`
  - `engaged_helped`
- The additional regression `p24_diagnostics_save_timeline_preserves_relief_valve_after_terminal_publish`
  closes the timeline-store bug where `engaged_helped` could be lost once the same save cycle had
  already been archived as terminal.
- Checked-in live evidence in
  [refactor-24-real-conf-test-ready-snapshot-relief-valve-live.json](/home/egor/code/bsl-gradual-types/openspec/changes/refactor-24-diagnostics-save-followup-budget-valve/validation/refactor-24-real-conf-test-ready-snapshot-relief-valve-live.json)
  now shows the intended narrow operational shape on a real repo-local configuration module:
  - `followup_ready_snapshot_zero_probe=not_ready`
  - `followup_ready_snapshot_wait_probe=timeout`
  - `followup_ready_snapshot_timeout_phase=parse_exec`
  - `followup_ready_snapshot_relief_valve_outcome=engaged_helped`
  - `followup_ready_snapshot_relief_valve_elapsed_ms=307`
  - `followup_semantic_path=ready_artifacts`
  - `bounded_wait_timeout_delta=1`
  - `relief_helped_delta=1`

## Interpretation

- The checked-in live artifact captures both sides of the temporary valve contract on one real save
  cycle: the base `3500ms` exact wait already times out truthfully, but the extra bounded
  `500ms` window still rescues the same exact producer and keeps heavy follow-up on
  `ready_artifacts` instead of falling back to `shadow_state`.
- Queue/apply-lag and coalesced-away cases remain explicitly excluded and keep their truthful
  existing paths, as shown by the synthetic `skipped_apply_lag` and
  `skipped_not_exact_still_current` regressions.

## Sunset Condition

- This valve should be disabled or removed once representative live evidence shows that the exact
  `didSave` ready-snapshot path fits comfortably under the base `3500ms` budget again.
- Operationally, treat the valve as sunset-ready when recent representative bundles no longer need
  `followup_ready_snapshot_relief_valve_outcome=engaged_helped` to stay on `ready_artifacts`, and
  the exact-path p95 remains below the base budget without this temporary window.
