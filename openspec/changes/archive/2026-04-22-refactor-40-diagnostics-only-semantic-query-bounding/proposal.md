# Change: bound the diagnostics-only semantic-facts build residual on the representative save-follow-up family

## Why

`refactor-39` moved the representative same-file `didSave` heavy follow-up family back onto the
current exact path.

The checked-in representative bundle now shows:

- `followup_semantic_path | ready_artifacts=4 | shadow_state=0`;
- `followup_publish_elapsed_ms=1398-1516`;
- `semantic_diagnostics_query_ms=1231-1343`.

The matching representative leaf drilldown still shows the dominant residual inside semantic
diagnostics rather than ready-snapshot routing:

- `semantic_diagnostics_ir_ms=938`;
- `semantic_diagnostics_collect_ms=431`;
- `ast_to_ir_convert_ms=201`;
- `followup_publish_elapsed_ms=1578`.

This means the next bottleneck is no longer exact ready-snapshot continuity (`refactor-39`).
It is the diagnostics-only semantic query body, and current measured evidence already points first
to the diagnostics-only semantic-facts build residual inside `ir_ms`:

- `semantic_diagnostics_ir_ms - ast_to_ir_convert_ms ≈ 737 ms`;
- `semantic_diagnostics_collect_ms ≈ 431 ms`.

So the first implementation branch should target diagnostics-only facts-build work before
revisiting diagnostics collection.

Current code structure also makes that first branch more concrete than a generic "speed up the
builder" instruction:

- `semantic_diagnostics_profiled()` builds diagnostics-only semantic facts through
  `build_diagnostics_semantic_facts_with_path_and_checkpoint()`;
- that result is then immediately collapsed by `semantic_type_hints_from_facts()`;
- `SemanticValidationVisitor` consumes only four hint families:
  `assignment_value_type_by_span`, `call_receiver_type_by_span`,
  `call_arg_types_by_span`, and `member_access_object_type_by_span`.

So the first implementation branch should try to stop paying for diagnostics-only facts output
that is never observed by the diagnostics visitor, before it starts rewriting the collector.

`refactor-38-diagnostics-only-semantic-facts-leaf-profiling` is still needed to make that
residual fully truthful, but it is profiling-only. It does not reduce the representative latency.

## What Changes

- Require the representative same-file save-follow-up family to reduce diagnostics-only semantic
  query latency once that family already remains on current exact `ready_artifacts`.
- Require the first optimization branch to target the dominant diagnostics-only semantic-facts
  build residual inside `ir_ms`, unless refreshed truthful leaf evidence from `refactor-38`
  proves another diagnostics-only leaf is larger.
- Require that first branch to eliminate or narrow diagnostics-only work that is immediately
  collapsed away before `SemanticValidationVisitor` runs, preferably by producing the four
  observed hint maps directly or through a reduced diagnostics-only facts surface.
- Require any deeper builder cut inside `seed_module_context`, `local_function_summaries`, or
  statement visitation to be justified by refreshed truthful diagnostics-only leaf evidence rather
  than picked upfront by convenience.
- Require the optimization to preserve diagnostics-only vs full-semantic-facts fallback
  truthfulness, rather than claiming a win by silently widening wait budgets or shifting work into
  a different semantic path.
- Require refreshed representative `p56` family evidence and `p55` leaf evidence that compare the
  reduced diagnostics semantic residual against the checked-in `refactor-39` baseline.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/lib/analysis_api.rs`
  - `analysis-v2/src/lib/snapshots.rs`
  - `analysis-v2/src/lib.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `semantic-diagnostics/src/type_hints.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `bsl-runtime/src/system/basic_observability/**`
  - representative backend/runtime tests and live-evidence assets
- Follow-up relationship:
  - builds on `refactor-36-diagnostics-semantic-hints-split`
  - depends on truthful leaf attribution from
    `refactor-38-diagnostics-only-semantic-facts-leaf-profiling`
  - uses the checked-in `refactor-39` representative bundle as the latency baseline
  - does not target ready-snapshot timeout/fallback routing or client/UI latency
