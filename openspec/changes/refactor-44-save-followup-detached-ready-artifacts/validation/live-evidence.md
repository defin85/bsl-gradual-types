# Validation Evidence

## Commands Run

### Targeted backend regressions

- `cargo test -p bsl-backend --bin bsl-lsp-server --no-run`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24b_diagnostics_save_timeline_prefers_detached_ready_artifacts_before_shadow_fallback -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24b_diagnostics_save_timeline_ignores_detached_ready_artifacts_from_older_save_cycle -- --nocapture`
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
- The new stale-artifact regression passed and proved that an older detached artifact with the
  same version and text hash but an older `save_cycle_sequence` is not consumed for the newer
  still-current target.
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

The repo-auditable checked-in representative summary for this change is:

- `openspec/changes/refactor-44-save-followup-detached-ready-artifacts/validation/p56-representative-summary.json`

That tracked summary captures four representative late-family cycles, and all four keep:

- `followup_semantic_path=detached_ready_artifacts`
- `followup_publish_semantic_path=detached_ready_artifacts`
- `followup_ready_snapshot_zero_probe=not_ready`
- `followup_ready_snapshot_wait_probe=timeout`
- `followup_ready_snapshot_timeout_leaf=ready_install`
- `semantic_query_dominates_parse_exec=true`
- `exact_ready_after_timeout=false`
- `completion_head_ready_after_timeout=false`

The current checked-in summary therefore proves the timeout-shaped detached branch truthfully.
The acceptance contract for `refactor-44` remains narrower than a fixed sub-shape claim:
it requires truthful detached-path labeling plus preserved interactive fail-closed behavior,
not a promise that every future detached publish must keep the exact same mix of timeout subcases.

## Requirement -> Code -> Test

- Detached `didSave` follow-up prefers diagnostics-only detached artifacts for the exact same-save
  target -> `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`,
  `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` -> `p24b_diagnostics_save_timeline_prefers_detached_ready_artifacts_before_shadow_fallback`,
  `validation/p56-representative-summary.json`
- Interactive exact consumers stay on the canonical live exact gate -> `backend/src/bin/lsp_server/server/language_server/impl_features_b.rs`,
  `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`,
  `backend/src/bin/lsp_server/server/language_server/impl_completion.rs` ->
  `detached_ready_artifact_does_not_weaken_hover_fail_closed_gate`,
  `same_revision_ready_snapshot_waits_for_exact_type_index_before_hover`,
  `p7_definition_timeout_still_seeds_exact_type_index_without_did_save`,
  `p7_signature_help_bootstraps_exact_type_index_without_did_save_when_precompute_fits_budget`
- Older same-file detached artifacts do not leak across save cycles -> `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` ->
  `p24b_diagnostics_save_timeline_ignores_detached_ready_artifacts_from_older_save_cycle`

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
