## Context

`refactor-32` reduced exact-head lag, and `refactor-36` moved semantic diagnostics off the full
semantic-facts path. The fresh incident bundle on `468658b1` still shows that the representative
`didSave` heavy follow-up is not stable on the exact path:

- `diagnostics-save-trace-1` publishes through `ready_artifacts`, but only after
  `followup_ready_snapshot_parse_exec_ms=2799`;
- `diagnostics-save-trace-2`, `3`, and `4` all stay on
  `followup_ready_snapshot_task_state=in_flight_same_version`,
  hit `followup_ready_snapshot_wait_probe=timeout`,
  record `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`,
  and terminate on `shadow_state`;
- those three traces report `followup_ready_snapshot_parse_exec_ms=2742`, `1919`, and `2522`;
- the bundle-level summary is `followup_semantic_path | ready_artifacts=1 | shadow_state=3`.

This is not primarily an observability gap and not primarily a diagnostics-only builder problem.
The incident class is a still-current exact producer that remains alive inside `parse_exec`, but
the save follow-up still defaults to `shadow_state`.

## Goals / Non-Goals

- Goals:
  - reduce representative `timeout -> engaged_timed_out -> shadow_state` incidence for
    same-version `didSave` heavy follow-up;
  - keep one still-current exact producer viable enough that `shadow_state` becomes an exceptional
    fallback rather than the representative terminal branch on this save profile;
  - preserve exactness, latest-wins supersession, and truthful operator evidence;
  - refresh representative incident evidence against the `2026-04-17T14:06:03Z` baseline.
- Non-Goals:
  - optimize diagnostics-only semantic-facts leaves covered by `refactor-38`;
  - tune VS Code UI / extension dispatch or other client-side latency surfaces;
  - redesign generic `didChange` parser-base recovery beyond what is necessary for this specific
    `didSave` timeout/fallback class.

## Decisions

### 1. Treat this as a same-version producer continuity problem

The failing traces do not show a missing exact producer. They show a still-current exact producer
remaining in flight while the follow-up path times out anyway.

The contract should therefore be framed around keeping one valid same-version producer on the
save-critical path long enough to reduce terminal `shadow_state` outcomes, not around re-labelling
the fallback after the fact.

### 2. Do not solve this by blindly widening wait budgets

The current bundle already shows the relief valve engaging and still timing out.

Blindly increasing the budget would weaken the latency envelope without proving that the same
still-current producer will actually win more often. The implementation may use stronger producer
promotion, more truthful continuation proof, reduced retarget waste, or another bounded mechanism,
but the spec should not require "wait longer and hope."

### 3. Keep truthful fallback when the current target is no longer provable

`shadow_state` remains a valid fallback when:

- a newer same-file revision arrives;
- a newer save cycle overtakes the target;
- the runtime can no longer prove that the in-flight producer is still the bounded best candidate.

This change is not allowed to keep an older target alive merely to avoid reporting `shadow_state`.

### 4. Acceptance is representative-bundle-first

Synthetic regressions are required, but they are not sufficient.

Acceptance must refresh the representative `conf_big` incident bundle and compare:

- `ready_artifacts` vs `shadow_state` follow-up incidence;
- `wait_probe=timeout` incidence for same-version in-flight producers;
- the remaining terminal fallback reasons, if any.

## Alternatives Considered

### 1. Optimize diagnostics-only semantic work first

Rejected.

That work is tracked by `refactor-38`, but the current bundle shows the dominant save-path failure
earlier: the follow-up often never stays on the exact path long enough to benefit from those
optimizations.

### 2. Start the investigation in `vscode-extension/`

Rejected.

The current incident evidence is server-side and save-path-specific. It does not show client
pre-send latency as the dominant blocker.

### 3. Treat the current behavior as acceptable because one trace already reaches `ready_artifacts`

Rejected.

`1/4` success on the representative bundle is not a stable steady-state contract.

## Validation Strategy

- Add targeted backend/runtime regressions for:
  - a still-current same-version producer that is already in `parse_exec` when heavy follow-up
    reaches the bounded timeout edge;
  - reduced terminal `shadow_state` fallback on the representative `didSave` churn family;
  - truthful supersession or fallback when a newer same-file target overtakes the producer.
- Refresh the representative live incident bundle and compare it directly to the
  `2026-04-17T14:06:03Z` baseline.
- Run strict OpenSpec validation before handoff.

## Quality Gates

- The representative bundle no longer shows `shadow_state` as the dominant steady-state terminal
  path for this save-follow-up family.
- Remaining `shadow_state` traces, if any, carry truthful supersession or exhausted-continuation
  evidence.
- Latest-wins and same-version exactness remain intact.
- If refreshed evidence still shows the same `1 ready_artifacts / 3 shadow_state` pattern, the
  change is not ready.
