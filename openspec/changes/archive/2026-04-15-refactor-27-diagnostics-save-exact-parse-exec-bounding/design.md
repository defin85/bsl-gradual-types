## Context

The exact same-version ready-snapshot path has already been tightened in three steps:

- `refactor-23` separated `parse_exec` from post-parse/materialization and document-symbol work;
- `refactor-25` removed `stale_parser_base` as the primary cause and added during-parse retarget
  observation;
- `refactor-26` removed post-ready `apply_lag` as the primary late blocker for representative
  exact workers.

Representative `conf_big` evidence still shows the exact `didSave` follow-up timing out while the
same-version producer remains in `parse_exec`. That means the next change must target the exact
critical path itself, not another external gate.

## Goals

- Reduce the time that the exact same-version producer spends on work that is not required to
  materialize current ready artifacts for `didSave` follow-up.
- Make exact `parse_exec` timeouts attributable to a bounded internal subphase rather than a single
  opaque phase label.
- Preserve exactness, supersession, and truthful fallback semantics.

## Non-Goals

- Reopening parser-base recovery from `refactor-25`.
- Reopening apply/publish gating from `refactor-26` as the primary focus.
- Broadening the `didSave` wait budget as a substitute for runtime improvement.

## Proposed Approach

1. Introduce a save-critical exact mode for the current same-version producer.
   When `didSave` follow-up is waiting on an exact still-current producer, the runtime should be
   able to mark that producer as save-critical. In this mode, any work inside `parse_exec` that is
   not required to materialize exact ready artifacts should be deferred, skipped, or made
   cancellable until after the ready snapshot is installed.

2. Split `parse_exec` into bounded observable checkpoints.
   The current truth stops at `timeout_phase=parse_exec`. The new change should allow the runtime to
   say which exact internal slice dominated the miss. The exact subphase names can follow the final
   implementation, but the contract should distinguish at least:
   - core parse/build work needed for exact ready artifacts;
   - optional enrichment or normalization that can be deferred off the save-critical path;
   - bounded in-parse waiting/yield points where retarget or save-critical promotion can take
     effect.

3. Keep fail-closed behavior.
   If the save-critical path still cannot prove current exact artifacts, the system must preserve
   the existing truthful fallback to `shadow_state`. The change is successful only if it reduces
   the amount of exact-path work on the critical path or makes the remaining residual more precise.

## Tradeoffs

- More checkpoints and modes increase internal state complexity, but the current bottleneck is too
  expensive to leave as a monolithic `parse_exec` blob.
- Deferring optional work may slightly shift when secondary caches or enrichments are populated,
  but that is acceptable as long as publishable exact ready artifacts remain semantically correct.
- Exporting a more detailed subphase contract adds versioning work to observability consumers, but
  that is preferable to another round of ambiguous latency evidence.

## Validation Strategy

- Targeted backend regressions should prove that save-critical exact follow-up can materialize
  ready artifacts without waiting for deferred optional parse work.
- Targeted backend regressions should prove that a remaining miss now reports the exact in-parse
  subphase, not only generic `parse_exec`.
- Representative `conf_big` live evidence should show either a return to `ready_artifacts` or a
  new truthful subphase-level residual.
