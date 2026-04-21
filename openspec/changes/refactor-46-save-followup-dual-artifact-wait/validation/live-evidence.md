# Validation Evidence

## Commands Run

### Targeted backend regressions

- `cargo test -p bsl-backend --bin bsl-lsp-server --no-run`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24c_diagnostics_save_timeline_bounded_wait_wakes_on_detached_ready_artifact_publication -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24c_diagnostics_save_timeline_bounded_wait_prefers_ready_artifacts_when_canonical_materializes_first -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24c_diagnostics_save_timeline_bounded_wait_ignores_stale_detached_publication_for_newer_target -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24b_diagnostics_save_timeline_prefers_detached_ready_artifacts_before_shadow_fallback -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p24b_diagnostics_save_timeline_ignores_detached_ready_artifacts_from_older_save_cycle -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server detached_ready_artifact_does_not_weaken_hover_fail_closed_gate -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server same_revision_ready_snapshot_waits_for_exact_type_index_before_hover -- --nocapture`
- `CHANGE_ID=refactor-46-save-followup-dual-artifact-wait BSL_V2_REAL_CONF_BIG_READY_SNAPSHOT_LEAF_REPORT=openspec/changes/refactor-46-save-followup-dual-artifact-wait/validation/refactor-46-real-conf-big-diagnostics-ready-snapshot-leaf-live.json cargo test -p bsl-backend --bin bsl-lsp-server p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live -- --nocapture`
- `CHANGE_ID=refactor-46-save-followup-dual-artifact-wait BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT=openspec/changes/refactor-46-save-followup-dual-artifact-wait/validation/refactor-46-real-conf-big-diagnostics-representative-save-followup-bundle-live.json cargo test -p bsl-backend --bin bsl-lsp-server p56_real_conf_big_diagnostics_representative_save_followup_bundle_live -- --nocapture`

### Spec validation

- `openspec validate refactor-46-save-followup-dual-artifact-wait --strict --no-interactive`

## Passing Results

- The new `p24c` regression family passed and proved that the same-version `didSave` bounded wait:
  - wakes on matching detached diagnostics-ready publication before timeout-sized canonical exhaustion,
  - still prefers canonical `ready_artifacts` when they materialize first,
  - ignores stale detached publication for a newer still-current target.
- The refreshed detached baseline regression passed and now proves the default two-step contract:
  zero-budget probing stays fail-closed on `not_ready`, then the bounded wait immediately
  attributes the already-published detached artifact as `detached_ready_artifacts` instead of
  falling through to `shadow_state`.
- The hover / exact fail-closed regressions still passed, preserving interactive exact-gate
  behavior outside the `didSave` heavy follow-up path.
- The `p55` leaf report now captures the new representative detached winner shape:
  - `followup_ready_snapshot_zero_probe=not_ready`
  - `followup_ready_snapshot_wait_probe=not_ready`
  - `followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts`
  - `followup_ready_snapshot_timeout_leaf=null`
  - `followup_ready_snapshot_dominant_phase=parse_exec`
  - `followup_semantic_path=detached_ready_artifacts`
- The `p56` representative bundle passed with all four cycles on the new detached bounded-wait
  winner path:
  - `followup_semantic_path_detached_ready_artifacts=4`
  - `followup_ready_snapshot_bounded_wait_winner_detached_ready_artifacts=4`
  - `followup_ready_snapshot_wait_probe_not_ready=4`
  - `followup_ready_snapshot_timeout_leaf_ready_install_count=0`
  - `followup_ready_snapshot_continuation_reason_count=0`
- Strict OpenSpec validation passed.

## Representative Live Artifacts

- `openspec/changes/refactor-46-save-followup-dual-artifact-wait/validation/refactor-46-real-conf-big-diagnostics-ready-snapshot-leaf-live.json`
- `openspec/changes/refactor-46-save-followup-dual-artifact-wait/validation/refactor-46-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`

Representative `p56` cycle snapshot from the generated bundle:

- `cycle=4`
- `followup_semantic_path=detached_ready_artifacts`
- `followup_ready_snapshot_zero_probe=not_ready`
- `followup_ready_snapshot_wait_probe=not_ready`
- `followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts`
- `followup_ready_snapshot_bounded_wait_elapsed_ms=49`
- `followup_ready_snapshot_timeout_leaf=null`
- `semantic_query_dominates_parse_exec=true`

## Requirement -> Code -> Test

- Detached publication becomes a first-class bounded-wait wake source for the exact save target ->
  `backend/src/bin/lsp_server/server/mod.rs`,
  `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`,
  `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` ->
  `p24c_diagnostics_save_timeline_bounded_wait_wakes_on_detached_ready_artifact_publication`
- Detached artifacts no longer bypass bounded-wait attribution through the zero-budget probe ->
  `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`,
  `backend/src/bin/lsp_server/server/core/tests/diagnostics_save_timeline/continuation_and_apply_lag.rs` ->
  `p24b_diagnostics_save_timeline_prefers_detached_ready_artifacts_before_shadow_fallback`,
  `p55_real_conf_big_diagnostics_ready_snapshot_leaf_report_live`
- Canonical `ready_artifacts` still retain priority and stale detached wakeups do not leak across
  targets -> `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` ->
  `p24c_diagnostics_save_timeline_bounded_wait_prefers_ready_artifacts_when_canonical_materializes_first`,
  `p24c_diagnostics_save_timeline_bounded_wait_ignores_stale_detached_publication_for_newer_target`,
  `p24b_diagnostics_save_timeline_ignores_detached_ready_artifacts_from_older_save_cycle`
- Incident bundles expose truthful bounded-wait winner attribution and elapsed fields ->
  `backend/src/bin/lsp_server/types.rs`,
  `backend/src/bin/lsp_server/server/core.rs`,
  `backend/src/bin/lsp_server/server/core/tests/live_reports/ready_snapshot_leaf_live.rs`,
  `backend/src/bin/lsp_server/server/core/tests/live_reports/representative_bundle_live.rs` ->
  `refactor-46-real-conf-big-diagnostics-ready-snapshot-leaf-live.json`,
  `refactor-46-real-conf-big-diagnostics-representative-save-followup-bundle-live.json`
- Interactive exact fail-closed semantics remain preserved -> `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs` ->
  `detached_ready_artifact_does_not_weaken_hover_fail_closed_gate`,
  `same_revision_ready_snapshot_waits_for_exact_type_index_before_hover`

## Residual Notes

- The refreshed `p55` leaf assertions had to drop several stale `refactor-44` assumptions because
  the representative detached path is no longer timeout-shaped once bounded wait races canonical
  ready artifacts against detached publication.
- The current `p56` bundle still reports large `did_change_ready_snapshot_materialization`
  histogram values (`p50=42346`, `p95=43187`). This change does not claim to improve canonical
  `didChange` materialization latency, so those numbers were left as a non-gating observation
  rather than a blocker for `refactor-46`.
