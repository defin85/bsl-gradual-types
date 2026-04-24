## Context

The current `didSave` follow-up lineage has several distinct failure classes:

- `refactor-50` documented waiting-only `shadow_state` fallback.
- `refactor-51` moved same-version `didSave` exact producers into a dedicated lane/lifecycle
  contract.
- `refactor-52` addressed started-producer `parser_base_recovery` fallback and added final
  same-family lifecycle evidence.
- `refactor-53` addressed `program_lowering full_rebuild -> shadow_state -> later detached-ready`
  as a correctness failure.

The bundle captured on `2026-04-24T14:22:42.992Z` is narrower and different:

- completion ingress, admission, same-file token wait, and response handoff remain bounded;
- both diagnostics-save follow-ups publish through `detached_ready_artifacts`, not `shadow_state`;
- one save cycle has a multi-second `save_fastlane` first publish dominated by syntax query while
  the exact producer is in `parser_base_recovery`;
- the next save cycle has a fast first publish but the heavy follow-up arrives only after bounded
  wait and relief-valve timeouts because exact `program_lowering` performs a full rebuild.

This means `detached_ready_artifacts` is now the right endpoint, but it can still arrive too late to
meet the save-follow-up latency contract.

## Goals

- Make "correct terminal path but too late" a first-class diagnostics-save residual.
- Keep `save_fastlane` first publish bounded and independently auditable.
- Bound heavy follow-up exact materialization when the producer remains still-current and eventually
  reaches detached diagnostics-ready.
- Preserve `program_lowering` reuse/rebuild evidence so full-rebuild latency cannot be hidden under
  a successful detached-ready terminal path.
- Keep UI, transport, and interactive exact-consumer semantics out of scope for this bundle.

## Non-Goals

- Do not increase wait budgets to absorb multi-second materialization.
- Do not treat detached diagnostics-ready artifacts as canonical live exact readiness.
- Do not merge this residual back into `shadow_state` fallback correctness.
- Do not accept aggregate metrics without per-cycle trace evidence.

## Decision

### 1. Split terminal correctness from latency boundedness

`followup_semantic_path=detached_ready_artifacts` is the correct terminal path for diagnostics-only
same-version follow-up, but it is not sufficient when the same trace also shows:

```text
bounded_wait_winner=timeout
relief_valve_outcome=engaged_timed_out
timeout_phase=parse_exec
timeout_leaf=program_lowering
program_lowering_reuse_outcome=full_rebuild
followup_publish_elapsed_ms=4884
final_lifecycle=detached_diagnostics_ready_published
```

That path proves the correctness fix avoided terminal `shadow_state`, but it also proves the
still-current exact producer missed the latency envelope before detached-ready publication.

### 2. Treat slow first publish as an independent save-fastlane failure

The first trace shows the opposite shape: heavy follow-up is fine, but first publish is not. A
`save_fastlane` syntax-only publish that takes `3397ms` and is dominated by
`syntax_diagnostics_query_ms=3397` must not be masked by a later `577ms` detached-ready follow-up.

The implementation should first identify whether the slow first publish comes from broad syntax
recomputation, parser-base coupling, snapshot contention, or another exact-producer dependency.
The contract should require either bounded first publish or a truthful first-publish blocker; it
should not prescribe a cache/reuse mechanism before that root cause is verified in code.

### 3. Fix exact materialization or classify it truthfully

For the heavy follow-up full-rebuild residual, implementation should prefer root fixes in this
order:

1. preserve or recover safe reuse context so same-version `program_lowering` does not full-rebuild
   unchanged lowering units;
2. make exact materialization observable and interruptible enough that still-current save-critical
   promotion or supersession can act before one opaque multi-second lowering span completes;
3. if bounded exact materialization cannot be proven, export a truthful terminal reason independent
   of bounded-wait expiry and full-rebuild reuse miss.

The change is not satisfied by publishing through detached-ready eventually while the trace still
shows a bounded wait timeout, relief-valve timeout, and multi-second full rebuild for a
still-current save family.

### 4. Extend representative validation

The representative gate should fail the new contour even though it no longer contains
`shadow_state`:

```text
save_fastlane first publish too slow due syntax query
or
detached_ready_artifacts terminal path after bounded wait timeout
  and relief timeout
  and program_lowering full_rebuild
  and no truthful supersession/cancellation/failure/continuity-loss reason
```

The gate should keep `refactor-53`'s shadow fallback check, but this change adds latency checks for
the corrected detached-ready path.

### 5. Preserve observability fidelity

The evidence path must keep the following fields available in checked-in representative reports and
incident-bundle projections:

- `requested_version`, `save_cycle_sequence`, and save-family identity;
- first publish profile, publish kind, elapsed, syntax query elapsed, and syntax work mode;
- zero-budget, bounded-wait, and relief-valve outcomes;
- timeout phase, timeout leaf, elapsed values, subphase, and checkpoint;
- `parser_base_recovery` and `program_lowering` phase timings;
- `program_lowering_reuse_outcome`, rebuilt/reused units, and reuse-plan hit flags;
- terminal semantic path and follow-up publish elapsed;
- lifecycle at timeout and final same-family lifecycle.

## Alternatives Considered

### Reopen `shadow_state` fallback

Rejected. The fresh bundle already publishes both follow-ups through `detached_ready_artifacts`.
The remaining failure is latency and full-rebuild materialization cost.

### Increase bounded wait or relief budgets

Rejected. That would hide the producer cost and make the live save-follow-up path less responsive.

### Start in VS Code extension dispatch

Rejected for this change. The fresh completion traces show `client_before_transport_write_wait_ms`
at `1-2ms` and no meaningful transport/admission/egress wait.

### Treat final detached-ready lifecycle as success

Rejected. Final lifecycle proves the endpoint is correct, but user-visible save feedback can still
arrive after `3397ms` first-publish latency or `4884ms` heavy follow-up latency.

## Risks

### Risk: save-fastlane syntax recomputation has a legitimate cold path

Mitigation: allow truthful first-publish blocker evidence, but keep the representative gate from
silently accepting multi-second first publish as success.

### Risk: some edits must full-rebuild lowering units for correctness

Mitigation: allow truthful full rebuild when invalidation requires it, but require per-cycle
evidence and keep the heavy follow-up latency gate honest for representative same-file saves.

### Risk: detached diagnostics-ready artifacts leak into interactive exact consumers

Mitigation: preserve the existing diagnostics-only detached artifact boundary and canonical live
exact gates.
