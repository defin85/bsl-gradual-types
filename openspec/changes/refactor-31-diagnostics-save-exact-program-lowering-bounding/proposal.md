# Change: bound exact `program_lowering` and keep conversion attribution coherent

## Why

The `2026-04-15` observability incident bundle captured on `0.4.151` / `git 5ddc793e` confirms
that the current `conf_big` bottleneck is no longer in completion transport, UI ingress, parser
tree construction, tree-cache install, or deferred syntax-error collection.

The bundle shows:

- completion traces are effectively clean on the server/transport path (`136ms` and `2ms`, with
  `transport_to_handler_wait_ms=0` and `response_output_handoff_send_wait_ms<=1ms`);
- `didSave` still gets a fast first `syntax_only` publish in `50-55ms`;
- the heavy follow-up still returns through `shadow_state` after `7.2-8.1s`;
- the exact ready-snapshot timeout path is now
  `parse_exec -> core_parse_build -> exact_ready_snapshot_assembly -> program_lowering`;
- one trace already exposes an internal coherence problem:
  `program_conversion_ms=654` while `program_lowering_ms=3363` and
  `publishable_artifact_packaging_ms=2`.

That means `refactor-08` through `refactor-30` did what they were supposed to do:

- protect the first publish path;
- isolate the exact-path residual down to `program_lowering`;
- expose enough observability to justify a narrower next step.

What remains is now twofold:

1. the save-critical exact path still spends too much time inside `program_lowering`;
2. the operator-facing conversion attribution is no longer fully trustworthy once multiple probe
   snapshots are merged into a single `didSave` trace.

## What Changes

- Require the exact same-version ready-snapshot producer to bound `program_lowering` on the
  save-critical path, so the first publishable exact ready snapshot no longer depends on one
  monolithic lowering span.
- Require bounded cooperative lowering checkpoints that preserve exactness while still allowing
  promotion, supersession, or retarget decisions to take effect during long-running lowering.
- Require diagnostics-save timeline / incident bundle attribution for
  `program_conversion` / `program_lowering` / `publishable_artifact_packaging` to remain
  internally coherent for the same traced target and cycle.
- Require targeted regressions and representative `conf_big` live evidence proving that the next
  residual is either smaller, or at least reported coherently and truthfully.

## Sequence

This change intentionally follows:

- `refactor-25-did-change-parser-base-reuse-and-parse-exec-bounding`
- `refactor-26-diagnostics-save-exact-publish-apply-lag-isolation`
- `refactor-27-diagnostics-save-exact-parse-exec-bounding`
- `refactor-28-diagnostics-save-exact-core-parse-build-bounding`
- `refactor-29-diagnostics-save-exact-ready-snapshot-assembly-bounding`
- `refactor-30-diagnostics-save-exact-ready-snapshot-program-conversion-bounding`

`refactor-30` proved that the dominant exact-path residual on representative `conf_big` load is
now `program_lowering`, not the old top-level `program_conversion` bucket. The new bundle also
shows that the current conversion-attribution merge is not yet coherence-safe. This change targets
both facts directly.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `syntax/src/tree_sitter_adapter/*`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - diagnostics save timeline / incident bundle rendering and checked-in live evidence

## Non-Goals

- Do not widen the `didSave` bounded wait or relief-valve budgets as the primary fix.
- Do not reopen UI-first or transport-first investigation for this class of incident.
- Do not reopen parser-base recovery, parser-tree build, tree-cache install, or deferred
  `syntax_error_collection` as the primary performance target.
- Do not relax exact same-version guarantees or publish stale diagnostics.
