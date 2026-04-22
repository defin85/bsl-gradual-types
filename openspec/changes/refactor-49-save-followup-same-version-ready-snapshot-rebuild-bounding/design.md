## Context

`refactor-44` and `refactor-46` already moved `didSave` heavy follow-up away from the old
"wait only for canonical ready install, then fall back blindly" shape. The new bundle on
`b050f812` confirms that this is not enough by itself.

The residual representative failure is now narrower:

- the save target is still current;
- `save_fastlane` already published quickly;
- one follow-up trace still sees an exact same-version task in flight;
- the bounded wait times out inside `parse_exec`, specifically inside
  `exact_ready_snapshot_assembly -> program_lowering`;
- heavy follow-up then degrades to `shadow_state`.

So the remaining problem is not "wake source missing" and not "didChange handoff too late". It is
that exact same-version ready-snapshot rebuild itself can still behave like a cold rebuild on the
saved revision, even though the server already has current same-file state for that revision.

## Goals

- Reduce same-version `didSave` ready-snapshot rebuild latency on still-current saved revisions.
- Avoid `shadow_state` fallback when the only blocker is seconds-scale exact rebuild work on the
  same target.
- Preserve canonical live exact truth, latest-wins supersession, and fail-closed behavior when a
  safe fast path cannot be proven.
- Keep request-centric observability able to distinguish rebuild-stage latency from queue/apply
  lag.

## Non-Goals

- Do not widen the bounded wait or relief-valve windows as the main fix.
- Do not publish partial or detached state as if it were canonical interactive exact readiness.
- Do not redesign the broader detached current-revision head architecture in this change.
- Do not erase truthful queue/apply attribution when those blockers are real.

## Decision

### 1. Treat same-version saved-revision rebuild as a first-class fast-path candidate

When `didSave` heavy follow-up is still targeting the exact same `(file_id, requested_version,
text_hash, save_cycle_sequence)` and the server already has matching current-revision state for
that target, the exact ready-snapshot producer should prefer a reuse-aware rebuild path before
defaulting to a cold same-version rebuild.

The concrete reuse input may be current shadow-backed parse state, same-version reusable lowering
inputs, or another semantically equivalent exact-safe seed. The design requirement is not the
specific data structure; it is that the same-version rebuild path no longer behaves like the cold
default when safe exact reuse inputs already exist.

### 2. Keep canonical truth and fallback semantics explicit

The fast path must stay keyed to the exact save target identity and preserve:

- latest-wins supersession;
- text/version/save-cycle matching;
- truthful timeout, mismatch, or cancellation behavior when the fast path cannot be proven safe.

If safe same-version reuse is absent, stale, or superseded, the system should keep the existing
canonical fallback instead of manufacturing exact readiness from partial work or from `shadow_state`
alone.

### 3. Acceptance should fail on rebuild-dominated shadow-state fallback

Representative live evidence must now distinguish this residual from the already-fixed ones.
The accepted gate should fail when:

- `save_fastlane` is still fast and same-version;
- no newer revision or explicit queue/apply blocker explains the miss;
- heavy follow-up still terminates through `shadow_state`;
- and the dominant blocker is rebuild-stage `parse_exec`, especially
  `exact_ready_snapshot_assembly -> program_lowering`.

This keeps the change tied to the observed regression family instead of drifting into a generic
"make diagnostics faster" umbrella.

## Alternatives Considered

### Widen bounded wait / relief-valve budgets

Rejected. The bundle already shows one path timing out after a `3500ms` bounded wait plus a
`500ms` relief valve while rebuild work remains slow. Larger budgets would hide the regression
instead of removing it.

### Accept `shadow_state` and improve only observability

Rejected. Observability is already sufficient to name the residual. The problem is operational:
same-version saved revisions still miss richer follow-up because exact rebuild is too slow.

### Reopen broad detached current-revision snapshot architecture

Rejected for this change. That architecture may still be useful later, but the current bundle
already narrows the failure to same-version `didSave` rebuild latency. The next change should stay
on that exact seam.

## Risks

### Risk: reuse path returns stale or mismatched exact state

Mitigation:

- keep the fast path keyed to `(file_id, requested_version, text_hash, save_cycle_sequence)` or a
  semantically equivalent identity;
- fall back truthfully when reuse proof is absent;
- preserve final canonical exact install semantics.

### Risk: rebuild gets faster only by moving work into an unobservable bucket

Mitigation:

- keep phase-level diagnostics-save attribution;
- keep representative gates tied to terminal path and dominant phase/checkpoint;
- require evidence to separate rebuild-stage latency from queue/apply blockers.
