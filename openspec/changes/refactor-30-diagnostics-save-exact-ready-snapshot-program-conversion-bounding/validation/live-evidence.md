# Live Evidence

## Commands

- `CHANGE_ID=refactor-30-diagnostics-save-exact-ready-snapshot-program-conversion-bounding BSL_V2_REAL_CONF_BIG_LAGGING_SHADOW_RECOVERY_SAVE_FOLLOWUP_REPORT=/tmp/refactor-30-real-conf-big-lagging-shadow-recovery-save-followup-live.json cargo test -p bsl-backend --bin bsl-lsp-server p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live -- --nocapture`

## Result

- Representative repo-local `conf_big` live evidence for OpenSpec task `3.2` was captured from the
  current worktree on `2026-04-15`.
- The mixed `didChange + didSave` path does **not** return to `ready_artifacts` yet.
- The residual is now narrower and truthful than the `refactor-29` checked-in evidence: the exact
  same-version timeout no longer stops at the generic `program_conversion` bucket and now points to
  `program_lowering` inside exact `ready_snapshot_assembly`.

## Representative `conf_big` Outcome

- Checked-in report:
  [refactor-30-real-conf-big-lagging-shadow-recovery-save-followup-live.json](./refactor-30-real-conf-big-lagging-shadow-recovery-save-followup-live.json)
- Observed fields:
  - `followup_semantic_path=shadow_state`
  - `followup_ready_snapshot_timeout_phase=parse_exec`
  - `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
  - `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint=program_lowering`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint=program_lowering`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms=4016`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms=4016`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms=null`
  - `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`

## Interpretation

- `refactor-30` satisfies the OpenSpec `3.2` success criterion through the second allowed outcome:
  representative `conf_big` still falls back to `shadow_state`, but the residual moved from the
  old monolithic `program_conversion` bucket to the narrower exact checkpoint
  `program_lowering`.
- This artifact unblocks Beads task `bsl-gradual-types-ptc7.4`, whose note explicitly left
  targeted validation open only until `bsl-gradual-types-ptc7.3` captured the required
  representative `conf_big` evidence.
