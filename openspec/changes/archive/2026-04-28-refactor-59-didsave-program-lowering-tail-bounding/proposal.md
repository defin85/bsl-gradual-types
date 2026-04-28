# Change: bound didSave program-lowering tail after ready-install contention

## Why

The fresh observability bundle
`/home/egor/code/temp/bsl-observability-incident-2026-04-27T08-39-19Z`
was captured from runtime git `033ac549`, after
`refactor-58-current-context-ready-install-contention` was published. It proves
the previous class moved:

- observability integrity is clean:
  `intellisense_v2_observability_contract_violation_total=0`, invalid
  saturation metrics absent;
- completion remains healthy: six completion traces, `fail_closed=0`,
  `service_future_to_first_poll_wait_ms` max `0ms`, output handoff max `5ms`;
- the new first-class current-context timeline is present: 12 traces, 5
  resolved, 7 superseded, 3 `ready_snapshot` results at `125-126ms`;
- compared to `refactor-08`, `didSave` first publish is now `51-65ms` instead
  of `102ms`, and full follow-up publishes at `1360ms` / `4346ms` instead of
  waiting `39097ms` for file version with no full publish;
- compared to the `2026-04-26T21-01-14Z` bundle, the v15 `ready_install`
  residual is effectively gone (`2193ms -> 1ms`).

The remaining residual is narrower: `didSave` v15 first publish is fast
(`51ms`), `ready_install=1ms`, `snapshot_with_deps_ms=47ms`, semantic
diagnostics is `796ms`, but full follow-up still takes `4346ms` because exact
ready-snapshot production spends `3598ms` in `parse_exec`, dominated by
`exact_ready_snapshot_assembly -> program_lowering=3596ms`.

The incident should therefore not reopen VS Code UI/pre-send, completion
transport, runtime saturation integrity, current-context contention, or generic
ready-install wait. The next change should bind the save-critical
`program_lowering` tail and make missing lowering-reuse evidence fail-visible.

## What Changes

- Add a `bsl-intellisense-v2` requirement that post-refactor-58 same-version
  `didSave` heavy follow-up MUST bound the exact
  `parse_exec -> exact_ready_snapshot_assembly -> program_lowering` tail when
  first publish, ready-install, snapshot-with-deps, completion, and current
  context are no longer the primary blockers.
- Require representative evidence to reject a generic
  `followup_readiness_blocker_bucket=snapshot_with_deps` explanation when the
  measured `snapshot_with_deps_ms` is small and the dominant residual is
  seconds-scale program lowering inside exact assembly.
- Preserve or restore program-lowering reuse-plan evidence end-to-end through
  diagnostics-save timelines, the VS Code custom request type, incident-bundle
  raw projection, and human summary when program lowering dominates the save
  follow-up tail.
- Add a representative gate that fails if a clean post-refactor-58 bundle still
  has a seconds-scale same-version `program_lowering` tail without a truthful
  supersession, cancellation, failure, continuity-loss, or required-full-rebuild
  reason.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - ready-snapshot exact assembly / program-lowering reuse surfaces
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts`
  - diagnostics-save timeline and incident-bundle projection tests
  - representative `conf_big` didSave follow-up live/perf reports
  - targeted diagnostics-save timeline regressions around program-lowering
    reuse evidence and post-refactor-58 blocker classification

## Non-Goals

- Do not reopen `refactor-58`; its ready-install/current-context attribution
  work is present in the new bundle and remains valuable.
- Do not reopen `refactor-57`; saturation integrity is clean in the new bundle.
- Do not treat VS Code UI rendering, extension pre-send, completion ingress, or
  completion output handoff as the primary suspect for this bundle.
- Do not satisfy this change by widening bounded wait or relief-valve budgets.
- Do not weaken exact same-version semantics or canonical interactive exact
  readiness for completion, hover, definition, signatureHelp, type-at-position,
  or equivalent interactive consumers.
