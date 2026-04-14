# Live Evidence

## Commands

- `cargo fmt --all`
- `cargo test -p bsl-backend --bin bsl-lsp-server p25_did_change_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24_diagnostics_save_timeline_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p26_diagnostics_save_timeline_ -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p27_diagnostics_save_timeline_reports_parse_exec_core_subphase_for_exact_worker -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p27_parsed_did_change_revision_is_retargeted_during_optional_cache_enrichment_when_newer_target_arrives -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p27_did_save_followup_promotes_exact_parse_exec_past_optional_cache_enrichment -- --nocapture`
- `BSL_TEST_GREP='LSP Custom Requests Test Suite|Observability Incident Bundle Test Suite' npm test`
- `CHANGE_ID=refactor-27-diagnostics-save-exact-parse-exec-bounding cargo test -p bsl-backend --bin bsl-lsp-server p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live -- --nocapture`

## Findings

- Save-critical exact `didSave` follow-up no longer waits for the full injected optional
  cache-enrichment tail. Regression
  `p27_did_save_followup_promotes_exact_parse_exec_past_optional_cache_enrichment` proves the
  exact path still publishes through `ready_artifacts`, with bounded `parse_exec` and without
  leaving stale timeout or relief-valve attribution behind.
- Supersession remains fail-closed inside the new optional-enrichment slice. Regression
  `p27_parsed_did_change_revision_is_retargeted_during_optional_cache_enrichment_when_newer_target_arrives`
  still terminates the stale worker as `retargeted_during_parse` and materializes only the newer
  target.
- Diagnostics save timeline and incident bundle surfaces now export exact parse-exec subphase
  truth. Regression
  `p27_diagnostics_save_timeline_reports_parse_exec_core_subphase_for_exact_worker` and the
  versioned TS tests verify contract `v13` and the `version=12` unavailable-by-design note.
- Checked-in live `conf_big` evidence in
  [refactor-27-real-conf-big-lagging-shadow-recovery-save-followup-live.json](./refactor-27-real-conf-big-lagging-shadow-recovery-save-followup-live.json)
  shows that the mixed path still does not return to `ready_artifacts`, but the residual is no
  longer an opaque `parse_exec` blob:
  - `followup_semantic_path=shadow_state`
  - `followup_ready_snapshot_zero_probe=not_ready`
  - `followup_ready_snapshot_wait_probe=timeout`
  - `followup_ready_snapshot_timeout_phase=parse_exec`
  - `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
  - `followup_ready_snapshot_parse_exec_dominant_subphase=core_parse_build`
  - `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`

## Interpretation

- `refactor-27` achieved its scoped goal: exact same-version follow-up no longer treats the whole
  in-parse tail as an opaque phase, and save-critical promotion can cut through deferrable
  optional-enrichment work on the critical path.
- Representative `conf_big` still falls back to `shadow_state`, but the remaining bottleneck is
  now narrower and truthful: `core_parse_build` dominates the exact timeout path. That gives the
  next change a concrete target instead of another generic `parse_exec` residual.
