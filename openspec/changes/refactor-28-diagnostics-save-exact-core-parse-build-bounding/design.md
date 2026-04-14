## Context

The exact same-version ready-snapshot path has already been narrowed in three steps:

- `refactor-25` removed `stale_parser_base` as the dominant root cause and bounded obsolete parse
  waste;
- `refactor-26` removed post-ready publish/apply gating as the primary blocker on the exact path;
- `refactor-27` separated `optional_cache_enrichment` from `core_parse_build` and proved in live
  `conf_big` evidence that the remaining timeout is now dominated by `core_parse_build`.

The checked-in `refactor-27` live evidence shows:

- `followup_ready_snapshot_timeout_phase=parse_exec`
- `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
- `followup_ready_snapshot_parse_exec_dominant_subphase=core_parse_build`
- `followup_ready_snapshot_parse_exec_core_parse_build_ms≈4082`
- `followup_semantic_path=shadow_state`

That means the remaining problem is not the whole parse phase anymore. It is the exact same-version
core build itself.

## Goals

- Reduce the amount of exact-path work that still happens inside `core_parse_build` before the
  first publishable exact ready snapshot can materialize.
- Replace the remaining monolithic `core_parse_build` residual with bounded internal attribution.
- Preserve exactness, supersession, and truthful fallback semantics.

## Non-Goals

- Reopening parser-base recovery from `refactor-25`.
- Reopening apply/publish gating from `refactor-26`.
- Reopening optional-cache-enrichment deferral from `refactor-27` as the primary focus.
- Broadening `didSave` wait budgets as a substitute for runtime improvement.

## Proposed Approach

1. Introduce a save-critical exact mode inside `core_parse_build`.
   When the exact same-version producer is on the save-critical path, the runtime should separate
   work that is strictly required to materialize publishable exact ready artifacts from secondary
   core-build work that can happen after first publish or be cancelled on supersession.

2. Split `core_parse_build` into bounded internal checkpoints.
   The final subphase names can follow the implementation, but the contract should expose enough
   detail to distinguish at least:
   - parser/tree work that is still required for the exact ready snapshot;
   - exact-ready artifact assembly / conversion work after the parser tree exists;
   - bounded yield or re-check points where save-critical promotion or retarget can take effect.

3. Keep fail-closed behavior.
   If the bounded core-build path still cannot prove current exact artifacts, the system must keep
   the truthful fallback to `shadow_state`. Success means either `conf_big` returns to
   `ready_artifacts`, or the residual moves from generic `core_parse_build` to a narrower,
   operator-meaningful checkpoint.

## Alternatives Considered

### 1. Widen the `didSave` wait and relief-valve budgets again

Rejected. `refactor-27` already proved the remaining dominant cost sits inside exact
`core_parse_build`. More budget would only stretch latency while leaving the same root cause in
place.

### 2. Accept `core_parse_build` as an irreducible hot path and stop at observability

Rejected. The current live evidence still falls back to `shadow_state` on the representative
`conf_big` path. That is acceptable as a truthful residual for `refactor-27`, but not as the final
state for this line of work.

### 3. Target the exact core-build residual directly

Selected. The runtime now has enough attribution to justify a focused change on the remaining hot
slice without reopening already-resolved parser-base and apply-lag issues.

## Validation Strategy

- Targeted backend regressions should prove that exact `didSave` follow-up can materialize ready
  artifacts without paying the full old `core_parse_build` cost on the critical path.
- Targeted backend regressions should prove that a remaining miss now reports a bounded
  core-build checkpoint, not the old top-level `core_parse_build` bucket.
- Representative `conf_big` live evidence should show either a return to `ready_artifacts` or a
  narrower truthful core-build residual.
