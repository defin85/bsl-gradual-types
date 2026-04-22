## Context

`refactor-44` established the detached diagnostics-ready artifact as a valid diagnostics-only read
model and proved that live exact consumers stayed fail-closed. The new post-`refactor-44` bundle
on `e7ffc155` shows the next residual more precisely:

- `diagnostics-save-trace-2` reaches `followup_semantic_path=detached_ready_artifacts`, but only
  after `followup_ready_snapshot_wait_probe=timeout` and
  `followup_ready_snapshot_timeout_leaf=ready_install` at `3423ms`;
- `diagnostics-save-trace-1` stays on `generic_pipeline` after
  `followup_ready_snapshot_wait_probe=version_mismatch`, which is the correct fail-closed outcome;
- completion traces in the same bundle are not the root cause for this change: the hot successful
  completion stays at `178ms`, while the only severe outlier is a separate
  `adapter_to_dispatch_wait_ms=14892` ingress backlog.

So the detached artifact itself is no longer the missing piece. The remaining problem is that
`didSave` bounded wait still waits on canonical ready-snapshot materialization only, and checks
detached artifacts only after the canonical wait returns `NotReady` or `Timeout`.

That means the runtime can already have the safe diagnostics-only answer for the current save
target and still burn most of the bounded window waiting for a stricter live exact install that
interactive consumers need but diagnostics follow-up does not.

## Goals / Non-Goals

- Goals:
  - wake same-version `didSave` heavy follow-up on the first matching safe artifact for the
    current save target;
  - preserve canonical `ready_artifacts` preference whenever live exact readiness materializes
    first;
  - keep detached artifacts diagnostics-only and fail-closed for interactive exact consumers;
  - expose the winning wake source truthfully in telemetry and incident bundles.
- Non-Goals:
  - changing interactive exact contracts;
  - reclassifying detached success as relief-valve exact success;
  - solving transport ingress backlog or generic completion latency;
  - broad detached current-revision head architecture beyond diagnostics follow-up.

## Decisions

### 1. Introduce a first-class dual-artifact wait contract

The save-follow-up waiter should no longer be modeled as:

- wait for canonical ready artifacts;
- if that fails, look for detached artifacts.

It should be modeled as:

- prefer already-materialized canonical ready artifacts immediately;
- otherwise, bounded-wait on the first matching wake source for the still-current target:
  canonical `ready_artifacts` or detached diagnostics-ready artifacts;
- fall through only on truthful timeout / supersession / cancellation / generation mismatch /
  version mismatch.

This keeps one bounded wait envelope but removes the artificial coupling where detached artifacts
exist yet remain invisible until the canonical wait is already exhausted.

### 2. Make detached publication observable through a cancellation-safe wake surface

The bounded wait loop is currently a `select!`-style polling path around canonical task
notifications. The new detached wake source must therefore be cancellation-safe under repeated
select/poll restarts.

The implementation should use a first-class publication signal with monotonic target identity
state, such as a `watch`-style channel or a semantically equivalent restart-safe publication
surface, instead of bolting another best-effort `Notify` onto the loop.

The waiter must re-check exact target identity after wake and before detached consumption.

### 3. Canonical ready artifacts remain the stronger winner

This change does not demote canonical live exact readiness.

Rules:

- if canonical ready artifacts are already materialized, they win immediately;
- if canonical ready artifacts materialize before detached artifacts during the bounded wait, the
  follow-up publishes through `ready_artifacts`;
- detached artifacts may win only while canonical live exact readiness is still pending for the
  same target;
- relief-valve semantics stay exact-only and remain separately attributable.

### 4. Truthful fail-closed outcomes remain stronger than detached convenience

Detached publication must not override:

- newer same-file revision;
- newer save cycle for the same revision;
- diagnostics-generation mismatch;
- explicit cancellation;
- any semantically equivalent proof that the waiting target is no longer current.

If a detached publication is observed for a stale target, the waiter must ignore it and keep the
truthful terminal outcome.

### 5. Observability must expose the wake winner explicitly

The new path needs dedicated operator-facing attribution:

- winner: `ready_artifacts`, `detached_ready_artifacts`, or a truthful miss outcome;
- bounded wait elapsed until the winning artifact became usable;
- detached publication latency, if detached won;
- preserved existing ready-snapshot probe attribution so `ready_install` bottlenecks remain visible
  instead of disappearing behind the new winner label.

Without this, the next bundle would still look like "detached happened after timeout" even if the
actual runtime behavior improved materially.

## Alternatives Considered

### 1. Lower the polling sleep or keep checking detached artifacts more often

Rejected.

This is a heuristic latency tweak, not a contract fix. It keeps the waiter modeled as a
canonical-only wait with opportunistic detached peeking.

### 2. Widen bounded wait / relief-valve budgets

Rejected.

That would merely rename the same `ready_install` residual under a longer clock.

### 3. Route detached wake through the relief valve

Rejected.

Relief valve is an exact-path extension mechanism. Detached wake is a diagnostics-only success path
and must stay separately attributable.

### 4. Use another raw `Notify` in the loop

Rejected by default.

The bounded wait loop is restartable and future cancellation must not lose detached wakeups or
fairness position. A restart-safe publication primitive is the safer baseline for this path.

## Validation Strategy

- Add a backend regression where detached diagnostics-ready publication happens during the bounded
  canonical wait and prove the follow-up completes before timeout-sized canonical wait exhaustion.
- Add a paired regression where canonical ready artifacts materialize first and still win over
  detached artifacts.
- Add a stale-target regression where detached publication for an older `save_cycle_sequence` or
  superseded target does not wake a newer waiter.
- Preserve existing interactive exact regressions proving fail-closed behavior.
- Refresh representative live evidence for the `p55` / `p56` family and require at least one
  authoritative save-followup sample where detached publish wins without a timeout-sized canonical
  wait being the primary gate.
