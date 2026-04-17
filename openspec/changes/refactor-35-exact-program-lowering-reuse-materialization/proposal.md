# Change: reduce exact `program_lowering` residual by eliminating clone-heavy reuse materialization

## Why

Representative `p55` evidence on `2026-04-17` shows that the previous save-follow-up fixes changed
the bottleneck again:

- the heavy follow-up already publishes through `ready_artifacts`;
- `local_function_summaries` is no longer the dominant residual after `refactor-34`;
- the exact parse-side tail still spends about `946ms` inside
  `exact_ready_snapshot_assembly -> program_lowering`;
- the same report already shows reuse is qualifying:
  `program_lowering_reuse_outcome=top_level_reuse`,
  `reused_lowering_units=2042`,
  `fully_reused_top_level_node_count=314`,
  `rebuilt_lowering_units=46`.

This means `refactor-33` proved the reuse planner, but the exact path still pays too much to
materialize already-proven reused units. Current exact assembly deep-clones and rebases unchanged
`Statement` subtrees and callable-body windows before final `Program` assembly, so large unchanged
regions remain expensive even when the reuse plan is already known.

## What Changes

- Require exact ready-snapshot assembly to consume safe lowering reuse plans by ownership, so
  unchanged top-level statements and callable-body windows are moved into the final `Program`
  instead of being deep-cloned a second time.
- Preserve the conservative invalidation boundaries and fail-closed rebuild semantics introduced by
  `refactor-33-exact-program-lowering-changed-range-reuse`; this change reduces materialization
  cost and MUST NOT relax exactness.
- Preserve truthful reuse-versus-rebuild observability after plan consumption, so representative
  evidence proves that the exact path performed less work rather than only reporting a different
  label.
- Add targeted regressions and refresh representative `p53` / `p55` live evidence against the
  current `2026-04-17` baseline.

## Sequence

This change intentionally follows:

- `refactor-33-exact-program-lowering-changed-range-reuse`
- `refactor-34-local-function-summaries-scc-fast-path`

`refactor-33` established conservative reuse detection for exact `program_lowering`.
`refactor-34` reduced the major semantic hotspot that previously masked the remaining save-follow-up
tail.
The next step is therefore not more reuse detection and not a diagnostics semantic refactor yet.
It is to make the already-proven exact reuse plan cheap to materialize.

The diagnostics-only semantic split is intentionally deferred into the next sequential change so
parser-path improvement and diagnostics-path improvement remain separately measurable.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `syntax/src/tree_sitter_adapter/mod.rs`
  - `syntax/src/tree_sitter_adapter/statement_converter/mod.rs`
  - `syntax/src/tree_sitter_adapter/statement_converter/declarations.rs`
  - exact save-follow-up observability and representative perf evidence

## Non-Goals

- Do not widen bounded waits or re-open save-follow-up routing as the primary fix.
- Do not weaken reuse invalidation boundaries or publish stale exact artifacts.
- Do not redesign the whole AST ownership model or introduce broad structural sharing in this
  change unless fresh evidence later proves move-based consumption is still insufficient.
- Do not mix diagnostics-only semantic work into this parser-path change.
