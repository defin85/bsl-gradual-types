## Context

The exact same-version ready-snapshot path has already been narrowed in five steps:

- `refactor-25` removed `stale_parser_base` as the dominant root cause and bounded obsolete parse
  waste;
- `refactor-26` removed post-ready publish/apply gating as the primary blocker on the exact path;
- `refactor-27` separated `optional_cache_enrichment` from `parse_exec`;
- `refactor-28` separated `tree_cache_install` from the save-critical exact path;
- `refactor-29` separated deferred `syntax_error_collection` from the save-critical exact
  assembly path and proved in live `conf_big` evidence that the remaining timeout is now dominated
  by `program_conversion`.

The checked-in `refactor-29` live evidence shows:

- `followup_ready_snapshot_timeout_phase=parse_exec`
- `followup_ready_snapshot_parse_exec_timeout_subphase=core_parse_build`
- `followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint=exact_ready_snapshot_assembly`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint=program_conversion`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms≈4034`
- `followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms=null`
- `followup_semantic_path=shadow_state`

That means the remaining problem is not parser-tree construction, not tree-cache install, and not
deferred syntax-error collection anymore. It is the exact same-version `program_conversion`
itself.

## Goals

- Reduce the amount of exact-path work that still happens inside `program_conversion` before the
  first publishable exact ready snapshot can materialize.
- Replace the remaining monolithic `program_conversion` residual with bounded internal attribution.
- Preserve exactness, supersession, and truthful fallback semantics.

## Non-Goals

- Reopening parser-base recovery from `refactor-25`.
- Reopening apply/publish gating from `refactor-26`.
- Reopening `optional_cache_enrichment`, `tree_cache_install`, or deferred
  `syntax_error_collection` as the primary focus.
- Broadening `didSave` wait budgets as a substitute for runtime improvement.

## Proposed Approach

1. Introduce a save-critical exact mode inside `program_conversion`.
   When the exact same-version producer is on the save-critical path, the runtime should separate
   work that is strictly required to materialize publishable exact ready artifacts from secondary
   conversion work that can happen after first publish or be cancelled on supersession.

2. Split `program_conversion` into bounded internal checkpoints.
   The final names can follow the implementation, but the contract should expose enough detail to
   distinguish at least:
   - tree-to-program lowering or conversion work still required for an exact ready snapshot;
   - exact publishable artifact packaging / ownership handoff after lowering succeeds;
   - bounded yield or re-check points where save-critical promotion or retarget can take effect.

3. Keep fail-closed behavior.
   If the bounded conversion path still cannot prove current exact artifacts, the system must keep
   the truthful fallback to `shadow_state`. Success means either `conf_big` returns to
   `ready_artifacts`, or the residual moves from generic `program_conversion` to a narrower,
   operator-meaningful checkpoint.

## Alternatives Considered

### 1. Widen the `didSave` wait and relief-valve budgets again

Rejected. `refactor-29` already proved the remaining dominant cost sits inside exact
`program_conversion`. More budget would only stretch latency while leaving the same root cause in
place.

### 2. Reopen deferred `syntax_error_collection` as the primary target

Rejected. The checked-in `refactor-29` live evidence already shows
`followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms=null`
on the representative mixed path. That slice is no longer the dominant residual.

### 3. Target the exact conversion residual directly

Selected. The runtime now has enough attribution to justify a focused change on the remaining hot
slice without reopening already-resolved parser-base, apply-lag, tree-cache-install, or
syntax-error-assembly issues.

## Validation Strategy

- Targeted backend regressions should prove that exact `didSave` follow-up can materialize ready
  artifacts without paying the full old `program_conversion` cost on the critical path.
- Targeted backend regressions should prove that a remaining miss now reports a bounded conversion
  checkpoint, not the old top-level `program_conversion` bucket.
- Representative `conf_big` live evidence should show either a return to `ready_artifacts` or a
  narrower truthful conversion residual.
