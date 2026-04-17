## Context

After `refactor-34`, representative save-follow-up evidence still shows a large diagnostics-side
semantic residual even when the heavy follow-up publishes through `ready_artifacts`:

- `semantic_diagnostics_query_ms` is about `1293ms`;
- `semantic_diagnostics_ir_ms` is about `802ms`;
- `semantic_facts_materialize_ms` is about `613ms`;
- `visit_statements_ms` is about `290ms`;
- `visit_callable_body_ms` is about `201ms`.

This residual is no longer primarily about local-function summary convergence.
It is primarily about building more semantic facts than semantic diagnostics actually consume.

The current contract shape makes that easy to see:

- `analysis-v2/src/lib/snapshots.rs` always routes semantic diagnostics through full semantic-facts
  materialization before extracting diagnostics hints;
- `shared/src/ir/semantic_facts.rs` stores far more than diagnostics need;
- `semantic-diagnostics/src/visitor.rs` ultimately consumes only diagnostics-facing type hints;
- `semantic-diagnostics/src/type_hints.rs` narrows that need to a small set of maps.

## Goals / Non-Goals

- Goals:
  - reduce diagnostics-side semantic materialization cost on representative save follow-ups by
    building only the type hints semantic diagnostics need;
  - preserve diagnostics correctness and keep full `SemanticFacts` as the contract for interactive
    exact features;
  - isolate diagnostics-only artifacts so they cannot poison later interactive queries.
- Non-Goals:
  - alter the parser-side exact lowering contract;
  - reuse the existing full exact semantic cache key for trimmed diagnostics artifacts;
  - rely on warm-host cache seeding or other speculative cache tricks that already failed to show a
    representative live win.

## Decisions

### 1. Introduce a dedicated diagnostics-only type-hints artifact

Semantic diagnostics do not need the full `SemanticFacts` surface.
This change should introduce a narrower diagnostics artifact containing only the type-hint maps
consumed by semantic diagnostics, rather than materializing all full semantic facts first.

At minimum that artifact should cover:

- `assignment_value_type_by_span`;
- `call_receiver_type_by_span`;
- `call_arg_types_by_span`;
- `member_access_object_type_by_span`.

### 2. Keep full `SemanticFacts` for interactive exact features

Completion, hover, definition, type-at-position, and other interactive exact features should keep
their current full semantic contract.
The diagnostics-only path is a narrower consumer optimization, not a redefinition of the canonical
exact semantic artifact.

### 3. Diagnostics-only materialization may skip work that diagnostics do not consume

If representative evidence confirms that some full semantic-facts work is diagnostics-irrelevant on
syntactically valid non-interactive targets, the diagnostics-only path may skip it.
That includes source-driven incomplete-member-access recovery only where soundness is proven for the
diagnostics contract.

Fail-closed behavior still applies: if the narrower path cannot prove parity for a case, it should
fall back to the full path rather than publish reduced diagnostics silently.

### 4. Cache isolation is mandatory

Diagnostics-only artifacts must be ephemeral or live under a separate diagnostics cache namespace.
They must not be stored under the current full exact semantic cache key.

In the current architecture, that also means diagnostics-only materialization cannot reuse or
overwrite the existing exact `ir_profiled` / `remember_ir_artifact` slot and cannot publish a
trimmed `SemanticProgram` or completion-head substitute into the current interactive exact cache
path.

Otherwise a trimmed artifact could be mistaken for a full exact semantic artifact by later
interactive queries, which would poison completion/hover/definition behavior.

### 5. Observability must show which semantic path ran

Representative evidence must distinguish:

- diagnostics-only hint materialization;
- full semantic-facts materialization fallback;
- the remaining diagnostics collection/query cost.

Without that split, a timing win would be hard to attribute and regressions would be harder to
localize.

### 6. Keep this change sequentially after the parser-path reduction

`refactor-35` should land first so parser-path improvement and diagnostics-path improvement remain
separately measurable on `p55`.
This avoids conflating exact lowering wins with semantic-facts wins in the same representative
bundle.

## Alternatives Considered

### 1. Add a diagnostics mode on the current full exact semantic cache key

Rejected.
That would risk cache poisoning between diagnostics-only and interactive exact consumers.

### 2. Keep full `SemanticFacts` and only micro-optimize `visit_statements`

Rejected for now.
It does not fix the larger contract problem that diagnostics currently pay for semantic facts they
do not consume.

### 3. Reuse warm-host or shared-cache seeding as the main fix

Rejected.
Representative live testing already failed to show a meaningful `p55` win from that approach.

## Validation Strategy

- Add parity regressions comparing diagnostics output between the full semantic path and the new
  diagnostics-only hints path.
- Add cache-isolation regressions proving diagnostics-only queries cannot poison later interactive
  exact requests.
- Refresh representative `p55` evidence against the post-`refactor-35` baseline.

## Quality Gates

- Representative `p55` still publishes through `ready_artifacts`.
- Representative evidence shows the diagnostics path used diagnostics-only hints or truthfully fell
  back to full semantic facts.
- Representative diagnostics-side residual is materially lower than the post-`refactor-35`
  baseline.
- Interactive exact features remain on full `SemanticFacts`, and cache-isolation regressions pass.
