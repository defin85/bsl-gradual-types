# Change: bound same-version `didSave` heavy follow-up by isolating the exact producer lane

## Why

`refactor-50-didsave-waiting-phase-shadow-state-bounding` correctly narrowed the remaining live
residual to waiting-only same-version `didSave` follow-up on `examples/conf_big`, but the current
failure shape shows that the bounded contract still lives on the consumer side rather than on the
exact producer side.

This change supersedes `refactor-50` as the implementation track. `refactor-50` remains the
diagnostic framing and representative fail gate; this change owns the producer-side admission,
lifecycle, runtime wiring, tests, and live evidence needed to close that gate.

The current representative contour is:

- `save_fastlane` publishes quickly for the same save family;
- heavy follow-up then observes `followup_ready_snapshot_task_state=in_flight_same_version`;
- bounded wait times out in `waiting`;
- heavy follow-up falls back through `shadow_state`;
- later the same save family still materializes exact readiness.

That means the system still has no first-class bounded admission contract for the same-version
`didSave` exact producer before detached diagnostics-ready publication.

## What Changes

- Add a `bsl-intellisense-v2` requirement that same-version `didSave` exact producers use a
  dedicated save-critical admission boundary, separate from generic interactive request work and
  generic background diagnostics work.
- Require the bounded save-followup contract to terminate at same-family detached
  diagnostics-ready publication rather than at full exact materialization.
- Require producer ownership and lifecycle to stay keyed to the exact
  `(file_id, requested_version, text_hash, save_cycle_sequence)` save family, or a semantically
  equivalent identity.
- Tighten representative acceptance so still-current waiting-only `shadow_state` fallback is no
  longer an allowed steady-state terminal branch when the exact producer simply failed to start or
  publish detached-ready work in time.
- Import the `refactor-50` representative fail condition: a run MUST fail if it observes
  `followup_semantic_path=shadow_state` with `followup_ready_snapshot_timeout_phase=waiting`,
  query-dominated semantic fallback work, and later same-family detached or fully materialized exact
  readiness.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core/tests/startup_and_fastlane/`
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/`
- Follow-up relationship:
  - supersedes `refactor-50-didsave-waiting-phase-shadow-state-bounding` for implementation
  - imports the `refactor-50` waiting-phase `shadow_state` representative fail gate
  - does not reopen `program_lowering` rebuild optimization
  - does not treat `shadow_state` semantic-query optimization as the primary fix
