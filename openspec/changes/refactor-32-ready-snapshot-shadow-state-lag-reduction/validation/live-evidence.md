# Live Evidence

## Commands

- `cargo fmt --all`
- `cargo test -p bsl-runtime save_critical_requested_during_program_lowering_returns_before_packaging_checkpoint -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server whole_text_change_to_parser_edit -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p32_diagnostics_save_timeline_relief_valve_treats_late_did_save_task_as_exact_current -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p32_ranged_did_change_program_lowering_retarget_preserves_parser_base_for_newer_target -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p49_real_conf_big_stale_parser_base_root_cause_report_live -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p53_real_conf_big_exact_program_lowering_report_live -- --nocapture`
- `openspec validate refactor-32-ready-snapshot-shadow-state-lag-reduction --strict --no-interactive`

## Result

- Same-file ranged `didChange` no longer defaults to `stale_parser_base` on the representative
  `conf_big` churn family. `p49` now records `parse_mode=incremental`, `base_text_source=shadow_state`,
  `base_document_version=3`, and no `fallback_reason` / `parser_base_root_cause`.
- Same-file `didSave` heavy follow-up no longer ends on `shadow_state` for the representative mixed
  profile. Both refreshed live reports now publish through `ready_artifacts`.
- The exact ready-snapshot path keeps truthful bounded attribution. In the refreshed `p53` report,
  `program_conversion_ms == program_lowering_ms == 13`.
- Targeted regressions still prove the intended bounded semantics:
  - late ranged retarget preserves parser-base continuity for the newest target;
  - late `didSave` exact worker still qualifies for bounded relief wait instead of being skipped as
    `not_exact_still_current`;
  - parser-coordinator save-critical lowering regression remains green.

## Representative `conf_big` Reports

- [refactor-32-ready-snapshot-shadow-state-lag-reduction-real-conf-big-stale-parser-base-root-cause-live.json](./refactor-32-ready-snapshot-shadow-state-lag-reduction-real-conf-big-stale-parser-base-root-cause-live.json)
- [refactor-32-ready-snapshot-shadow-state-lag-reduction-real-conf-big-lagging-shadow-recovery-save-followup-live.json](./refactor-32-ready-snapshot-shadow-state-lag-reduction-real-conf-big-lagging-shadow-recovery-save-followup-live.json)
- [refactor-32-ready-snapshot-shadow-state-lag-reduction-real-conf-big-exact-program-lowering-live.json](./refactor-32-ready-snapshot-shadow-state-lag-reduction-real-conf-big-exact-program-lowering-live.json)

## Notes

- This rollout refreshes the representative checked-in live reports. A separate interactive VS Code
  incident-bundle export was not re-captured in this implementation pass.
