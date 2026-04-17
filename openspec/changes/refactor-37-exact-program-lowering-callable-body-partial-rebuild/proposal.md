# Change: reduce exact `program_lowering` residual via partial rebuild inside changed callable bodies

## Why

Representative parser-side evidence on `2026-04-17` shows that `refactor-35` removed the
reuse-materialization bottleneck, but did not finish the exact-path parser residual.

The latest live `p55` attribution shows:

- exact `program_lowering` still dominates exact assembly on the representative save-follow-up path;
- top-level reuse is already qualifying (`reuse_outcome=top_level_reuse`);
- reuse-plan build and reused-region work are now negligible (`build_ms=1`, `rebase_ms=0`,
  `reused_progress_ms=0`);
- the remaining parser CPU is concentrated inside one rebuilt callable body rather than in reused
  regions.

In the latest traced follow-up, the direct rebuilt callable-body dispatch path accounted for almost
all rebuilt callable cost (`rebuild_dispatch_callable_body_dispatch_ms=1399` over `45`
body-dispatch units), while callable non-body overhead remained negligible.

This means the next parser-side step is not more reuse materialization and not `refactor-36`.
It is to stop rebuilding the whole changed callable body when only a bounded local region inside
that body actually changed.

## What Changes

- Require exact same-version `program_lowering` to derive a conservative body-local rebuild plan
  inside one rebuilt callable body when the changed ranges stay within safe body-local boundaries.
- Require fail-closed fallback to whole-callable rebuild when body-local invalidation boundaries are
  ambiguous or not yet proven sound.
- Require representative observability that distinguishes direct rebuilt callable-body work from
  outer aggregate dispatch totals, so acceptance can prove that less callable-body work was rebuilt.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code: `syntax/src/tree_sitter_adapter/**`,
  `bsl-runtime/src/system/parser_coordinator.rs`,
  `backend/src/bin/lsp_server/server/core/**`,
  diagnostics-save live evidence/tests
- Follow-up relationship:
  - builds on `refactor-33-exact-program-lowering-changed-range-reuse`
  - follows `refactor-35-exact-program-lowering-reuse-materialization`
  - remains separate from `refactor-36-diagnostics-semantic-hints-split`
