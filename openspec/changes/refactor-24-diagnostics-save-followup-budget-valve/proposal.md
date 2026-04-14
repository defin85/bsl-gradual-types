# Change: add a guarded temporary budget valve for didSave exact follow-up wait

## Why

The latest bundle shows a narrow but operationally painful gap:

- exact ready-snapshot work often looks like the right path to wait on;
- the current bounded wait is `3500ms`;
- representative exact materialization in the same incident is around `3720ms`.

Raising the budget blindly would be the wrong fix. It would mask queue/apply problems and make
slow fallbacks even slower.

But once the miss taxonomy and exact-path phase attribution from `refactor-22` and `refactor-23`
exist, the system can make a narrower decision: allow a small temporary relief window only when it
is clearly waiting on the right exact producer and that producer is merely finishing late.

## What Changes

- Require an evidence-gated temporary relief window beyond the base `didSave` ready-snapshot wait
  budget, limited to the exact still-current producer path.
- Require the relief valve to stay off for queue/apply-lag cases, coalesced-away producers, and
  non-exact fallback paths.
- Require explicit observability whenever the temporary valve is engaged, skipped, or proves
  ineffective.

## Sequence

This is the third step after:

- `refactor-22-did-change-parser-base-root-cause-attribution`
- `refactor-23-ready-snapshot-materialization-phase-attribution`

This change is intentionally a temporary operational valve, not the primary root-cause fix.

## Epic

This change is part of Beads epic `bsl-gradual-types-wikt`
(`Epic: didChange ready-snapshot root-cause follow-ups`).

Execution child for this step: `bsl-gradual-types-wikt.3`.
Upstream chain:
- `bsl-gradual-types-wikt.1` blocks `bsl-gradual-types-wikt.2`
- `bsl-gradual-types-wikt.2` blocks `bsl-gradual-types-wikt.3`

Umbrella narrative for the full chain lives in
`openspec/changes/refactor-22-did-change-parser-base-root-cause-attribution/validation/epic-summary.md`.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - readiness metrics / diagnostics-save timeline / incident-bundle export
  - runtime-config plumbing if a rollout gate is needed

## Non-Goals

- Do not replace the base wait budget with an unbounded or permanently larger budget.
- Do not apply the valve to coalesced-away, other-version, or generic fallback producers.
- Do not use the valve as a substitute for parser-base or exact-path optimization work.
