## Context

`refactor-39` fixed the representative same-file save-follow-up routing problem.

The checked-in representative bundle for that change now shows:

- `followup_semantic_path | ready_artifacts=4 | shadow_state=0`;
- `followup_publish_elapsed_ms=1398-1516`;
- `semantic_diagnostics_query_ms=1231-1343`.

The matching representative `p55` leaf drilldown still shows that the dominant residual is inside
semantic diagnostics rather than exact ready-snapshot materialization:

- `semantic_diagnostics_ir_ms=938`;
- `semantic_diagnostics_collect_ms=431`;
- `ast_to_ir_convert_ms=201`;
- `followup_publish_elapsed_ms=1578`.

So the next bottleneck is now downstream of `ready_artifacts`.

`refactor-38` is already the profiling/truthfulness change for diagnostics-only semantic-facts leaf
attribution. This change must build on that work rather than duplicating it.

Even before that leaf split lands, the current representative numbers already narrow the first
implementation branch:

- `semantic_diagnostics_ir_ms - ast_to_ir_convert_ms ≈ 737 ms`;
- `semantic_diagnostics_collect_ms ≈ 431 ms`.

So the facts-build residual is currently the better first target than diagnostics collection.

Current code structure narrows it further:

- `analysis_api.rs::semantic_diagnostics_profiled()` builds diagnostics-only semantic facts via
  `type_inference_v2::build_diagnostics_semantic_facts_with_path_and_checkpoint()`;
- it then immediately converts that result into `SemanticTypeHints` through
  `semantic_type_hints_from_facts()`;
- `SemanticValidationVisitor` consumes only four hint families:
  `assignment_value_type_by_span`, `call_receiver_type_by_span`,
  `call_arg_types_by_span`, and `member_access_object_type_by_span`.

So the diagnostics-only path is already output-narrow but still work-wide: it pays for a broader
facts build before collapsing the result to the four maps that the collector actually reads.

## Goals / Non-Goals

- Goals:
  - reduce representative diagnostics-only semantic-query latency on the same-file save-follow-up
    family after exact-path stabilization;
  - preserve current exact `ready_artifacts` incidence while reducing the semantic residual;
  - preserve truthful diagnostics-only vs full-semantic-facts fallback attribution;
  - refresh representative `p55` and `p56` evidence against the checked-in `refactor-39`
    baseline.
- Non-Goals:
  - re-open ready-snapshot timeout/fallback routing from `refactor-39`;
  - optimize unrelated interactive exact queries outside the representative diagnostics path;
  - treat silent full-path fallback or broader wait budgets as an acceptable latency win;
  - duplicate the profiling-only scope of `refactor-38`.

## Decisions

### 1. Treat diagnostics-only semantic query as the next dominant residual

The representative family no longer fails because it cannot stay on the exact path.

It now succeeds on `ready_artifacts`, but spends most of its remaining wall-clock time inside
semantic diagnostics.

This change therefore targets the diagnostics-only semantic query body, not producer continuity,
parse-exec timing, or client-side latency.

### 2. Use `refactor-38` leaf truthfulness as the branch selector, but keep facts-build as the default first branch

Current evidence is already enough to show that semantic diagnostics dominate, but it is not yet
specific enough to justify hard-coding one guessed leaf as the only valid implementation target.

`refactor-38` should make the diagnostics-only leaf profile truthful. This change should then
reduce the dominant diagnostics-only residual that refreshed representative evidence actually
shows.

Current evidence already makes one branch more likely than the others: facts-build work inside
`ir_ms` is larger than `collect_ms` even before truthful diagnostics-only leaf attribution lands.
So the implementation should start by trying to bound the dominant diagnostics-only facts-build
leaf, and only switch to collector work if refreshed truthful evidence disproves that choice.

### 3. Make the first branch remove unobserved diagnostics-only facts work before collector rewrites

The preferred first optimization is not "rewrite the diagnostics collector" and not a vague
"speed up type inference somehow".

The current diagnostics-only path already exposes a narrower output contract than full semantic
facts: it keeps only the four hint families that `SemanticValidationVisitor` reads.
But the builder still materializes a wider `SemanticFacts` surface before that collapse happens.

