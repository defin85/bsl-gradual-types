## Context

The current didSave heavy follow-up uses a fail-closed order that is safe but pessimistic for one
specific case: an exact same-version snapshot task is already in flight and likely to finish within
the existing wait budget, yet `shadow_state` is consumed before that bounded wait is attempted.

This change is only justified if the new observability proves that this case is real and material.

## Goals

- Let the same save cycle benefit from an already-known in-flight exact snapshot.
- Keep fallback immediate when no exact same-version task exists.
- Preserve supersession, cancellation, and no-stale-publish guarantees.

## Non-Goals

- No new unbounded waits.
- No speculative use of stale or cross-version snapshots.
- No generic slowdown of didSave when the exact snapshot path is not actually in flight.

## Decisions

### 1. Reorder only under explicit same-version task evidence

The runtime should only change the branch order when it can prove that a same-version
ready-snapshot task is currently in flight for the requested revision.

Without that evidence, the current immediate truthful fallback remains correct.

### 2. The bounded wait remains bounded

This optimization should reuse the existing wait budget and should not create a longer or
unbounded stall before truthful fallback.

### 3. Freshness guards still dominate

Even after waiting, the runtime must still reject stale, superseded, cancelled, or mismatched
snapshots. The optimization changes ordering, not correctness rules.

## Risks

- If task-state evidence is wrong, the runtime could waste time waiting for a snapshot that cannot
  become usable.
- If the optimization is applied too broadly, didSave could regress when no exact snapshot path is
  actually available.

## Mitigations

- Gate the reordered branch on exact same-version task evidence.
- Preserve immediate fallback for absent, mismatched, stale, or cancelled task states.
