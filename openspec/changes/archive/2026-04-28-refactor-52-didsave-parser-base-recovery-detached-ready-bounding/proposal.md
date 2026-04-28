# Change: bound started didSave parser-base recovery before detached-ready fallback

## Why

The fresh observability incident bundle captured at `2026-04-24T00:33:15.387Z` on
`0.4.159` / `git 070aa428` shows a new residual after
`refactor-51-didsave-exact-producer-lane-bounding`.

`refactor-51` closed the representative p56 gate: same-version `didSave` follow-up reached
`detached_ready_artifacts` as the bounded winner in the checked-in live run. The new bundle is not
that old waiting-only producer-admission failure. It shows the exact producer already started, but
the bounded follow-up still times out inside `parser_base_recovery`, falls back to
`shadow_state`, and spends seconds on semantic diagnostics query.

Current evidence:

- completion is not the primary bottleneck: client pre-write and transport/dispatch waits stay in
  the `0-2ms` range for the captured completion requests, with the only non-trivial request
  dominated by local `collect`;
- both `didSave` cycles publish fast same-version `save_fastlane` first refreshes
  (`76ms` and `53ms`);
- both heavy follow-ups publish only after `8244ms` / `8871ms`;
- both follow-ups have `followup_ready_snapshot_task_state=in_flight_same_version`;
- both follow-ups export `followup_did_save_exact_producer_lifecycle_state=started`;
- both bounded waits time out at `3500ms` with
  `followup_ready_snapshot_timeout_leaf=parser_base_recovery`;
- both terminal paths are `followup_semantic_path=shadow_state`;
- semantic fallback work is still expensive (`3488ms` / `4110ms`);
- cumulative metrics show `ready_snapshot_materialization source=did_save count=2 p50=4591ms`,
  but the per-cycle timeline does not prove whether that later materialization belongs to the same
  save family after timeout.

So the next change should not reopen VS Code UI, transport ingress, generic waiting-only producer
admission, or detached artifact availability. It should bind the started same-version `didSave`
producer through the `parser_base_recovery -> detached diagnostics-ready` handoff, and make the
representative gate fail when a started same-family producer times out in parser-base recovery and
then publishes through `shadow_state`.

## What Changes

- Add a `bsl-intellisense-v2` requirement that a still-current same-version `didSave` exact
  producer which has already reached `started` MUST either publish detached diagnostics-ready within
  the bounded parser-base contract or terminate with a truthful per-cycle reason such as
  supersession, lost continuity, or failure.
- Tighten representative validation so `started -> parser_base_recovery timeout -> shadow_state`
  is not accepted as a steady-state terminal branch when the save family remains current and no
  truthful terminal producer reason exists.
- Require per-cycle observability to keep following the same save-family producer after timeout or
  fallback, so later detached/full exact readiness can be tied to the concrete
  `(file_id, requested_version, text_hash, save_cycle_sequence)` family instead of inferred only
  from cumulative metrics.
- Preserve the `refactor-51` success endpoint: detached diagnostics-ready publication remains the
  bounded success condition for diagnostics follow-up; full live exact install is still not
  required for this diagnostics-only path.
- Preserve the existing non-goal: this is a backend diagnostics save-followup residual, not a
  VS Code UI or transport-first investigation.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/`
  - ready-snapshot producer lifecycle / parser-base recovery coordination surfaces
  - observability incident-bundle diagnostics-save projection
- Follow-up relationship:
  - follows `refactor-51-didsave-exact-producer-lane-bounding`;
  - reopens the `parser_base_recovery` residual only for the new `070aa428` started-producer
    contour, not the old waiting-only `refactor-50` contour;
  - builds on the archived `refactor-43-save-critical-parser-base-recovery-bounding` contract;
  - builds on the archived `refactor-44-save-followup-detached-ready-artifacts` detached
    diagnostics-ready boundary;
  - does not reopen completion transport/runtime isolation or VS Code extension pre-send work.

## Non-Goals

- Do not widen bounded wait or relief-valve budgets as the primary remedy.
- Do not optimize `shadow_state` semantic query first; the steady-state target is to avoid this
  fallback when a same-family exact producer is still current and already started.
- Do not weaken canonical live exact readiness for completion, hover, definition, signatureHelp,
  type-at-position, or semantically equivalent interactive consumers.
- Do not treat cumulative materialization metrics as enough proof of same-cycle success without
  per-cycle producer identity and terminal lifecycle evidence.
- Do not start in `vscode-extension/` request dispatch code unless a newer bundle contradicts the
  current client/transport evidence.
