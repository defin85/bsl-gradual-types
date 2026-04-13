# Change: harden didSave exact snapshot reuse against worker starvation

## Why

The incident bundle from `2026-04-12T22-17-44Z` shows that `didSave + idle_heavy` still misses
same-version ready artifacts even after `refactor-17` changed the branch order for the
`in_flight_same_version` case.

The new lifecycle metrics narrow the residual failure class:

- `did_change` ready-snapshot workers start repeatedly for the target file/version;
- almost all of them terminate without materialization as `aborted`;
- no `did_save` worker starts for the same version;
- `didSave` bounded wait still times out and falls back to `shadow_state + salsa`;
- `bsl.getCurrentContext` continues to parse through `parser_coordinator` instead of consuming a
  ready snapshot.

This points to a deeper root cause than branch ordering alone: the exact same-version
ready-snapshot worker that `didSave` waits on is still too easy to starve or abort before
materialization, and auxiliary current-context parsing keeps competing for the same parse
resources instead of reusing that in-flight work.

## What Changes

- Replace abort-only supersession of background ready-snapshot workers with cooperative control so
  obsolete `didChange` workers stop before they continue consuming blocking/parser capacity.
- Require `didSave` heavy follow-up to promote an existing exact same-version ready-snapshot worker
  before falling back to `shadow_state`, instead of spawning redundant `didSave` parse work or
  depending on abort-only cleanup.
- Require `bsl.getCurrentContext` to reuse or briefly await an equivalent exact same-version
  snapshot worker before launching an independent `parser_coordinator` parse.
- Keep snapshot-backed install on the background writer path; this change fixes materialization
  starvation and duplicate parse pressure, not current-revision handoff semantics.

## Sequence

This is a follow-up to:

- `refactor-15-diagnostics-save-ready-snapshot-miss-attribution`
- `refactor-17-diagnostics-save-inflight-snapshot-preference`

Those changes made the miss class visible and reordered `didSave` for the exact-task case. This
change addresses the newly observed root cause: the exact same-version worker itself is not yet
stable enough to help the same save cycle under parser contention.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - ready-snapshot worker, diagnostics save, and current-context regressions

## Non-Goals

- Do not increase the existing bounded `didSave` wait budget.
- Do not introduce a second same-version `didSave` parse worker for identical text/version.
- Do not move snapshot-backed `SetFileWithSnapshot` install onto the interactive writer queue.
- Do not guarantee that every mixed-load live profile materializes exact same-version ready
  artifacts before the existing wait deadline; truthful `shadow_state` fallback remains valid when
  parser-coordinator contention still wins.
- Do not redesign `didChange` ranged replay, fallback taxonomy, or current completion transport
  admission in this change.
