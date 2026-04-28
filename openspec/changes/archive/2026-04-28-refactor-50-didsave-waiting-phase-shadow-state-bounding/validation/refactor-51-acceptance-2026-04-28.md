# Refactor-50 acceptance via refactor-51

`refactor-50-didsave-waiting-phase-shadow-state-bounding` is a diagnostic/audit-trail change.
Its implementation owner is archived change
`openspec/changes/archive/2026-04-28-refactor-51-didsave-exact-producer-lane-bounding/`.

## Evidence

- `refactor-51` task 1.3 imported the `refactor-50` fail gate: waiting-phase timeout,
  `shadow_state` terminal path, query-dominated semantic fallback, and later same-family exact
  readiness.
- `refactor-51` tasks 3.1-3.5 implemented the producer-side path: dedicated same-version
  `didSave` exact-producer admission/lifecycle, detached diagnostics-ready publication, truthful
  follow-up waiting/wakeup observability, and representative fail-gate expectations.
- Runtime artifacts include:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `bsl-runtime/src/system/basic_observability/runtime_metrics.rs`
- Regression artifacts include:
  - `backend/src/bin/lsp_server/server/core/tests/startup_and_fastlane/same_version_followup_waits.rs`
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/representative_bundle_live.rs`
- Archived representative evidence:
  `openspec/changes/archive/2026-04-28-refactor-51-didsave-exact-producer-lane-bounding/validation/refactor-51-real-conf-big-diagnostics-representative-save-followup-bundle-live.md`.

## Representative result

The archived representative `examples/conf_big` run for `refactor-51` recorded:

- 4/4 cycles with `followup_semantic_path=detached_ready_artifacts`;
- 4/4 cycles with `followup_ready_snapshot_bounded_wait_winner=detached_ready_artifacts`;
- 4/4 cycles with
  `followup_did_save_exact_producer_lifecycle_state=detached_diagnostics_ready_published`;
- 0 cycles with `followup_semantic_path=shadow_state`;
- 0 bounded waits timed out.

This closes the `refactor-50` waiting-phase `shadow_state` gate without adding a second runtime
workaround in `refactor-50`.

## Current-turn validation

- `cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_promotes_already_queued_exact_worker_before_shadow_fallback -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_prefers_inflight_same_version_ready_snapshot_before_shadow_state -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p7_did_save_followup_skips_bounded_wait_after_exact_producer_is_retargeted_away -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p26_diagnostics_save_timeline_keeps_skipped_apply_lag_for_waiting_exact_phase -- --nocapture`
- `openspec validate refactor-50-didsave-waiting-phase-shadow-state-bounding --strict --no-interactive`

Exploratory non-evidence: `p7_did_save_followup_uses_detached_ready_artifacts_when_only_did_save_refresh_task_exists`
currently times out while waiting for the initial `didChange` applied state and ready parse snapshot,
before the `didSave` follow-up path under review.
