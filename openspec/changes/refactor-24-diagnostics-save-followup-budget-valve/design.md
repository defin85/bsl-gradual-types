## Context

After `refactor-22` and `refactor-23`, the runtime should be able to prove two things at
`didSave` timeout:

- whether it was waiting on the right exact producer;
- which exact-path phase was still in progress.

Only after that evidence exists does a temporary budget valve become defensible. The current bundle
suggests the exact path can finish just after the base `3500ms` budget, but the same bundle also
shows that blindly waiting longer would be wrong for queue/apply-lag or coalesced-away paths.

## Goals / Non-Goals

- Goals:
  - provide temporary operator relief when the runtime is demonstrably waiting on the right exact
    producer and that producer is merely late;
  - keep the valve narrow, bounded, and explicitly observable;
  - make it removable once the root cause is fixed.
- Non-Goals:
  - no permanent budget increase;
  - no relief for queue/apply-lag cases;
  - no masking of coalesced-away or wrong-version producers.

## Decisions

### Decision: the valve applies only after the base budget is exhausted on an exact producer

The base budget remains the primary contract. The temporary valve may extend waiting only after the
runtime has already proven:

- the producer still matches the exact `(file_id, requested_version, text_hash)`;
- the producer was not retargeted away;
- the incident is not explained by runtime queue wait or apply lag.

### Decision: the valve must be self-attributing

Every use of the valve should leave an explicit trace in metrics and bundle export:

- valve engaged;
- valve skipped because proof was absent;
- valve ineffective because the extra window still timed out.

This is necessary so the temporary relief does not hide the remaining root cause.

### Decision: the valve is a temporary operational tool with sunset criteria

The proposal should define removal criteria up front. The valve should be disabled or removed once
the exact-path p95 fits comfortably under the base budget on representative live evidence.

## Alternatives Considered

### Raise the base budget for everyone

Rejected. That would hide queue/apply regressions and make truthful fallback slower.

### Never add any temporary relief

Rejected for now. The bundle already shows a narrow gap where exact readiness appears to lose by a
few hundred milliseconds, which creates real operator pain while root-cause work is still pending.

## Risks / Trade-offs

- A too-permissive valve would become a silent permanent budget increase.
- A too-small valve may add complexity without meaningful operator relief.
- Rollout gating must not create a second undocumented contract.

## Migration Plan

1. Reuse the exact-path proof and phase attribution added by `refactor-22` and `refactor-23`.
2. Add the temporary valve behind explicit runtime gating if needed.
3. Compare live evidence with valve off vs on for the same save-cycle profile.
4. Remove or disable the valve once the base budget becomes sufficient again.
