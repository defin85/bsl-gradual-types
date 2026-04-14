# Change: attribute phase latency inside ready-snapshot materialization

## Why

The latest bundle shows `did_change` ready-snapshot materialization around `3720ms`, but the
current metrics still collapse too much of that path into one end-to-end number.

That is not enough to answer the next engineering question:

- is the time spent in blocking parse execution;
- in cancellation / retarget granularity after parse completes;
- in a post-parse window before ready install;
- or in symbol / outline side-work that should not be blamed on exact readiness.

Without phase attribution the team can see the pain, but cannot tell which phase should be
optimized next.

## What Changes

- Require phase-level latency attribution for the exact ready-snapshot producer from parse start to
  ready install/materialization.
- Require incident bundles to expose the producer phase observed at `didSave` timeout, plus the
  dominant phase duration.
- Require symbol / outline side-work to remain separately attributable from exact readiness.

## Sequence

This is the second step after:

- `refactor-22-did-change-parser-base-root-cause-attribution`

It MUST land before:

- `refactor-24-diagnostics-save-followup-budget-valve`

This change measures the exact-path cost before any temporary wait-budget relief is considered.

## Epic

This change is part of Beads epic `bsl-gradual-types-wikt`
(`Epic: didChange ready-snapshot root-cause follow-ups`).

Execution child for this step: `bsl-gradual-types-wikt.2`.
Upstream/downstream chain:
- `bsl-gradual-types-wikt.1` blocks `bsl-gradual-types-wikt.2`
- `bsl-gradual-types-wikt.2` blocks `bsl-gradual-types-wikt.3`

Umbrella narrative for the full chain lives in
`openspec/changes/refactor-22-did-change-parser-base-root-cause-attribution/validation/epic-summary.md`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - incident-bundle export / readiness metrics / live regressions

## Non-Goals

- Do not change parser-base selection policy in this change.
- Do not increase the `didSave` bounded wait budget in this change.
- Do not move documentSymbol logic back onto the async runtime.
