# Change: bound current-context and ready-install contention

## Why

The new incident bundle
`/home/egor/code/temp/bsl-observability-incident-2026-04-26T21-01-14Z`
was captured from runtime git `0f3a07de` after
`refactor-57-runtime-saturation-contract-completeness`. It no longer shows
runtime saturation contract violations:
`intellisense_v2_observability_contract_violation_total=0`, and invalid
saturation metrics are absent.

The remaining evidence is a different backend contention class, not a VS Code
UI/pre-send or completion-path regression. Completion requests stay small, while
same-file `didSave` follow-up and concurrent `bsl.getCurrentContext` work expose
seconds-scale readiness/install waits: one save follow-up spends `2440ms`
overall with `ready_install=2193ms`, `snapshot_with_deps=1949ms`, and only
`84ms` of `parse_exec`; `bsl.getCurrentContext` contenders age up to `3485ms`.

## What Changes

- Add a `bsl-intellisense-v2` requirement that current-context and didSave
  ready-install contention is bounded, latest-only where applicable, and
  attributable in incident bundles.
- Require per-request `bsl.getCurrentContext` evidence in the bundle instead of
  relying only on cumulative metrics or completion contender ages.
- Split opaque didSave follow-up `ready_install` / `snapshot_with_deps` /
  `wait_for_file_version` residuals into explicit bounded attribution buckets.
- Preserve the `refactor-57` saturation integrity result: representative
  validation MUST keep observability contract violations absent or zero.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - VS Code/backend incident-bundle projection code if a new
    current-context request section is needed

## Non-Goals

- Do not reopen `refactor-57-runtime-saturation-contract-completeness`; the new
  bundle proves the saturation contract violation is gone for this run.
- Do not reopen
  `refactor-50-didsave-waiting-phase-shadow-state-bounding`; this bundle does
  not show terminal `shadow_state` fallback on the affected save traces.
- Do not treat VS Code UI rendering or extension pre-send work as the primary
  suspect unless a newer bundle shows materially elevated
  `client_before_transport_write_wait_ms`.
- Do not solve this by widening readiness budgets without adding attribution
  and bounded stale-work behavior.
