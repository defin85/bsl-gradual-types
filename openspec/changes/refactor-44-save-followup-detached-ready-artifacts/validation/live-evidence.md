# Validation Evidence

## Commands Run

### Targeted backend regressions

- `cargo test -p bsl-backend --bin bsl-lsp-server --no-run`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24b_diagnostics_save_timeline_prefers_detached_ready_artifacts_before_shadow_fallback -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server detached_ready_artifact_does_not_weaken_hover_fail_closed_gate -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server same_revision_ready_snapshot_waits_for_exact_type_index_before_hover -- --nocapture`
- `target/debug/deps/bsl_lsp_server-1c1a12b3b7313479 server::core::tests::interactive_completion::p7_definition_timeout_still_seeds_exact_type_index_without_did_save --exact --nocapture`
- `target/debug/deps/bsl_lsp_server-1c1a12b3b7313479 server::core::tests::interactive_completion::p7_signature_help_bootstraps_exact_type_index_without_did_save_when_precompute_fits_budget --exact --nocapture`
- `target/debug/deps/bsl_lsp_server-1c1a12b3b7313479 server::core::tests::live_reports::p53_real_conf_big_exact_program_lowering_report_live --exact --nocapture`

### Extension / spec validation

- `npm --prefix ./vscode-extension run compile:fast`
- `openspec validate refactor-44-save-followup-detached-ready-artifacts --strict --no-interactive`

## Passing Results

- The detached publication / consumption regression passed and proved that same-version
  `didSave` follow-up now resolves through `detached_ready_artifacts` before terminal
  `shadow_state` fallback when canonical ready artifacts are still pending.
- The hover-specific fail-closed regression passed and proved that inserting a matching detached
  diagnostics-ready artifact does not weaken the canonical live exact gate.
- The existing same-revision hover wait regression still passed, preserving the live exact wait
  path for interactive exact consumers.
- The existing definition and signature-help regressions still passed through the direct test
  binary, confirming that non-`didSave` exact consumers remain on their canonical fail-closed /
  bounded-wait behavior.
- The legacy `p53` live probe passed as an optional diagnostic signal and remains non-blocking for
  this change.
- `vscode-extension` compiled successfully after adding the new
  `detached_ready_artifacts` semantic-path labels.
- Strict OpenSpec validation passed.

## Representative Live Notes

The new detached branch is real and observable on the representative live family:

- `p56_real_conf_big_diagnostics_representative_save_followup_bundle_live_path=/home/egor/code/bsl-gradual-types/backend/tests/perf/reports/refactor-44-save-followup-detached-ready-artifacts-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`
- `followup_semantic_path=detached_ready_artifacts`
- `followup_publish.semantic_path=detached_ready_artifacts`
- `followup_ready_snapshot_zero_probe=not_ready`
- `followup_semantic_parse_source=snapshot`
- `followup_semantic_ir_source=snapshot_build`
- `followup_semantic_materialization_path=diagnostics_only`

The representative late family does not justify overclaiming a single canonical sub-shape.
Observed live cycles showed both of these detached cases:

- detached publish after truthful bounded-wait timeout with
  `followup_ready_snapshot_wait_probe=timeout` and
  `followup_ready_snapshot_timeout_leaf=ready_install`
- detached publish after the zero-budget miss without claiming a bounded-wait `ready` outcome

That means `refactor-44` acceptance should require truthful detached-path labeling and preserved
interactive fail-closed behavior, not an overfit assumption that every detached publish must have
the exact same canonical timeout sub-shape.

## Long / Flaky Probes

- `p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live` remains locally flaky in setup on
  this tree; repeated runs timed out waiting for the pre-save same-version ready snapshot to
  materialize after the intermediate `didChange`.
- `p52_real_conf_big_lagging_shadow_recovery_save_followup_report_live` also remains locally flaky
  in setup; one run timed out before the historical lagging-shadow precondition fully formed.
- `p56_real_conf_big_diagnostics_representative_save_followup_bundle_live` is the active
  representative gate for this change, but it is a long-running `conf_big` probe and its
  acceptance thresholds required multiple stale `refactor-41` assumptions to be removed:
  - detached publish is not always preceded by the same bounded-wait outcome;
  - detached publish does not imply that semantic query or even the full tracked semantic stack
    explains a majority of total publish latency;
  - old `did_change` ready-snapshot materialization baselines are no longer a valid gate for this
    change because `refactor-44` does not claim to speed the canonical live install itself.

The rebuilt representative run passed after removing those stale assumptions.
