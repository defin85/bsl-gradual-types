# Change: explain didSave ready-snapshot misses in the save timeline

## Why

Incident bundle `2026-04-11T18-27-18Z` proves that `didSave + idle_heavy` falls back to
`shadow_state + salsa`, but the current save timeline cannot explain why the exact-version
`ready_artifacts` path was missed.

Today the runtime collapses several materially different situations into the same effective
outcome:

- exact snapshot not built yet;
- exact snapshot built but rejected by version/generation freshness guards;
- exact snapshot wait budget expired;
- request was cancelled or superseded before the wait completed.

Without explicit miss attribution, operators cannot tell whether the next fix belongs in
`didChange` snapshot materialization, freshness guards, or `didSave` branch ordering.

## What Changes

- Require the didSave diagnostics timeline to expose canonical, low-cardinality outcomes for:
  - zero-budget `ready_artifacts` probe;
  - bounded-wait `ready_artifacts` probe;
  - same-version ready-snapshot task state at branch-selection time;
  - `shadow_state` availability at branch-selection time.
- Require additive contract versioning for the diagnostics save timeline so older consumers
  degrade explicitly instead of silently hiding the new attribution.
- Keep the change observability-only: no branch-order or latency behavior change is introduced
  here.

## Sequence

This is the first change in the chain.

It should land before any behavior change, because later changes need authoritative evidence about
why `ready_artifacts` was not selected.

## Epic

This change is part of Beads epic `bsl-gradual-types-1rkq`
(`Epic: didSave snapshot reuse hardening follow-ups`).

Execution child for this step: `bsl-gradual-types-1rkq.1`.
Downstream chain:
- `bsl-gradual-types-1rkq.1` blocks `bsl-gradual-types-1rkq.2`
- `bsl-gradual-types-1rkq.2` blocks `bsl-gradual-types-1rkq.3`

Umbrella narrative for the full chain lives in `validation/epic-summary.md`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts`
  - diagnostics save timeline tests and bundle projections

## Non-Goals

- Do not change `didSave` branch ordering yet.
- Do not change `didChange` parse-snapshot building yet.
- Do not introduce high-cardinality error strings or raw text into incident bundles.
