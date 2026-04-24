## Context

The current lineage is important:

- `refactor-43` scoped save-critical `parser_base_recovery` boundedness when the timeout leaf was
  parser-base recovery.
- `refactor-44` introduced detached diagnostics-ready artifacts so `didSave` follow-up did not
  wait for canonical live exact install.
- `refactor-50` framed a waiting-only `shadow_state` fallback gate.
- `refactor-51` gave same-version `didSave` exact producers first-class admission/lifecycle and
  passed p56 with `detached_ready_artifacts` as bounded winner.

The new bundle was captured after `refactor-51` on `git 070aa428`, so it is not evidence that
`refactor-51` was merely uninstalled. It is a narrower runtime contour:

- first publish is fast;
- gate/admission is not the bottleneck;
- the exact producer lifecycle reaches `started`;
- the bounded wait times out in `parse_exec` with leaf `parser_base_recovery`;
- the terminal diagnostics publish uses `shadow_state`;
- later exact materialization appears only as aggregate process metrics, not as a per-cycle
  same-family terminal lifecycle fact.

## Goals

- Make `started` a meaningful lifecycle boundary: once a same-version `didSave` producer starts,
  parser-base recovery must lead to detached diagnostics-ready publication, truthful supersession,
  truthful lost-continuity/failure, or a representative failure.
- Keep detached diagnostics-ready publication as the bounded success endpoint for diagnostics
  follow-up.
- Preserve `refactor-51` producer ownership instead of falling back to mutable per-file inference.
- Export enough per-cycle lifecycle evidence to tell "same producer later became ready" apart from
  "a different or superseded producer materialized later".
- Keep completion and transport out of scope unless new evidence shows they dominate.

## Non-Goals

- Do not widen the 3500ms bounded wait or 500ms relief valve as the fix.
- Do not make diagnostics-only detached artifacts visible as canonical exact readiness for
  interactive consumers.
- Do not make `shadow_state` an exact substitute for the saved revision.
- Do not satisfy the change with aggregate metrics only.

## Decision

### 1. Treat `started + parser_base_recovery` as a producer-owned bounded handoff

The producer contract should not stop at "started". For a still-current same-version `didSave`
producer, `started` must be followed by one of the bounded terminal lifecycle facts:

- `detached_diagnostics_ready_published`, or semantically equivalent detached-ready success;
- `fully_materialized`, if it happens first;
- `superseded` / `cancelled`, when a newer revision or save cycle overtakes the target;
- `failed` / `continuity_lost`, when the runtime can no longer prove the producer still owns the
  exact save family.

If the only observable path is `started -> parser_base_recovery timeout -> shadow_state`, the
representative gate must fail unless one of the truthful terminal reasons is also present.

### 2. Parser-base recovery should feed detached-ready, not a terminal shadow fallback

`parser_base_recovery` may still be expensive on large modules, but for this diagnostics-save path
the useful endpoint is not full live exact install. It is the detached diagnostics-ready payload
that `refactor-44` and `refactor-51` already made safe for diagnostics follow-up.

The implementation should therefore focus on the handoff from parser-base recovery completion or
bounded proof to detached-ready publication/wakeup, rather than on making the fallback
`shadow_state` semantic query cheaper.

### 3. Per-cycle lifecycle evidence is mandatory

The new bundle shows aggregate `ready_snapshot_materialization source=did_save count=2`, but the
request timeline only records lifecycle `started` at timeout. That is not enough to prove whether
the same save family later produced detached/full exact readiness or was truthfully superseded.

The incident bundle and representative report must preserve at least:

- save-family identity fields;
- lifecycle at bounded timeout;
- final lifecycle after timeout/fallback;
- parser-base recovery timeout leaf and elapsed values;
- detached-ready publication/wakeup evidence when it happens later;
- truthful fallback reason when detached-ready is not allowed.

### 4. Acceptance is a fail gate, not a budget relaxation

The representative gate should fail on the new contour:

```text
save_fastlane published
producer lifecycle started
bounded wait timeout leaf parser_base_recovery
semantic path shadow_state
semantic query dominated fallback
no per-cycle truthful terminal producer reason
```

The change is complete only when that contour becomes either a detached-ready success or a
truthfully explained non-exact terminal outcome.

## Alternatives Considered

### Increase wait budgets

Rejected. It masks the parser-base handoff failure and risks reintroducing long save latency.

### Optimize the shadow-state semantic query first

Rejected as the primary fix. The latest bundle shows the system reached an exact producer and then
abandoned it for `shadow_state`; cheaper fallback would still leave the wrong terminal branch.

### Treat aggregate materialization as success evidence

Rejected. Aggregate metrics prove some `didSave` materialization happened in the process, but do
not prove it belongs to the timed-out save cycle.

### Reopen VS Code UI or transport investigation

Rejected for this change. The same bundle shows client pre-write and transport/dispatch waits in
the 0-2ms range for completion, while diagnostics-save residuals are backend parser-base/fallback
facts.

## Risks

### Risk: parser-base recovery remains legitimately slow

Mitigation: the contract allows truthful failure or lost-continuity outcomes. It only rejects
silent `started -> timeout -> shadow_state` as if it were a normal steady-state path.

### Risk: detached diagnostics-ready leaks into interactive exact consumers

Mitigation: keep the detached artifact boundary diagnostics-only and preserve the existing exact
readiness gates for completion, hover, definition, signatureHelp, and type-at-position.

### Risk: observability becomes noisy

Mitigation: add only bounded low-cardinality lifecycle fields keyed to the existing save-family
identity, not free-form logs.
