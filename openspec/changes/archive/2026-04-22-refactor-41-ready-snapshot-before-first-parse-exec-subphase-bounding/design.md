## Context

The `2026-04-18T18:52:50Z` incident bundle on `f3e72b9e` confirms that the previous save-follow-up
routing regression is fixed, but the representative latency family is still not healthy.

The bundle now shows:

- `followup_semantic_path | ready_artifacts=2 | shadow_state=0`;
- raw timeline
  `followup_ready_snapshot_continuation_reason=continued_still_current`;
- `followup_publish_elapsed_ms=5219` and `5052`;
- `followup_ready_snapshot_parse_exec_ms=3327` and `3222`;
- `followup_ready_snapshot_timeout_leaf=before_first_parse_exec_subphase`;
- `semantic_diagnostics_query_ms=467` and `459`.

Cumulative metrics agree with the same class:

- `ready_parse_snapshot_materialization_ms origin=did_change | p50=3226 | p95=3329`;
- `ready_parse_snapshot_phase_ms origin=did_change phase=parse_exec | p50=3222 | p95=3327`;
- `ready_snapshot_probe slot=bounded_wait outcome=timeout | count=2`;
- `ready_snapshot_probe slot=relief_valve outcome=timeout | count=2`;
- `diagnostics_save_followup_ready_snapshot_continuation_total reason=continued_still_current | count=2`.

So the representative path now stays exact enough to recover through `ready_artifacts`, and the
diagnostics-only semantic query is no longer the main residual. The remaining issue is earlier:
the same-version exact producer still spends most of its representative save-critical time in an
opaque `parse_exec` residence before the first bounded subphase callback appears.

That also means this change cannot safely assume that a later checkpoint such as
`ready_snapshot_assembly -> program_lowering` is still the dominant first branch. Current truthful
timeout evidence stops earlier.

## Goals / Non-Goals

- Goals:
  - reduce representative `didSave` follow-up latency by bounding the opaque pre-subphase
    `parse_exec` residence of the still-current exact producer;
  - preserve current `ready_artifacts` terminal behavior and truthful
    `continued_still_current` continuity semantics;
  - preserve exact same-version artifacts, latest-wins supersession, and truthful cancellation or
    retarget behavior;
  - refresh representative evidence directly against the `2026-04-18T18:52:50Z` baseline.
- Non-Goals:
  - re-open the `shadow_state` fallback-routing contract already addressed by `refactor-39`;
  - re-open diagnostics-only semantic-query optimization already addressed by `refactor-40`;
  - start from `vscode-extension/` or client/UI latency surfaces;
  - assume that `program_lowering` is still the first justified optimization target without
    refreshed proof after the pre-subphase residual is reduced or truthfully subdivided;
  - satisfy the change by merely widening wait or relief-valve budgets.

## Decisions

### 1. Treat this as a pre-subphase `parse_exec` entry problem

The new bundle proves that the same-version exact producer often remains current and eventually
wins. The remaining regression is therefore not "wrong terminal path" and not "diagnostics-only
query still dominates". It is "the producer stays too long in the unproductive front edge of
`parse_exec` before the first bounded callback."

The contract should target that front edge directly.

### 2. Do not pre-commit to a later checkpoint

Previous `p55` evidence justified targeting `program_lowering`, and `refactor-35` remains a valid
active change for that residual. But the new real incident bundle no longer proves that
`program_lowering` is the first live bottleneck on this family, because both timeout traces still
terminate at `before_first_parse_exec_subphase`.

So the first branch for this change must reduce or split the pre-subphase residence itself before
assuming the later assembly/lowering branch is still dominant on the same real path.

Acceptable shapes include:

- reducing the producer setup or entry cost before the first subphase callback;
- introducing truthful bounded internal progress inside the currently opaque region;
- or otherwise reaching the first publishable exact state earlier without weakening exactness.

### 3. Preserve the current latency envelope

The bundle already shows:

- bounded wait timeout;
- relief-valve timeout;
- eventual success through `continued_still_current`.

That means "wait longer" is not a truthful solution. The change must keep the current bounded wait
and relief-valve budgets as the primary envelope and reduce or truthfully subdivide the work
inside it.

### 4. Keep exactness and latest-wins semantics fail-closed

The still-current producer is only valuable because it remains exact and current for the same
target. The implementation is not allowed to:

- keep an obsolete target alive merely to avoid pre-subphase attribution;
- publish stale exact artifacts;
- weaken supersession, retarget, or cancellation behavior.

### 5. Acceptance is representative-bundle-first

Synthetic regressions are required, but they are not enough.

Acceptance must refresh representative evidence against the `2026-04-18T18:52:50Z` bundle and
compare at least:

- `followup_publish_elapsed_ms`;
- `followup_ready_snapshot_parse_exec_ms`;
- ready-snapshot materialization latency for `did_change`;
- terminal path incidence (`ready_artifacts` vs `shadow_state`);
- the remaining timeout leaf / continuation reason, if any.

If refreshed evidence still spends representative save-critical latency in
`before_first_parse_exec_subphase`, the change is not ready.

## Alternatives Considered

### 1. Keep optimizing diagnostics-only semantic work first

Rejected.

The new bundle already reduced `semantic_diagnostics_query_ms` to `459-467 ms`, while the exact
producer still spends `3222-3327 ms` in `parse_exec`.

### 2. Treat `refactor-35` as sufficient and avoid a new change

Rejected for now.

`refactor-35` is a legitimate parser-side optimization, but the current real incident bundle does
not yet prove that a later checkpoint such as `program_lowering` is still the first justified live
target on this family.

### 3. Satisfy the issue by widening wait budgets

Rejected.

That would only relabel the same slow path and weaken the latency envelope without proving earlier
exact progress.

## Validation Strategy

- Add targeted backend/runtime regressions for:
  - a still-current same-version producer that would otherwise remain in
    `before_first_parse_exec_subphase`;
  - truthful supersession or retarget when a newer target overtakes the producer before first
    bounded progress;
  - non-stale exact publish after the early `parse_exec` path is reworked.
- Refresh representative real-module evidence against the `2026-04-18T18:52:50Z` baseline.
- Run strict OpenSpec validation before handoff.

## Quality Gates

- Refreshed representative evidence still publishes through `ready_artifacts`.
- Refreshed representative evidence no longer spends the steady-state save-critical latency inside
  `before_first_parse_exec_subphase`.
- Exact same-version semantics and latest-wins supersession remain intact.
- If representative numbers only improve by widening wait budgets or by hiding the same work under
  another label, the change is not ready.
