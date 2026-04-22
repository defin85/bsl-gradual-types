# Change: bound same-version ready-snapshot latency that remains opaque before the first `parse_exec` subphase callback

## Why

The fresh incident bundle captured at `2026-04-18T18:52:50Z` on `0.4.155` / `git f3e72b9e`
confirms that the previous `didSave` follow-up bottleneck changed again.

On the representative same-file save-follow-up family:

- terminal fallback is no longer the dominant issue:
  `followup_semantic_path | ready_artifacts=2 | shadow_state=0`;
- raw timeline now shows
  `followup_ready_snapshot_continuation_reason=continued_still_current`;
- diagnostics-only semantic work is no longer the dominant residual:
  `semantic_diagnostics_query_ms=459-467`;
- but the heavy follow-up still publishes only at `5052-5219 ms` because the same-version exact
  producer still spends `3222-3327 ms` inside ready-snapshot `parse_exec`;
- both traces still report
  `followup_ready_snapshot_timeout_leaf=before_first_parse_exec_subphase`.

This means the next incident class is no longer:

- "`timeout -> engaged_timed_out -> shadow_state` while a still-current producer exists"
  (`refactor-39`);
- or "diagnostics-only semantic query dominates after exact-path stabilization"
  (`refactor-40`).

It is now: "the same-version exact producer stays current and eventually wins, but representative
latency is still trapped inside an opaque pre-subphase `parse_exec` residence before the first
bounded callback."

## What Changes

- Require the representative same-file `didSave` follow-up family to bound the pre-subphase
  `parse_exec` residence of the exact producer that eventually publishes through `ready_artifacts`.
- Require the first implementation branch to reduce or split the work currently reported as
  `before_first_parse_exec_subphase`, rather than assuming a later checkpoint such as
  `program_lowering` remains dominant without refreshed proof.
- Require the change to preserve exactness, latest-wins supersession, and truthful
  continuation/fallback evidence while keeping the existing bounded wait and relief-valve budgets
  as the primary latency envelope.
- Require refreshed representative incident evidence against the `2026-04-18T18:52:50Z` bundle
  baseline, including `followup_publish`, ready-snapshot materialization, and `parse_exec`
  comparisons.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `bsl-runtime/src/system/basic_observability/**`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `syntax/src/tree_sitter_adapter/**`
  - representative backend/runtime tests and incident-bundle assets
- Follow-up relationship:
  - builds on `refactor-39-ready-snapshot-timeout-shadow-state-fallback-bounding`
  - uses the `2026-04-18T18:52:50Z` incident bundle as the new baseline
  - is intentionally separate from `refactor-40-diagnostics-only-semantic-query-bounding`
  - does not assume active `refactor-35-exact-program-lowering-reuse-materialization` is already
    sufficient, because the current truthful timeout leaf still stops before the first
    `parse_exec` subphase callback
  - does not target `vscode-extension/` or client/UI latency
