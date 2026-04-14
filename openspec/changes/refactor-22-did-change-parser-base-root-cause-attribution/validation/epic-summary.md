# Epic Summary

## Epic

- Beads epic: `bsl-gradual-types-wikt`
- Title: `Epic: didChange ready-snapshot root-cause follow-ups`
- Triggering evidence: incident bundle `2026-04-14T07-44-27Z`

## Goal

Close the remaining `didChange` / `didSave` exact-ready gap in three ordered steps:

1. Explain why ranged `didChange` falls into `stale_parser_base` instead of cheap parser-base
   reuse.
2. Explain where exact ready-snapshot materialization spends its latency budget.
3. Only then consider a narrow temporary `didSave` budget valve for the proven exact late-path
   case.

## Ordered Delivery Chain

| Order | OpenSpec change | Beads child | Role |
|---|---|---|---|
| 1 | `openspec/changes/refactor-22-did-change-parser-base-root-cause-attribution/` | `bsl-gradual-types-wikt.1` | Classify root causes behind `stale_parser_base` misses |
| 2 | `openspec/changes/refactor-23-ready-snapshot-materialization-phase-attribution/` | `bsl-gradual-types-wikt.2` | Attribute exact ready-snapshot latency by phase and expose phase-at-timeout |
| 3 | `openspec/changes/refactor-24-diagnostics-save-followup-budget-valve/` | `bsl-gradual-types-wikt.3` | Add a guarded temporary relief valve only for the proven exact late-path case |

## Dependency Graph

- `bsl-gradual-types-wikt.1` blocks `bsl-gradual-types-wikt.2`
- `bsl-gradual-types-wikt.2` blocks `bsl-gradual-types-wikt.3`

Interpretation:

- `refactor-22` must land first so the system can say why parser-base reuse was unavailable.
- `refactor-23` must land second so the system can say where exact readiness lost time.
- `refactor-24` is allowed only after the first two steps prove that the dominant residual gap is
  a narrow exact-path late-materialization case rather than queue/apply lag or the wrong producer.

## Current Status

As of 2026-04-14:

- `refactor-22` / `bsl-gradual-types-wikt.1`: planned and unblocked
- `refactor-23` / `bsl-gradual-types-wikt.2`: planned, blocked by step 1
- `refactor-24` / `bsl-gradual-types-wikt.3`: planned, blocked by step 2

## Why This Is One Epic

These three changes address one incident class, not three unrelated improvements:

- the same bundle shows `didSave` bounded wait losing to a late exact path;
- the same bundle shows ranged `didChange` repeatedly falling into `stale_parser_base`;
- the same bundle shows exact ready-snapshot materialization landing just after the base wait
  budget.

Splitting them into separate changes keeps each rollout narrow, but the execution and readiness
story is one epic.

## Exit Criteria

The epic is ready for closure only when all of the following are true:

- `refactor-22` is implemented and proves why parser-base reuse was missed.
- `refactor-23` is implemented and proves where exact readiness spent its time.
- `refactor-24` is implemented only if the prior two steps show that a temporary budget valve is
  justified for a narrow exact-path case.
- OpenSpec validation passes for all three changes.
- Beads statuses for `bsl-gradual-types-wikt.1`, `.2`, and `.3` match the real delivery state.

## Operator Reading Order

For a human or agent reading this epic without `bd`:

1. Read `proposal.md` in `refactor-22`.
2. Read this file.
3. Read `proposal.md` in `refactor-23`.
4. Read `proposal.md` in `refactor-24`.

This preserves the intended evidence-first sequencing.
