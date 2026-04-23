## Context

`refactor-50` correctly framed the remaining residual as waiting-only same-version `didSave`
fallback rather than rebuild-dominated `program_lowering`. The unresolved issue is narrower:

- the heavy follow-up path already has its own admission lane;
- detached diagnostics-ready publication already exists as a wake source;
- but the same-version `didSave` exact producer still competes through a mutable per-file worker
  model and generic CPU-class arbitration;
- therefore the follower can wait truthfully and still lose, because the producer has no separate
  bounded admission contract before `detached_ready_artifacts`.

`refactor-51` supersedes `refactor-50` for implementation. The `refactor-50` fail condition remains
mandatory acceptance evidence here: representative validation must fail if a still-current save
family terminates through waiting-phase `shadow_state` while the same run later proves same-family
detached or fully materialized exact readiness.

## Goals

- Give the same-version `didSave` exact producer a first-class bounded admission contract.
- Keep detached diagnostics-ready publication as the user-visible bounded success endpoint for the
  save family.
- Close the `refactor-50` waiting-phase `shadow_state` gate by making the producer, not the
  consumer fallback branch, the bounded contract owner.
- Preserve exactness, latest-wins supersession, and fail-closed semantics for interactive exact
  consumers.
- Keep observability truthful about producer queue wait, producer start, detached-ready publish,
  and fallback reasons.

## Non-Goals

- Do not reopen rebuild-dominated `program_lowering` optimization as the main diagnosis.
- Do not satisfy the inherited `refactor-50` gate with a consumer-only bypass that merely moves the
  terminal work from `shadow_state` to another generic semantic path while the producer still lacks
  bounded admission.
- Do not satisfy the change by widening wait budgets.
- Do not make `shadow_state` canonical exact truth for the saved revision.
- Do not start with a dedicated out-of-process executor or a separate runtime unless the narrower
  lane-and-budget fix still leaves a live residual.

## Decision

### 1. Introduce a first-class same-version `didSave` exact producer contract

The system should treat the exact producer for a same-version save family as a separate entity from
the heavy follow-up consumer. Its contract must start before `ParseExec` admission and end at
detached diagnostics-ready publication.

This prevents the bounded save-followup outcome from depending on whether a reused mutable worker
happened to start in time.

### 2. Keep admission lane and CPU arbitration tier orthogonal

The producer must not rely on generic `Interactive` versus `Background` routing alone. The system
should assign the producer a dedicated save-critical admission lane and CPU-budget tier, or a
semantically equivalent arbitration tier, that is distinct from:

- interactive exact request work such as completion/hover/definition;
- generic background diagnostics and precompute work.

This keeps save-followup exact producer latency bounded without collapsing it into the completion
fast path.

### 3. Key producer ownership to the exact save family

Producer ownership should be keyed to
`(file_id, requested_version, text_hash, save_cycle_sequence)`, or a semantically equivalent
identity, rather than inferred indirectly from one mutable per-file worker plus promotion flags.

The producer lifecycle must expose at least:

- admitted or started;
- detached diagnostics-ready published;
- fully materialized;
- superseded or cancelled;
- failed.

The heavy follow-up consumer should wait on those lifecycle events rather than on generic mutable
worker shape alone.

Consumer-side guards may remain as defense in depth, but they do not satisfy this change unless the
producer lifecycle proves that the same-family exact producer was admitted, published detached
diagnostics-ready, was truthfully superseded/cancelled/failed, or exhausted a bounded producer-owned
continuity proof.

### 4. Detached diagnostics-ready publication is the bounded success endpoint

The bounded success endpoint for the same-version save family should be detached
diagnostics-ready publication, not full exact ready-install or post-type-index completion.

Once detached-ready publication is available for the still-current save family, heavy follow-up may
publish through that exact-safe path while later full materialization continues independently.

## Alternatives Considered

### Keep refining worker promotion and respawn on the current per-file task

Rejected as the primary architecture. That approach keeps the latency contract implicit and tied to
one mutable worker slot, which is exactly the shape that still leaves `waiting`-phase timeouts on
the representative path.

### Increase bounded wait and relief-valve budgets

Rejected. It hides the producer admission problem instead of removing it.

### Optimize `shadow_state` semantic query first

Rejected as the primary fix. The user should not reach that branch as the steady-state terminal
outcome while the same save family remains current and can still publish detached exact readiness
later.

### Introduce a dedicated executor immediately

Deferred. If a dedicated lane and CPU-budget tier inside the existing runtime still leaves
representative queue-bound producer timeouts, that becomes the next escalation path.

## Risks

### Risk: save-critical producer capacity steals too much from completion

Mitigation:

- keep the save-critical tier small and explicit;
- limit its bounded success contract to detached diagnostics-ready publication;
- continue to keep completion and other exact interactive consumers on their own exact path.

### Risk: multiple producer instances waste CPU on duplicate same-family work

Mitigation:

- keep ownership keyed to exact save-family identity;
- make producer lifecycle explicit so superseded or duplicate producers can terminate truthfully.

### Risk: the change relabels waiting without changing terminal behavior

Mitigation:

- tighten representative acceptance so still-current waiting-only `shadow_state` terminal publish is
  no longer an allowed steady-state outcome;
- require evidence that the bounded winner is detached-ready or another exact-safe producer event.
- fail the representative gate when the only remaining explanation is
  `waiting -> shadow_state -> later same-family exact readiness`; do not accept this as merely a
  truthful attribution improvement.
