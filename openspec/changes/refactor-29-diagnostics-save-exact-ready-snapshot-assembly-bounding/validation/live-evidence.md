# Live Evidence

## Commands

- `cargo fmt --all`
- `cargo test -p bsl-backend --bin bsl-lsp-server p29_diagnostics_save_timeline_reports_exact_ready_snapshot_assembly_checkpoint_for_exact_worker -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p29_parsed_did_change_revision_is_retargeted_during_syntax_error_collection_when_newer_target_arrives -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p29_did_save_followup_promotes_exact_ready_snapshot_assembly_past_syntax_error_collection -- --nocapture`
- `BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite' npm test`
- `CHANGE_ID=refactor-29-diagnostics-save-exact-ready-snapshot-assembly-bounding BSL_V2_REAL_CONF_BIG_LAGGING_SHADOW_RECOVERY_SAVE_FOLLOWUP_REPORT=/home/egor/code/bsl-gradual-types/openspec/changes/refactor-29-diagnostics-save-exact-ready-snapshot-assembly-bounding/validation/refactor-29-real-conf-big-lagging-shadow-recovery-save-followup-live.json cargo test -p bsl-backend --bin bsl-lsp-server p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live -- --nocapture`
- `openspec validate refactor-29-diagnostics-save-exact-ready-snapshot-assembly-bounding --strict --no-interactive`

## Result

- Targeted regressions show that exact same-version `didSave` follow-up can publish through
  `ready_artifacts` without waiting for deferred syntax-error assembly on the critical path.
- Same-file supersession remains fail-closed inside the new exact assembly slice. The stale worker
  still terminates as `retargeted_during_parse`, not `retargeted_before_parse` or generic abort.
- Diagnostics save timeline and incident bundle surfaces now export contract `v15`, including the
  exact `ready_snapshot_assembly` timeout checkpoint, dominant checkpoint, and per-slice timing.
- Versioned VS Code tests prove the new `version=14` unavailable-by-design note for exact
  ready-snapshot assembly checkpoint attribution.

## Representative `conf_big` Outcome

- Checked-in report:
  [refactor-29-real-conf-big-lagging-shadow-recovery-save-followup-live.json](./refactor-29-real-conf-big-lagging-shadow-recovery-save-followup-live.json)
- The mixed `didChange + didSave` path does **not** return to `ready_artifacts` yet.
- The residual is now narrower and truthful:
  - `followup_semantic_path=shadow_state`
  - `followup_ready_snapshot_timeout_phase=parse_exec`
  - `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
  - `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint=program_conversion`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms=4034`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms=4034`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms=null`
  - `followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms=64`
  - `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`

## Interpretation

- `refactor-29` achieved its scope: exact same-version follow-up no longer treats the whole
  `ready_snapshot_assembly` bucket as opaque, and the save-critical path can cut through deferred
  syntax-error assembly in the synthetic regression.
- Representative `conf_big` still falls back to `shadow_state`, but the remaining exact-path
  bottleneck is now narrower and operator-meaningful: `program_conversion` inside exact
  `ready_snapshot_assembly`.
- The next change should target exact `program_conversion`, not reopen parser-tree build,
  tree-cache install, apply-lag, or generic wait-budget tuning.
