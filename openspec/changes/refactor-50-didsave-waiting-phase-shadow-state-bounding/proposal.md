# Change: bound same-version didSave waiting-phase shadow-state fallback before semantic query dominates

## Status

Superseded for implementation by
`refactor-51-didsave-exact-producer-lane-bounding`.

This change remains the diagnostic framing and acceptance gate for the incident bundle: a
still-current same-version save family MUST NOT be accepted if it reaches
`followup_semantic_path=shadow_state` with `followup_ready_snapshot_timeout_phase=waiting` and the
same run later shows same-family exact readiness. The implementation owner is `refactor-51`, because
the root cause is now understood as missing exact-producer admission/lifecycle ownership rather than
only consumer-side fallback selection.

## Why

The fresh observability incident bundle captured at `2026-04-23T11:00:28.547Z` on
`0.4.159` / `git 500d1352` shows a different residual than the one just closed by `refactor-49`.

The old rebuild seam is no longer the dominant issue:

- completion is healthy on the same bundle: 5 of 6 traces finish in `0-6ms`, and the only
  non-trivial completion outlier is local `collect=192ms`, not transport, ingress, or UI;
- the bundle does not show the old rebuild-dominated
  `parse_exec -> exact_ready_snapshot_assembly -> program_lowering` contour that `refactor-49`
  targeted.

The remaining incident is still in same-version `didSave` heavy follow-up, but its shape changed:

- both diagnostics-save traces finish through `followup_semantic_path=shadow_state`;
- both traces show `followup_ready_snapshot_task_state=in_flight_same_version`,
  `followup_ready_snapshot_zero_probe=not_ready`,
  `followup_ready_snapshot_wait_probe=timeout`, and
  `followup_ready_snapshot_timeout_phase=waiting`;
- both traces keep `save_fastlane` fast (`87-156ms`) yet publish the heavy follow-up only after
  `13.1-13.3s`;
- the expensive wall time is now in shadow-state semantic work:
  `semantic_diagnostics_query_ms=8666` and `9445`;
- cumulative metrics show `ready_snapshot_materialization source=did_save p50/p95=16341ms`, so
  the exact same-version path still materializes later for the same family of requests.

So the next change should not reopen completion transport/UI investigation and should not reopen
the old `program_lowering` rebuild fix. It should narrow the scope to the waiting-only same-version
`didSave` exact producer path: after `save_fastlane` already published, heavy follow-up still times
out in `waiting`, falls back to `shadow_state`, and then burns most of the wall time on semantic
query there.

## What Changes

- Retain a `bsl-intellisense-v2` requirement that same-version `didSave` heavy follow-up MUST NOT
  treat expensive `shadow_state` semantic publication as the steady-state terminal branch solely
  because the still-current exact producer remained in `waiting` after `save_fastlane` already
  published.
- Require representative `examples/conf_big` validation to fail if heavy follow-up still lands on
  `shadow_state` with `timeout_phase=waiting` and query-dominated semantic work while the exact
  same-version path remains current and later materializes for the same request family.
- Defer runtime implementation and representative gate rewiring to
  `refactor-51-didsave-exact-producer-lane-bounding`, which owns the producer-side admission and
  lifecycle fix.
- Keep request-centric diagnostics-save evidence truthful about the distinction between:
  waiting-only exact delay, parse-exec rebuild delay, apply-lag, and semantic-query cost after
  fallback.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/tests/diagnostics_save_timeline/`
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/`
- Follow-up relationship:
  - builds on `refactor-49-save-followup-same-version-ready-snapshot-rebuild-bounding`
  - superseded for implementation by `refactor-51-didsave-exact-producer-lane-bounding`
  - does not reopen completion transport or UI investigation
  - remains narrower than `refactor-current-revision-head-detached-snapshot`

## Non-Goals

- Do not reopen the old rebuild-dominated `program_lowering` seam from `refactor-49`.
- Do not widen bounded wait or relief-valve budgets as the primary remedy.
- Do not weaken canonical interactive exact-readiness gates for completion, hover, definition,
  signatureHelp, or semantically equivalent exact consumers.
- Do not treat `shadow_state` as canonical exact truth for the saved revision.
