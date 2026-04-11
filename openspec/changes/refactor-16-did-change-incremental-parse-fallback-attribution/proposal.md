# Change: classify didChange incremental-parse fallback causes

## Why

The same incident bundle shows that the parse-snapshot path on `didChange` did not produce any
`incremental` or `reused` builds:

- `parse_snapshot_total`: `full=3`, `incremental=0`, `reused=0`;
- `parse_snapshot_fallback`: `no_previous_tree=1`, `incremental_failed=2`.

This is enough to suspect cold-start plus broken incremental handoff, but it is not enough to
decide which concrete defect to fix. The current observability collapses all incremental failures
into a generic bucket and does not preserve request/version-bound context such as base-text source
or change shape.

## What Changes

- Require a stable, bounded taxonomy for didChange parse-snapshot fallback reasons instead of
  generic `incremental_failed`.
- Require version-bound attribution for parse-snapshot builds, including:
  - base-text source used to derive edits;
  - change-shape classification;
  - canonical fallback reason when incremental parsing does not succeed.
- Require incident-bundle evidence that lets operators correlate didChange parse fallback with a
  later didSave `ready_artifacts` miss without reading logs or source code.
- Keep this change diagnostic-only: it does not attempt to fix the incremental algorithm yet.

## Sequence

This is the second change in the chain.

It should land after `refactor-15-diagnostics-save-ready-snapshot-miss-attribution`, because the
didSave side must first become explicit about why `ready_artifacts` was missed. After that, this
change narrows the didChange-side root cause.

## Epic

This change is part of Beads epic `bsl-gradual-types-1rkq`
(`Epic: didSave snapshot reuse hardening follow-ups`).

Execution child for this step: `bsl-gradual-types-1rkq.2`.
Dependency order:
- blocked by `bsl-gradual-types-1rkq.1`
- blocks `bsl-gradual-types-1rkq.3`

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - parse-snapshot observability and incident-bundle export paths

## Non-Goals

- Do not change incremental parsing behavior in this change.
- Do not introduce raw parser error strings into user-facing bundle payloads.
- Do not widen didChange into a heavy semantic path.