So the first implementation branch should eliminate or narrow diagnostics-only work that is
immediately discarded before validation.

The acceptable shapes are:

- build the four `SemanticTypeHints` maps directly on the diagnostics-only path;
- or keep a diagnostics-only facts builder, but reduce it to the fact families required to produce
  those four maps and nothing more.

Either shape is acceptable as long as the refreshed evidence stays truthful and semantic parity is
preserved.

### 4. Treat deeper diagnostics-only sub-leaf cuts as second-order within the builder branch

Once the reduced-output builder branch exists, refreshed truthful leaf evidence from `refactor-38`
should decide whether another diagnostics-only sub-leaf still dominates.

The likely candidates are still inside the current builder path:

- `seed_module_context`;
- `local_function_summaries`;
- statement/body visitation.

But those cuts are not the first acceptance definition for this change. They are only valid if the
post-narrowing diagnostics-only leaf profile still shows them as dominant and if they preserve the
same four-map hint surface plus downstream diagnostics parity.

### 5. Preserve diagnostics-only versus full-path semantics

This change is not allowed to make the representative numbers look better by:

- silently moving supported cases onto the full semantic-facts path;
- publishing stale or weakened semantic diagnostics;
- widening upstream wait budgets and counting that as a semantic win.

Any unsupported optimization must keep truthful fallback attribution.

### 6. Treat diagnostics collection as a secondary branch, not the default target

The collector is still expensive:

- it constructs resolver/metadata/validator state;
- it walks the whole `SemanticProgram`;
- it sorts and deduplicates the final diagnostics vector.

But current measured evidence does not justify making collector work the first implementation
branch. The change should only pivot there if the truthful diagnostics-only leaf profile from
`refactor-38` shows that the facts-build branch is no longer dominant after refreshed evidence.

### 7. Acceptance is two-report-first

`p56` is now the canonical representative family bundle for this incident class.
`p55` remains the leaf drilldown for one representative cycle.

Acceptance must therefore compare both:

- representative-family behavior in `p56`;
- leaf-level semantic residual in `p55`.

The change is not ready if only one report improves while the other regresses or hides the path
identity.

## Alternatives Considered

### 1. Keep optimizing ready-snapshot routing first

Rejected.

The checked-in `refactor-39` bundle already shows `ready_artifacts=4` and `shadow_state=0`.
That is no longer the dominant latency class.

### 2. Start another optimization pass before `refactor-38`

Rejected.

That would turn the next perf step into guesswork about the diagnostics-only hotspot instead of a
measured optimization.

### 3. Start with collector work because it is easier to reason about

Rejected.

`collect_ms` is large, but current evidence already shows a larger residual inside `ir_ms` after
subtracting AST->IR conversion. Taking the easier branch first would be an evidence-last decision.

### 4. Treat aggregate `semantic_diagnostics_query_ms` as sufficient proof

Rejected.

Aggregate query timing is enough to justify a new change, but not enough to choose which internal
diagnostics-only leaf should be rewritten without the truthfulness surface from `refactor-38`.

## Validation Strategy

- Add targeted `analysis-v2` regressions proving the optimized diagnostics-only path still
  preserves the same four observed hint maps, preserves downstream diagnostics parity, and keeps
  truthful fallback when the optimization is not valid.
- Add backend/runtime regressions proving representative traced payloads still expose the correct
  diagnostics semantic path while the optimized residual shrinks.
- Refresh representative `p55` leaf evidence and representative `p56` family evidence against the
  checked-in `refactor-39` bundle.
- Run strict OpenSpec validation before handoff.

## Quality Gates

- Refreshed `p56` representative evidence still stays on `ready_artifacts` for the representative
  family.
- Refreshed `p56` shows lower diagnostics-only semantic-query latency than the checked-in
  `refactor-39` bundle.
- Refreshed `p55` shows that the dominant diagnostics-only residual actually moved down on the
  truthful diagnostics-only path rather than disappearing behind full fallback or coarse aggregate
  attribution.
- If representative evidence only improves by leaving the diagnostics-only path or by regressing
  exactness truthfulness, the change is not ready.
