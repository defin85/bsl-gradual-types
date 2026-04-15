# Live Evidence

## Commands

- `cargo fmt --all`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24_diagnostics_save_timeline_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p26_diagnostics_save_timeline_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p27_diagnostics_save_timeline_reports_parse_exec_core_subphase_for_exact_worker -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p27_parsed_did_change_revision_is_retargeted_during_optional_cache_enrichment_when_newer_target_arrives -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p27_did_save_followup_promotes_exact_parse_exec_past_optional_cache_enrichment -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p28_diagnostics_save_timeline_reports_core_build_checkpoint_for_exact_worker -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p28_parsed_did_change_revision_is_retargeted_during_tree_cache_install_when_newer_target_arrives -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p28_did_save_followup_promotes_exact_core_build_past_tree_cache_install -- --nocapture`
- `cargo test -p bsl-runtime did_save_followup_ready_snapshot_metrics_are_exported -- --nocapture`
- `BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite' npm test`
- `CHANGE_ID=refactor-28-diagnostics-save-exact-core-parse-build-bounding BSL_V2_REAL_CONF_BIG_LAGGING_SHADOW_RECOVERY_SAVE_FOLLOWUP_REPORT=/home/egor/code/bsl-gradual-types/openspec/changes/refactor-28-diagnostics-save-exact-core-parse-build-bounding/validation/refactor-28-real-conf-big-lagging-shadow-recovery-save-followup-live.json cargo test -p bsl-backend --bin bsl-lsp-server p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live -- --nocapture`
- `openspec validate refactor-28-diagnostics-save-exact-core-parse-build-bounding --strict --no-interactive`

## Result

- Targeted regressions show the new bounded checkpoint layer on the exact `core_parse_build` path:
  `parser_tree_build`, `exact_ready_snapshot_assembly`, and `tree_cache_install`.
- Save-critical same-version `didSave` follow-up now bypasses delayed `tree_cache_install` on the
  critical path and still publishes through `ready_artifacts` in the synthetic regression.
- Same-file supersession remains truthful inside the new `tree_cache_install` checkpoint and still
  terminates as `retargeted_during_parse`, not `retargeted_before_parse` or generic abort.

## Representative `conf_big` Outcome

- Checked-in report:
  [refactor-28-real-conf-big-lagging-shadow-recovery-save-followup-live.json](./refactor-28-real-conf-big-lagging-shadow-recovery-save-followup-live.json)
- The mixed `didChange + didSave` path does **not** return to `ready_artifacts` yet.
- The residual is now narrower and truthful:
  - `followup_semantic_path=shadow_state`
  - `followup_ready_snapshot_timeout_phase=parse_exec`
  - `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
  - `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
  - `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms=4050`
  - `followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms=54`
  - `followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms=null`
  - `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`

## Interpretation

- `refactor-28` achieved its scope: the exact same-version residual no longer collapses into a
  monolithic `core_parse_build` bucket, and delayed `tree_cache_install` is no longer on the
  save-critical path.
- The next bottleneck is now the `exact_ready_snapshot_assembly` slice inside exact
  `core_parse_build`, not parser-tree construction and not tree-cache install.
