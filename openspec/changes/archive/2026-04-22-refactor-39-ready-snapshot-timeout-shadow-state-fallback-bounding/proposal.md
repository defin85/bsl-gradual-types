# Change: bound `ready_snapshot` parse-exec timeouts that still force `didSave` follow-up into `shadow_state`

## Why

The fresh incident bundle captured at `2026-04-17T14:06:03Z` on git `468658b1` shows a distinct
save-follow-up failure class that is no longer explained by diagnostics-only semantic work.

On the representative `conf_big` save path:

- only `1/4` diagnostics-save traces publish the heavy follow-up through `ready_artifacts`;
- `3/4` traces end on `shadow_state`;
- those `shadow_state` traces all report
  `followup_ready_snapshot_wait_probe=timeout`,
  `followup_ready_snapshot_relief_valve_outcome=engaged_timed_out`,
  `followup_ready_snapshot_task_state=in_flight_same_version`;
- the dominant blocked phase is `parse_exec` at `1919-2742 ms`.

This means the next incident-class is no longer "diagnostics-only IR lacks leaf attribution"
(`refactor-38`). It is "a still-current same-version ready-snapshot producer keeps timing out and
the save follow-up still terminates on `shadow_state`."

## What Changes

- Require the `didSave` heavy follow-up path to stop treating
  `timeout -> engaged_timed_out -> shadow_state` as the representative steady-state outcome while a
  same-version exact producer is still current inside bounded `parse_exec`.
- Require a bounded, truthful continuation/fallback contract for that still-current producer so the
  runtime reduces representative `shadow_state` incidence without weakening latest-wins semantics.
- Require refreshed representative incident evidence that compares `ready_artifacts` vs
  `shadow_state` incidence against the `2026-04-17T14:06:03Z` bundle baseline.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `bsl-runtime/src/system/basic_observability/**`
  - representative backend/runtime tests and incident-bundle assets
- Follow-up relationship:
  - builds on `refactor-32-ready-snapshot-shadow-state-lag-reduction`
  - uses the `2026-04-17T14:06:03Z` bundle as the new baseline
  - is intentionally separate from
    `refactor-38-diagnostics-only-semantic-facts-leaf-profiling`
  - does not target `vscode-extension/` or client/UI latency
