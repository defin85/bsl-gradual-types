## Context

The post-`refactor-30` incident bundle captured on `2026-04-15` establishes four facts:

1. completion is not the bottleneck in this capture;
2. `didSave` still gets its first syntax-only publish quickly, so `refactor-08..10` remain
   effective on the first publish path;
3. the exact same-version follow-up still misses the bounded wait on representative `conf_big`
   load, and the dominant exact residual is now `program_lowering`;
4. one live trace already violates the intended aggregate relationship between
   `program_conversion_ms` and its constituent slices, which means the exported evidence is no
   longer fully trustworthy for the hottest remaining path.

In other words, the optimization target and the observability target are now coupled:
without a coherent conversion-attribution model, the team cannot trust the next `conf_big`
measurements enough to validate a `program_lowering` optimization.

## Goals

- Bound long-running exact same-version `program_lowering` on the save-critical path.
- Preserve fail-closed exactness and truthful supersession / retarget semantics during bounded
  lowering.
- Guarantee that exported conversion attribution stays internally coherent for the same traced
  target and cycle.

## Non-Goals

- Broaden wait budgets.
- Reopen already-isolated slices (`parser_tree_build`, `tree_cache_install`,
  `syntax_error_collection`) as the primary residual.
- Reframe a diagnostics-save incident as a completion UI / transport issue without contradictory
  fresh evidence.

## Proposed Approach

### 1. Introduce bounded cooperative progress inside exact `program_lowering`

The current exact path still behaves as if `program_lowering` were one large blocking region from
the point of view of save-follow-up observability. The next implementation should introduce
cooperative progress boundaries inside lowering that are meaningful to the runtime, for example:

- top-level declaration / procedure boundaries;
- bounded batches of lowered children;
- explicit observer / cancellation checkpoints aligned with the runtime's exact producer control.

The implementation does not have to expose every internal lowering detail to operators. It does
need enough bounded structure that:

- save-critical promotion is not effectively blind until a monolithic lowering span ends;
- supersession / retarget decisions can take effect at the next bounded lowering checkpoint;
- future live evidence can show whether a single lowering slice still dominates.

### 2. Keep exact publish semantics fail-closed

The optimization target is the save-critical path, not exactness relaxation.

Therefore the implementation may:

- reduce mandatory lowering work before the first publishable exact ready snapshot;
- split lowering into publish-critical and later work only if the runtime can still prove the
  current exact artifacts are sufficient for publish;
- terminate or retarget at bounded lowering checkpoints when a newer same-file target overtakes
  the current one.

The implementation must not:

- publish stale exact artifacts;
- silently downgrade to a weaker semantic contract;
- hide a miss by broadening the wait budget.

### 3. Make conversion attribution tuple-coherent, not field-coherent-by-accident

The current bundle strongly suggests that diagnostics-save timeline merging can combine
independent maxima from different probe snapshots into one final trace. That is why a single trace
can end up with:

- `program_conversion_ms=654`
- `program_lowering_ms=3363`
- `publishable_artifact_packaging_ms=2`

This is not just cosmetically odd. It weakens operator trust in the exact bottleneck.

The next implementation should therefore treat conversion attribution as a coherent tuple for the
same `(file_id, requested_version, text_hash, save_cycle_sequence)` target, rather than as
unrelated scalar maxima. Acceptable implementation strategies include:

- recomputing aggregate `program_conversion_ms` from the merged slice fields after merge;
- storing the most authoritative single attribution snapshot for the target and deriving all
  aggregate fields from that snapshot;
- resetting or separating follow-up attribution when a new traced target supersedes the previous
  one within the same save cycle.

Whichever strategy is chosen, the exported invariants must hold:

- `program_conversion_ms` is absent or greater than or equal to every constituent conversion slice
  present in the same trace;
- dominant checkpoint and dominant duration come from the same target-coherent attribution view;
- stale aggregate timing from an older target is not merged into a newer target's follow-up trace.

### 4. Validate on representative `conf_big`, not only synthetic fixtures

Synthetic regressions are necessary but not sufficient here. The previous refactors already showed
that synthetic coverage can pass while representative mixed load still falls back to `shadow_state`.

This change therefore needs both:

- targeted backend regressions for lowering checkpoints and attribution coherence;
- checked-in repo-local live evidence on `examples/conf_big`.

## Alternatives Considered

### 1. Widen the wait budgets again

Rejected. The bundle still shows `program_lowering` as the dominant exact residual. More budget
would only hide the same hot path behind worse latency.

### 2. Do a pure observability cleanup first

Rejected. The observability coherence problem is real, but the bundle also still shows a clear
runtime hotspot. Splitting the work into "instrumentation only" and "runtime only" changes would
slow the feedback loop and preserve ambiguity in the next live evidence.

### 3. Chase completion transport or VS Code UI

Rejected for this incident class. The captured completion traces do not support that theory.

## Validation Strategy

- Add backend regressions that keep a worker inside bounded `program_lowering`, then verify
  save-critical publish / supersession behavior at the new lowering checkpoints.
- Add backend regressions that prove one diagnostics-save trace cannot report
  `program_conversion_ms < program_lowering_ms` or `< publishable_artifact_packaging_ms`.
- Refresh representative `conf_big` live evidence and compare it directly against the checked-in
  `refactor-30` baseline.
