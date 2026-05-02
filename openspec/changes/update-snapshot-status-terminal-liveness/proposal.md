# Change: update snapshot status terminal liveness

## Why
The live VS Code status bar can remain stuck at `BSL Snap: building` or `shadow-only` for an active BSL document even after backend work has moved on. The 2026-05-01 incident for `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` showed the last client-visible snapshot status as `building requested=v38 ready=v36`, while backend logs later showed exact type-index precompute and diagnostics publication for `v38`.

This makes the new snapshot readiness UI truthful at the field level but not live enough as an operator signal: a `building` notification needs a terminal transition, a supersession transition, or an explicit failure, not an indefinite last-known state.

## What Changes
- Tighten the snapshot-status runtime contract so any observed `building` state for a requested revision has a bounded terminal lifecycle.
- Ensure backend worker cleanup, retargeting, supersession, exact-index deadline, and failure paths refresh snapshot status after the authoritative task/artifact state changes.
- Ensure live notification coalescing cannot suppress a meaningful `building` to terminal transition.
- Ensure `bsl/getSnapshotStatus` remains a repair/read-through source of truth when a notification is missed.
- Add regression coverage for the incident shape: `building requested=vN ready=vN-2` followed by backend exact/diagnostics progress must not leave the client stuck on the old building state.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/core/snapshot_status.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/tests/snapshot_status_and_perf/snapshot_status.rs`
  - representative incident/readiness tests or perf evidence
  - optional VS Code snapshot-status tests if client stale-update behavior needs adjustment

## Dependencies
- Builds on `add-snapshot-readiness-diagnostics-view`.
- Must be completed before `update-local-variable-member-completion-children`, because member completion failures under a hanging exact/current-head snapshot cannot be interpreted cleanly.

## Non-Goals
- Do not change local-variable type inference or member completion result semantics.
- Do not make `shadow_only` look exact-ready.
- Do not synthesize readiness from diagnostics timelines, completion timelines, logs, or aggregate metrics.
- Do not remove notification coalescing; only prevent it from hiding semantic lifecycle transitions.
