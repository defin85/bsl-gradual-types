# Change: restore didChange ready materialization baseline

## Why

`refactor-55-didchange-ready-install-type-index-wait-bounding` made the
post-detached save-followup blocker explicit, but the representative p56 live
report still records:

```text
did_change_materialization_within_baseline=false
did_change_ready_snapshot_materialization_ms p50=40311 p95=40319 count=4
baseline p50=3226 p95=3329
```

That is a separate residual. Refactor-55 proves the later save-cycle canonical
ready install can be classified as an exact type-index blocker, but it does not
fix the pure didChange canonical ready materialization histogram. The current p56
gate still accepts the run because every save-cycle has a contract-approved
ready-install blocker; refactor-56 must remove that fallback for the didChange
baseline itself.

## What Changes

- Add `bsl-intellisense-v2` requirements that pure didChange canonical ready
  snapshot materialization must stay within the checked-in p56 baseline for the
  representative current-revision flow.
- Extend ready-install/type-index wait tracing to non-save-cycle didChange
  canonical installs, not only same-version didSave/save-cycle targets.
- Require materialization metrics and representative reports to distinguish:
  pure didChange canonical install success, didSave-promoted save-cycle work,
  and classified non-success blockers.
- Tighten p56 validation so `did_change_materialization_within_baseline=false`
  fails even when a later save-cycle exact type-index blocker is truthfully
  classified.
- Preserve canonical exact gates for interactive consumers; the fix must not
  publish a canonical ready snapshot as exact until exact type-index readiness is
  proven for that revision.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - type-index precompute scheduling and exact-ready wait/probe state
  - ready-parse-snapshot materialization metric emission and source labels
  - `backend/src/bin/lsp_server/server/core/tests/live_reports/representative_bundle_live.rs`
  - observability report projections and runtime metric normalization if new
    terminal reasons are introduced
- Follow-up relationship:
  - follows `refactor-55-didchange-ready-install-type-index-wait-bounding`;
  - does not reopen refactor-55 save-cycle blocker acceptance;
  - targets the remaining `did_change_materialization_within_baseline=false`
    contract result directly.

## Non-Goals

- Do not satisfy the change by widening the didChange materialization baseline.
- Do not let a save-cycle/didSave blocker make the pure didChange baseline pass.
- Do not weaken canonical exact readiness for completion, hover, definition,
  signatureHelp, type-at-position, or equivalent exact consumers.
- Do not start in VS Code UI, extension dispatch, completion transport, or
  response egress without fresh evidence that those layers contribute to this
  residual.
