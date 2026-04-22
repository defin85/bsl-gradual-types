# Change: reduce save-follow-up diagnostics cost by splitting diagnostics-only semantic hints from full `SemanticFacts`

## Why

Representative `p55` evidence on `2026-04-17` already showed that the remaining diagnostics-side
cost was no longer dominated by local-function summary solving after `refactor-34` and
`refactor-35`, and a later same-day rerun after the parser-side `refactor-37` reduction kept
semantic diagnostics as the dominant residual:

- post-`refactor-35`: `semantic_diagnostics_query_ms` remained about `1293ms`,
  `semantic_diagnostics_ir_ms` about `802ms`, `semantic_facts_materialize_ms` about `613ms`,
  `visit_statements_ms` about `290ms`, `visit_callable_body_ms` about `201ms`, and
  `local_function_summaries_ms` was already lower at about `249ms`;
- post-`refactor-37`: parser-side `program_lowering_ms` dropped to `244ms`, but the same
  representative path still reported `semantic_diagnostics_query_ms=2451` and
  `semantic_diagnostics_ir_ms=1611`.

The current diagnostics path still materializes full `SemanticFacts` even though semantic
diagnostics only consume a narrower set of type-hint maps.
That means same-file save follow-ups still pay for definition-location, method-target, constructor,
and other full semantic-facts work that diagnostics do not need.

## What Changes

- Require semantic diagnostics to support a dedicated diagnostics-only type-hints artifact instead
  of always materializing full `SemanticFacts`.
- Keep full `SemanticFacts` as the exact semantic contract for interactive features such as
  completion, hover, definition, `signatureHelp`, `type-at-position`, and other full exact queries.
- Require diagnostics-only semantic artifacts to stay isolated from the current exact semantic cache
  key so a trimmed diagnostics artifact cannot poison later interactive requests.
- Add parity, cache-isolation, and observability regressions plus refreshed representative `p55`
  live evidence against the latest post-parser-side baseline.

## Sequence

This change intentionally follows:

- `refactor-34-local-function-summaries-scc-fast-path`
- `refactor-35-exact-program-lowering-reuse-materialization`
- `refactor-37-exact-program-lowering-callable-body-partial-rebuild`

`refactor-34` removed the major local-summary hotspot.
`refactor-35` and `refactor-37` are the parser-side exact-path reductions that make the remaining
`p55` tail more legible.
The next sequential step is then to remove diagnostics-only work from the full semantic-facts path
without weakening the interactive exact semantic contract.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/lib/snapshots.rs`
  - `analysis-v2/src/lib/analysis_api.rs`
  - `analysis-v2/src/derived_artifacts.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `shared/src/ir/semantic_facts.rs`
  - `semantic-diagnostics/src/type_hints.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - diagnostics-path observability and representative perf evidence

## Non-Goals

- Do not store diagnostics-only semantic artifacts under the existing full exact semantic cache key.
- Do not weaken diagnostics correctness or silently drop diagnostics that currently depend on the
  canonical exact semantic contract.
- Do not reopen parser-path exact lowering work in this change.
- Do not treat transport, UI, or generic background-lane policy as the primary fix for this
  residual.
