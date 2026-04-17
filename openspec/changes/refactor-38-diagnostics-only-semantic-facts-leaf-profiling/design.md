## Context

`refactor-36` intentionally removed full `SemanticFacts` materialization from the representative
diagnostics-only path.

That worked: the latest `p55` report now shows truthful path selection
(`ready_artifacts` + `snapshot` + diagnostics-only behavior) and no longer exports full
semantic-facts subphases on that path.

But the current observability is now too coarse for the next decision:

- `semantic_diagnostics_ir_ms=837`
- `ast_to_ir_convert_ms=214`
- diagnostics-only leaf breakdown for the remaining IR work is missing

The old full-semantic-facts leaves are now `null`, which is correct, but that leaves a large
unexplained residual inside the diagnostics-only builder.

## Goals / Non-Goals

- Goals:
  - expose truthful leaf attribution for diagnostics-only semantic-facts work;
  - preserve clear separation between diagnostics-only leaves and full-semantic-facts leaves;
  - refresh representative `p55` evidence so the next optimization target is chosen from measured
    data instead of inference.
- Non-Goals:
  - optimize the diagnostics-only builder itself in this change;
  - redesign cache isolation or the diagnostics-only contract introduced by `refactor-36`;
  - widen parser-side exact reuse or assembly work covered by `refactor-35` / `refactor-37`.

## Decisions

### 1. Add a dedicated diagnostics-only leaf profile instead of reusing full-semantic-facts fields

The diagnostics-only builder should not keep exporting through the old full-semantic-facts leaf
surface.

Those fields already have a clear meaning:

- full semantic-facts materialization subphases;
- recovery and summary work associated with the full semantic contract.

Reusing those names for diagnostics-only work would blur the distinction that `refactor-36`
introduced. This change therefore requires a dedicated diagnostics-only leaf profile and a
dedicated exported namespace for that path.

### 2. Thread diagnostics-only profile data from the builder instead of reconstructing it downstream

The current helper
`build_diagnostics_semantic_facts_with_path_and_checkpoint(...)`
returns only `SemanticFacts`.

That forces downstream code to know only:

- aggregate diagnostics-only IR time;
- aggregate AST->IR time;
- no path-specific leaf timings for the facts builder.

The builder should instead return a profiled result that carries both:

- the built diagnostics-only facts;
- the diagnostics-only leaf profile for that build.

This keeps attribution truthful and avoids heuristic reconstruction later in the runtime.

### 3. Keep skipped full-path leaves truthful

When diagnostics-only materialization runs, skipped full-semantic-facts leaves must stay absent or
zero.

This change is not allowed to make representative reports look “more detailed” by incorrectly
backfilling old full-path fields with diagnostics-only timings.

### 4. Representative traced payloads must expose the materialization path directly

`SemanticDiagnosticsProfile.materialization_path` already exists in `analysis-v2`, and the runtime
already records it into cumulative metrics.

That is not sufficient for this change. Representative reports and diagnostics-save timeline
payloads also need the traced path directly. Otherwise a new diagnostics-only leaf family would
still require operators to infer its meaning indirectly from missing full-path leaves or from
separate cumulative counters.

### 5. Representative evidence must compare against the `refactor-36` baseline

This change is only useful if the refreshed `p55` report proves that the diagnostics-only residual
is now attributable.

The acceptance comparison point is the `2026-04-17` `refactor-36` report, which currently shows:

- `followup_publish_elapsed_ms=1371`
- `semantic_diagnostics_query_ms=1224`
- `semantic_diagnostics_ir_ms=837`
- `semantic_diagnostics_collect_ms=383`
- `ast_to_ir_convert_ms=214`
- no diagnostics-only leaf attribution for the remaining IR work

## Alternatives Considered

### 1. Start another optimization change immediately

Rejected.

The current evidence is still too coarse inside the diagnostics-only builder. Another optimization
change would likely guess at the hotspot instead of measuring it.

### 2. Reuse the old full-semantic-facts leaf names for diagnostics-only work

Rejected.

That would make the exported report harder to interpret and would weaken the truthfulness gained by
`refactor-36`.

### 3. Treat `collect_ms` as the only next hotspot and skip diagnostics-only IR profiling

Rejected.

`collect_ms=383` is still a plausible follow-up target, but `semantic_diagnostics_ir_ms=837`
remains larger and is not yet sufficiently attributed.

## Validation Strategy

- Add `analysis-v2` regressions that assert diagnostics-only profile data is returned when the
  diagnostics-only path runs.
- Add observability/export regressions that assert diagnostics-only leaf fields and the traced
  `materialization_path` are exported together, and full-only leaf fields remain absent or zero on
  that same path.
- Refresh representative `p55` evidence and compare it to the `refactor-36` baseline.
- Run strict OpenSpec validation before handoff.

## Quality Gates

- Representative `p55` still publishes through `ready_artifacts`.
- The report still truthfully identifies the diagnostics semantic path as diagnostics-only via an
  explicit traced `materialization_path`.
- Diagnostics-only IR work is no longer explained only by aggregate `ir_ms` plus `ast_to_ir`.
- Full-semantic-facts-only leaves stay absent or zero on the diagnostics-only path.
- If the refreshed report still leaves most diagnostics-only IR time unattributed, the change is
  not ready.
