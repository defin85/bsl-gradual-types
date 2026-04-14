# Change: expose root-cause attribution for ranged didChange parser-base misses

## Why

Bundle `2026-04-14T07-44-27Z` shows that the latest exact `didChange` worker still spends too much
time on the expensive `stale_parser_base -> full parse` path before `didSave` heavy follow-up times
out.

The current observability is truthful but still too coarse:

- `fallback_reason=stale_parser_base` tells us that incremental reuse did not happen;
- it does not tell us whether the miss happened because ready snapshot lagged behind shadow state,
  because a matching ready base never existed, or because tree-cache state still diverged after an
  attempted prime;
- without that distinction the next fix risks optimizing the wrong miss class.

## What Changes

- Require low-cardinality root-cause attribution for ranged `didChange` parser-base reuse misses
  before the system falls back to `stale_parser_base` full parse.
- Require incident bundles and observability payloads to show bounded shadow/ready base state
  around the miss so operators can explain why cheap reuse was unavailable.
- Require regressions and repo-local evidence for the main miss classes that can lead to
  `stale_parser_base`.

## Sequence

This change is the first step after `refactor-21-did-change-ready-snapshot-coalescing`.

It MUST land before:

- `refactor-23-ready-snapshot-materialization-phase-attribution`
- `refactor-24-diagnostics-save-followup-budget-valve`

The goal of this change is understanding, not relief: classify the miss correctly before adding new
timers or temporary wait-budget valves.

## Epic

This change is part of Beads epic `bsl-gradual-types-wikt`
(`Epic: didChange ready-snapshot root-cause follow-ups`).

Execution child for this step: `bsl-gradual-types-wikt.1`.
Downstream chain:
- `bsl-gradual-types-wikt.1` blocks `bsl-gradual-types-wikt.2`
- `bsl-gradual-types-wikt.2` blocks `bsl-gradual-types-wikt.3`

Umbrella narrative for the full chain lives in `validation/epic-summary.md`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - incident-bundle export / observability payload wiring

## Non-Goals

- Do not raise the `didSave` ready-snapshot wait budget.
- Do not redesign the parser algorithm in this change.
- Do not change `didSave` semantic fallback policy yet.
- Do not introduce high-cardinality free-form miss reasons.
