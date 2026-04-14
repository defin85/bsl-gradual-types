# Change: coalesce same-file didChange ready-snapshot production

## Why

Bundle `2026-04-13T19-13-36Z` confirms that the previous observability work is now truthful:

- `didChange` fallback is explicitly classified as `stale_parser_base`;
- `edits_do_not_match_new_content` and `input_edit_conversion_failed` stay at `0`;
- but same-file ready-snapshot churn remains high: `did_change started=14`, `superseded=12`,
  materialized only `2`;
- both `didSave` heavy follow-ups still time out and publish through `shadow_state`.

This means the current system can finally explain the problem, but still does too much obsolete
same-file work before the newest exact revision becomes usable.

## What Changes

- Require file-scoped latest-wins coalescing for `didChange` ready-snapshot production instead of
  spawn-per-revision churn for same-file bursts.
- Require `didSave` heavy follow-up to wait only on an exact still-current coalesced producer and
  to skip waiting for coalesced-away or newer-target producers.
- Require incident-bundle observability to distinguish "retargeted/coalesced before parse or
  materialization" from "worker really started and later timed out/superseded".

## Sequence

This is a follow-up to:

- `refactor-16-did-change-incremental-parse-fallback-attribution`
- `refactor-17-diagnostics-save-inflight-snapshot-preference`
- `refactor-20-diagnostics-save-snapshot-worker-hardening`

Those changes made `didSave` miss classes, exact-task preference, and `stale_parser_base`
observable. This change addresses the remaining root cause: same-file `didChange` snapshot
production still churns through too many obsolete revisions before an exact-ready artifact can help
the current save cycle.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core/snapshot_status.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - live incident-bundle export / diagnostics-save regressions

## Non-Goals

- Do not increase the existing bounded `didSave` wait budget.
- Do not redesign the incremental parser algorithm beyond already-classified
  `stale_parser_base` fallback behavior.
- Do not add duplicate same-version `didSave` parse workers.
- Do not fix build metadata stamping in the binary identity; that is a separate issue.
