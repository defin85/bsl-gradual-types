## Context

The current `didSave` follow-up lineage has several distinct failure classes:

- `refactor-50` documented a waiting-only `shadow_state` fallback after fast save publication.
- `refactor-51` gave same-version `didSave` exact producers dedicated admission/lifecycle ownership.
- `refactor-52` closed the started-producer/parser-base contour and added final same-family lifecycle
  evidence after timeout or fallback.

The new bundle captured on `2026-04-24T10:50:21.149Z` is narrower:

- completion ingress/egress remains bounded;
- `save_fastlane` first publish is fast;
- the bounded wait times out at `parse_exec`, not `waiting`;
- the timeout leaf is `program_lowering`, not `parser_base_recovery`;
- program-lowering reuse misses completely (`full_rebuild`, `2088` rebuilt units, `0` reused units);
- the terminal branch is `shadow_state`;
- final same-family lifecycle later reaches `detached_diagnostics_ready_published`.

The current backend guard in `diagnostics_runtime.rs` blocks `shadow_state` fallback for the
same-version `waiting` timeout contour, but the new failing sample is a rebuild-dominated
`parse_exec/program_lowering` contour. That means the old guard is too narrow for this new residual,
but simply blocking every parse-exec timeout would be too broad unless the runtime can preserve
truthful supersession, cancellation, failure, or continuity-loss evidence.

## Goals

- Make `program_lowering full_rebuild` a first-class residual in the same-version `didSave`
  follow-up contract.
- Keep detached diagnostics-ready publication as the bounded success endpoint for diagnostics
  follow-up.
- Ensure still-current same-family producers either avoid full rebuild, reach detached-ready within
  the existing envelope, or report a truthful non-exact terminal reason.
- Preserve per-cycle evidence that proves whether shadow fallback raced a later detached-ready
  producer for the same save family.
- Keep completion, transport, and VS Code UI out of scope unless a newer bundle contradicts the
  current measurements.

## Non-Goals

- Do not increase the bounded wait or relief-valve budgets.
- Do not make `shadow_state` an exact substitute for the saved revision.
- Do not make detached diagnostics-ready artifacts visible as canonical exact readiness for
  interactive consumers.
- Do not satisfy the change with aggregate metrics only.

## Decision

### 1. Treat program-lowering full rebuild as a producer-owned boundedness failure

For a still-current same-version `didSave` save family, the diagnostics follow-up path should not
treat this sequence as acceptable:

```text
save_fastlane published
bounded wait times out in parse_exec/program_lowering
program_lowering_reuse_outcome=full_rebuild
followup_semantic_path=shadow_state
final same-family lifecycle=detached_diagnostics_ready_published
```

That is not a truthful non-exact terminal outcome. It is a race between a bounded consumer fallback
and a same-family producer that eventually reaches the intended detached-ready endpoint.

### 2. Fix the reuse/proof boundary before changing fallback semantics broadly

The first implementation target should be the exact producer input/reuse boundary:

- preserve or derive ranged parser-edit context for direct same-version `didSave` producers;
- ensure the program-lowering reuse plan can borrow or rebase from the previous safe snapshot when
  the save family remains same-file and same-current;
- keep the `detached_ready_artifacts` path as the consumer endpoint once the producer publishes.

Only if the runtime cannot prove continuity should it emit a truthful non-exact terminal reason.
Suppressing `shadow_state` without repairing reuse or emitting truth would hide the failure.
Bounded-wait expiry and `program_lowering_reuse_outcome=full_rebuild` are not, by themselves,
truthful terminal reasons. If final same-family lifecycle later proves
`detached_diagnostics_ready_published` or `fully_materialized`, a prior `shadow_state` publication
is still the failing contour unless the runtime also proves independent supersession, cancellation,
failure, or continuity loss.

### 3. Extend the representative fail gate

The existing representative gate already fails on waiting/parser-base shadow fallback. This change
adds a distinct fail gate for rebuild-dominated parse-exec fallback:

```text
timeout_phase=parse_exec
timeout_leaf=program_lowering
program_lowering_reuse_outcome=full_rebuild
followup_semantic_path=shadow_state
final lifecycle later proves detached diagnostics-ready for the same save family
```

The gate must not require full live exact install. The pass condition remains detached
diagnostics-ready or a truthful terminal producer reason.

### 4. Preserve observability fidelity

The new bundle already exposes enough raw fields to identify the residual. The implementation should
keep these fields available in representative and incident-bundle projections:

- `requested_version` and `save_cycle_sequence`;
- zero-budget, bounded-wait, and relief-valve outcomes;
- timeout phase/leaf and elapsed values;
- `program_lowering_reuse_outcome`;
- rebuilt/reused lowering units and reuse-plan hit flags;
- terminal semantic path and semantic query elapsed;
- lifecycle at timeout and final same-family lifecycle.

If any path can still fall back through `shadow_state`, it must export a truthful reason that makes
the non-exact terminal outcome auditable.

## Alternatives Considered

### Increase wait budgets

Rejected. It would hide full rebuild latency and risks reintroducing long save-follow-up stalls.

### Optimize shadow-state semantic query first

Rejected as the primary fix. The failing sample shows `publish_wait_ms=1`; the expensive query is
downstream of the wrong terminal branch, not the root cause.

### Treat final lifecycle detached-ready as enough success

Rejected. Final lifecycle proves the producer eventually did the right work, but the user-visible
follow-up already published through `shadow_state` after an 8440ms path.

### Reopen transport or VS Code UI

Rejected for this change. Completion transport fields are bounded in the same bundle while the
dominant residual is backend ready-snapshot/program-lowering evidence.

## Risks

### Risk: program-lowering reuse cannot be proven for some edits

Mitigation: the contract allows truthful supersession, cancellation, failure, or continuity-loss
outcomes. It only rejects silent `full_rebuild -> shadow_state` when the same-family producer later
publishes detached-ready.

### Risk: overbroad shadow fallback suppression

Mitigation: key the fail gate to same-family identity and explicit program-lowering full rebuild
evidence, not to every `parse_exec` timeout.

### Risk: diagnostics-only artifacts leak into interactive exact consumers

Mitigation: preserve the existing detached diagnostics-ready boundary and interactive exact readiness
gates.
