## Context

The exact same-version ready-snapshot path has already been narrowed in four steps:

- `refactor-25` removed `stale_parser_base` as the dominant root cause and bounded obsolete parse
  waste;
- `refactor-26` removed post-ready publish/apply gating as the primary blocker on the exact path;
- `refactor-27` separated `optional_cache_enrichment` from `parse_exec`;
- `refactor-28` separated `tree_cache_install` from the save-critical exact path and proved in
  live `conf_big` evidence that the remaining timeout is now dominated by
  `exact_ready_snapshot_assembly`.

The checked-in `refactor-28` live evidence shows:

- `followup_ready_snapshot_timeout_phase=parse_exec`
- `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
- `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms≈4050`
- `followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms≈54`
- `followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms=null`
- `followup_semantic_path=shadow_state`

That means the remaining problem is not parser-tree construction and not tree-cache install
anymore. It is the exact same-version ready-snapshot assembly itself.

## Goals

- Reduce the amount of exact-path work that still happens inside `ready_snapshot_assembly` before
  the first publishable exact ready snapshot can materialize.
- Replace the remaining monolithic `exact_ready_snapshot_assembly` residual with bounded internal
  attribution.
- Preserve exactness, supersession, and truthful fallback semantics.

## Non-Goals

- Reopening parser-base recovery from `refactor-25`.
- Reopening apply/publish gating from `refactor-26`.
- Reopening `optional_cache_enrichment` or `tree_cache_install` as the primary focus.
- Broadening `didSave` wait budgets as a substitute for runtime improvement.

## Proposed Approach

1. Introduce a save-critical exact mode inside `ready_snapshot_assembly`.
   When the exact same-version producer is on the save-critical path, the runtime should separate
   work that is strictly required to materialize publishable exact ready artifacts from secondary
   assembly work that can happen after first publish or be cancelled on supersession.

2. Split `ready_snapshot_assembly` into bounded internal checkpoints.
   The final names can follow the implementation, but the contract should expose enough detail to
   distinguish at least:
   - tree-to-ready conversion work still required for an exact ready snapshot;
   - exact publishable artifact packaging / attachment work after conversion succeeds;
   - bounded yield or re-check points where save-critical promotion or retarget can take effect.

3. Keep fail-closed behavior.
   If the bounded assembly path still cannot prove current exact artifacts, the system must keep
   the truthful fallback to `shadow_state`. Success means either `conf_big` returns to
   `ready_artifacts`, or the residual moves from generic `exact_ready_snapshot_assembly` to a
   narrower, operator-meaningful checkpoint.

## Alternatives Considered

### 1. Widen the `didSave` wait and relief-valve budgets again

Rejected. `refactor-28` already proved the remaining dominant cost sits inside exact
`ready_snapshot_assembly`. More budget would only stretch latency while leaving the same root
cause in place.

### 2. Accept `ready_snapshot_assembly` as an irreducible hot path and stop at observability

Rejected. The current live evidence still falls back to `shadow_state` on the representative
`conf_big` path. That is acceptable as a truthful residual for `refactor-28`, but not as the final
state for this line of work.

### 3. Target the exact assembly residual directly

Selected. The runtime now has enough attribution to justify a focused change on the remaining hot
slice without reopening already-resolved parser-base, apply-lag, and tree-cache-install issues.

## Validation Strategy

- Targeted backend regressions should prove that exact `didSave` follow-up can materialize ready
  artifacts without paying the full old `ready_snapshot_assembly` cost on the critical path.
- Targeted backend regressions should prove that a remaining miss now reports a bounded assembly
  checkpoint, not the old top-level `exact_ready_snapshot_assembly` bucket.
- Representative `conf_big` live evidence should show either a return to `ready_artifacts` or a
  narrower truthful assembly residual.
