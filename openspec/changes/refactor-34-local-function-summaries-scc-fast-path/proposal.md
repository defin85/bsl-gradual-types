# Change: reduce semantic diagnostics latency by eliminating avoidable `local_function_summaries` fixed-point work

## Why

Fresh representative live evidence on the current workspace shows that the dominant residual for
`didSave` heavy follow-up is now inside canonical semantic diagnostics rather than routing or
fallback policy:

- `followup_publish_semantic_path=ready_artifacts`;
- `semantic_diagnostics_query_ms=5251`;
- `semantic_diagnostics_ir_ms=3954`;
- `semantic_facts_materialize_ms=3494`;
- `local_function_summaries_ms=2343`;
- `local_function_summaries_fixed_point_ms=2335`;
- `local_function_summaries_function_count=311`;
- `local_function_summaries_scc_count=311`;
- `local_function_summaries_fixed_point_iteration_count=622`.

That profile means the current solver is still paying recursive fixed-point costs even when the
observed SCC workload is effectively singleton and non-recursive. It also keeps rebuilding a
file-wide local-summary snapshot on each iteration, so the representative save-follow-up keeps
spending real CPU on avoidable summary orchestration rather than on unavoidable semantic work.

## What Changes

- Require canonical local-function-summary inference to short-circuit singleton non-recursive SCCs
  instead of sending them through the general fixed-point loop.
- Require recursive SCC solving to reuse stable out-of-SCC summaries from a base lookup and to
  rebuild only the current SCC overlay on each iteration, rather than rebuilding a full-file
  snapshot every time.
- Require parity-preserving semantics: self-recursive or mutually recursive SCCs MUST still use a
  convergence path, and the new fast path MUST NOT weaken exact semantic results.
- Require representative evidence and low-cardinality observability proving that the residual moved
  away from file-wide fixed-point churn.

## Sequence

This change intentionally follows:

- `refactor-31-diagnostics-save-exact-program-lowering-bounding`
- `refactor-32-ready-snapshot-shadow-state-lag-reduction`
- `refactor-33-exact-program-lowering-changed-range-reuse`

Those changes made the exact save-follow-up path observable, restored `ready_artifacts` on the
representative path, and reduced one earlier exact CPU hotspot. The next step is to remove the
avoidable semantic-facts work that remains visible on the same representative save-follow-up.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/type_inference_v2/local_function_summaries.rs`
  - `analysis-v2/src/lib.rs`
  - `analysis-v2/src/lib/analysis_api.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - representative live evidence / diagnostics-save timeline exports

## Non-Goals

- Do not widen `didSave` wait budgets or hide the current residual behind routing changes.
- Do not weaken exact same-version semantics or introduce stale semantic substitutes.
- Do not perform a full dataflow/solver rewrite beyond the local-function-summary path needed for
  this hotspot.
- Do not treat transport, UI, or `shadow_state` routing as the primary target for this change
  unless fresh contradictory evidence appears.
