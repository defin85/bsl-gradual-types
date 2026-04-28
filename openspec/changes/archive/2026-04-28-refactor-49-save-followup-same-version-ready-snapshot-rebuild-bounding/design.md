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

After the latest truthful-attribution pass, the contour is narrower still:

- moving `ParseExec` attribution to the blocking worker `on_exec_started` hook removed the fake
  `parse_exec -> before_first_parse_exec_subphase` bucket;
- the still-current timeout path can now be reported truthfully as `waiting`, which is a separate
  queue/admission issue and not evidence of `program_lowering`;
- but representative `conf_big` evidence still shows a real same-version exact rebuild outlier on a
  later cycle where the path reaches `detached_ready_artifacts` only after
  `parse_exec -> exact_ready_snapshot_assembly -> program_lowering` remains seconds-scale.

After the latest parser/runtime pass, the picture changed again:

- a narrow synthetic reproducer now proves the same-version reprime seam: preserving an existing
  owned AST cache entry lets the same-content exact path materialize through
  `reuse_plan_take_if_unique=true` instead of re-entering the borrowed-clone path;
- the next representative `conf_big` rerun no longer reached the older cycle-2 cold
  `program_lowering` outlier first;
- instead, that intermediate `p56` run blocked earlier on cycle 1, where the observed trace could
  stay at `followup_wait_reason=pending_publish` without any bounded follow-up semantic-path
  decision.

After the latest diagnostics-save timeline pass, the picture narrowed again:

- `wait_for_save_fastlane_first_publish_v2` no longer relies only on `active_cycles` state to
  prove a successful fastlane first publish;
- the server now rechecks matching archived traces by
  `(uri, requested_version, diagnostics_generation, save_cycle_sequence)`, and a focused unit
  regression proves the proof survives active-cycle archival;
- the latest representative `p56` rerun no longer fails first on cycle-1 `pending_publish`;
- but cycle 2 still lands on the older outlier shape:
  `followup_ready_snapshot_parse_exec_ms~=38.5s`,
  `exact_ready_snapshot_assembly/program_lowering_ms~=38.5s`,
  `followup_publish.elapsed_ms~=81.3s`, and
  `semantic_diagnostics_query_ms~=1.8s`, so semantic query work still does not dominate the ready
  snapshot parse-exec stage.

After the final same-version rebuild pass, the representative live contour is now acceptance-ready:

- stale completed previous-version type-index tasks are discarded before `didChange` and
  `didSave` follow-up observation, so waiting-phase representative probes no longer inherit fake
  previous-version `type_index_task_state_after_timeout` noise;
- when a same-version `DidSaveFollowup` rebuild has non-empty `parser_edits`, the exact worker may
  prime the parser AST cache from the previous ready snapshot if that snapshot is exact-safe,
  older than the requested version, syntax-complete, parse-error-free, and text-distinct from the
  requested content;
- the `2026-04-23` representative `p56` rerun on `examples/conf_big` now keeps the detached cycle
  out of the old cold-rebuild bucket:
  `followup_semantic_path=detached_ready_artifacts`,
  `followup_ready_snapshot_parse_exec_ms=150`,
  `followup_ready_snapshot_program_lowering_ms=137`,
  `followup_publish_semantic_diagnostics_query_ms=1118`,
  `program_lowering_reuse_outcome=routine_body_reuse`,
  `program_lowering_reuse_plan_build_source=owned`,
  `program_lowering_reuse_plan_take_if_unique_hit=true`,
  `program_lowering_reused_lowering_units=2079`, and
  `program_lowering_rebuilt_lowering_units=9`;
- the same representative report still includes waiting-phase `shadow_state` cycles, but those are
  now separately attributed to `waiting`, not `parse_exec/program_lowering`, and the acceptance
  test only allows them when the saved exact ready snapshot later materializes on the same save
  target.

So the accepted closure is now explicit:

1. the parser/runtime seam is fixed locally by preserving previous-ready AST ownership for
   same-version `didSave` rebuilds with real edits;
2. stale previous-version type-index leftovers no longer pollute the waiting-phase contour;
3. detached representative cycles prove query-dominated follow-up rather than rebuild-dominated
   `program_lowering`;
4. remaining `shadow_state` cycles are truthful waiting-phase queue blockers, which stay visible
   without re-opening this change as a rebuild regression.

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

The same gate must remain honest about the new split:

- separately attributed `waiting` is not sufficient evidence to close the change;
- but it also must not be misreported as rebuild-stage `parse_exec/program_lowering`;
- acceptance closes only when the representative outlier no longer shows cold exact rebuild on the
  still-current same-version path, regardless of whether the terminal semantic path is
  `shadow_state` or a very-late `detached_ready_artifacts`.
- if the representative contour stalls earlier at `pending_publish`, that earlier live blocker must
  be explained and removed rather than ignored;
- once that earlier blocker is removed, the gate still stays open until the later cycle-2
  `program_lowering` outlier is gone as well;
- if the representative contour alternates between those two cycle-2 failures across reruns, the
  change also stays open until that instability itself is explained.

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
