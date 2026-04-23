## Context

`refactor-49` closed the representative rebuild seam: the accepted `p56` contour no longer falls
into `shadow_state` because of cold
`parse_exec -> exact_ready_snapshot_assembly -> program_lowering`.

The new incident bundle on `git 500d1352` shows a different residual:

- `save_fastlane` still publishes quickly (`87-156ms`);
- the exact same-version producer remains `in_flight_same_version`;
- zero-budget probe is `not_ready`, bounded wait times out at about `3.5s`, and timeout
  attribution is now truthfully `waiting`;
- the heavy follow-up then publishes via `shadow_state`;
- the dominant cost on that branch is semantic query work on the fallback path
  (`semantic_diagnostics_query_ms=8666/9445`);
- cumulative metrics still show later exact same-version materialization for `did_save`
  (`p50/p95=16341ms`).

So the open seam is no longer "exact rebuild is too cold". It is "waiting-only exact continuity is
still too weak, so the system spends the user's wall time on expensive shadow-state semantic work
before the same save family reaches exact readiness."

`refactor-50` is now diagnostic framing, not the implementation owner. The implementation track is
`refactor-51-didsave-exact-producer-lane-bounding`, because the reviewed runtime surface showed that
consumer-side suppression of `shadow_state` can still fall through to another generic path while the
same-version exact producer lacks first-class bounded admission and lifecycle ownership.

## Goals

- Bound same-version `didSave` waiting-phase fallback before expensive `shadow_state` semantic
  publish becomes the steady-state outcome.
- Preserve this waiting-phase fallback shape as the representative fail gate that `refactor-51`
  must close.
- Preserve exact same-version truth, latest-wins semantics, and fail-closed interactive behavior.
- Keep diagnostics-save evidence truthful about whether the primary blocker is waiting-phase exact
  delay, parse-exec rebuild, apply-lag, or semantic-query cost after fallback.

## Non-Goals

- Do not implement a separate consumer-side workaround in this change after `refactor-51` becomes
  the producer-side implementation owner.
- Do not reopen `program_lowering`-class rebuild optimization as the main diagnosis.
- Do not blame VS Code UI, client probes, or completion transport for this incident family.
- Do not satisfy the change merely by increasing timeout or relief-valve budgets.
- Do not turn `shadow_state` semantic output into canonical exact readiness.

## Decision

### 1. Treat waiting-only exact delay as its own save-followup failure class

After same-version `save_fastlane` already published, a still-current exact producer that remains
only in `waiting` is a different failure class from the already-fixed rebuild-dominated exact path.

The runtime and validation need to reason about that class directly instead of implicitly treating
it as either parse-exec work or generic fallback noise.

### 2. Avoid expensive terminal shadow-state publish as the steady-state outcome

If the current same-version exact producer is still provably the newest valid candidate, heavy
follow-up should not default to a full semantic `shadow_state` publish merely because the bounded
wait elapsed before the exact worker left `waiting`.

The concrete implementation may involve stronger still-current continuity, better wake/proof
handoff, or another exact-safe path. The design requirement is narrower: expensive
`shadow_state` semantic publication must stop being the default terminal branch for this
waiting-only shape.

### 3. Representative acceptance must fail on waiting-phase shadow-state semantic dominance

The representative `examples/conf_big` gate should now fail when all of these are true at once:

- `save_fastlane` already published quickly for the same save family;
- the exact producer remains `in_flight_same_version`;
- timeout attribution is `waiting`, not rebuild-stage parse-exec;
- heavy follow-up still publishes through `shadow_state`;
- and semantic query on that fallback path dominates the wall time while exact materialization for
  the same family still occurs later.

This keeps the next change tied to the new live residual instead of stretching `refactor-49`.

This acceptance rule is carried forward into `refactor-51`: the representative gate must fail if a
still-current save family reaches `shadow_state` from the waiting phase and the same run later proves
same-family detached or fully materialized exact readiness.

## Alternatives Considered

### Widen bounded wait / relief-valve budgets

Rejected. The bundle already proves the expensive wall time moved into fallback semantic query.
Bigger budgets would blur the residual instead of removing it.

### Reopen `program_lowering` optimization

Rejected. The new bundle does not show rebuild-dominated `parse_exec/program_lowering`. Reopening
that seam would chase the wrong bottleneck.

### Optimize shadow-state semantic query in isolation

Rejected as a primary framing. The query is expensive, but the user should not be forced onto that
fallback branch as the steady-state terminal outcome while the exact same save family remains
current and later materializes.

## Risks

### Risk: a fix hides the waiting seam by relabeling the path rather than changing behavior

Mitigation:

- keep representative validation tied to terminal path, timeout phase, and query cost;
- require evidence that distinguishes waiting-only exact delay from rebuild-stage delay.

### Risk: reducing shadow-state fallback weakens truthful supersession or fail-closed semantics

Mitigation:

- keep the behavior keyed to exact `(file_id, requested_version, text_hash, save_cycle_sequence)`
  identity or a semantically equivalent save-cycle identity;
- preserve truthful fallback whenever a newer revision/save cycle overtakes the target or the
  runtime can no longer prove still-current exact continuity.
