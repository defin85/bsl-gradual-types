# Epic Summary

## Epic

- Beads epic: `bsl-gradual-types-1rkq`
- Title: `Epic: didSave snapshot reuse hardening follow-ups`
- Triggering evidence: incident bundle `2026-04-11T18-27-18Z`

## Goal

Close the remaining `didSave` snapshot-reuse diagnosis and delivery gap in three ordered steps:

1. Make `didSave` explain why exact same-version `ready_artifacts` was not selected.
2. Make `didChange` explain why exact same-version parse snapshots did or did not materialize in time.
3. Only then change `didSave` branch behavior for the exact case where a same-version snapshot task is already in flight.

## Ordered Delivery Chain

| Order | OpenSpec change | Beads child | Role |
|---|---|---|---|
| 1 | `openspec/changes/refactor-15-diagnostics-save-ready-snapshot-miss-attribution/` | `bsl-gradual-types-1rkq.1` | Save-timeline attribution for `ready_artifacts` misses |
| 2 | `openspec/changes/refactor-16-did-change-incremental-parse-fallback-attribution/` | `bsl-gradual-types-1rkq.2` | didChange parse-snapshot fallback attribution |
| 3 | `openspec/changes/refactor-17-diagnostics-save-inflight-snapshot-preference/` | `bsl-gradual-types-1rkq.3` | Bounded behavior change for in-flight exact snapshot preference |

## Dependency Graph

- `bsl-gradual-types-1rkq.1` blocks `bsl-gradual-types-1rkq.2`
- `bsl-gradual-types-1rkq.2` blocks `bsl-gradual-types-1rkq.3`

Interpretation:
- `refactor-15` must land first so the system can say why `ready_artifacts` lost.
- `refactor-16` must land second so the system can say why `didChange` failed to materialize a usable exact snapshot.
- `refactor-17` is allowed only after the first two steps prove that the dominant residual miss class is "same-version snapshot task exists or is imminently ready, but current didSave branch ordering commits to `shadow_state` too early".

## Current Status

As of 2026-04-11:

- `refactor-15` / `bsl-gradual-types-1rkq.1`: implemented and validated
- `refactor-16` / `bsl-gradual-types-1rkq.2`: planned, blocked by completed step 1 evidence
- `refactor-17` / `bsl-gradual-types-1rkq.3`: planned, still blocked by step 2

## Why This Is One Epic

These three changes address one incident class, not three unrelated improvements:

- the same bundle showed `didSave` falling back to `shadow_state + salsa`;
- the same bundle showed `didChange` producing `full=3`, `incremental=0`, `reused=0`;
- the behavior change in step 3 is only safe once steps 1 and 2 make the miss class explicit.

Splitting them into separate changes keeps each rollout narrow, but the execution and readiness story is one epic.

## Exit Criteria

The epic is ready for closure only when all of the following are true:

- `refactor-15` is implemented and proves why `ready_artifacts` was missed on `didSave`.
- `refactor-16` is implemented and proves why `didChange` incremental parsing fell back.
- `refactor-17` is implemented only after the new evidence shows the exact-task-in-flight case is real and material.
- OpenSpec validation passes for all three changes.
- Beads statuses for `bsl-gradual-types-1rkq.1`, `.2`, and `.3` match the real delivery state.

## Operator Reading Order

For a human or agent reading this epic without `bd`:

1. Read `proposal.md` in `refactor-15`.
2. Read this file.
3. Read `proposal.md` in `refactor-16`.
4. Read `proposal.md` in `refactor-17`.

This preserves the intended evidence-first sequencing.
