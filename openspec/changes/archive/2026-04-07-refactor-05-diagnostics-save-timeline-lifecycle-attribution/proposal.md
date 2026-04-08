# Change: Diagnostics Save Timeline Lifecycle And Attribution

## Why

Live bundle `bsl-observability-incident-2026-04-07T16-47-14Z` показал два незакрытых класса проблем в
`diagnostics_save_timeline`:

- для одного `(requested_version, diagnostics_generation)` может появляться duplicate trace после terminal archive;
- `save_fastlane` first publish всё ещё может скрывать большую очередь до syntax query, хотя request-centric trace уже есть.

Параллельно human-readable bundle summary помечает active save cycles как `unknown`, хотя это уже in-flight state,
а не отсутствие данных.

## What Changes

- Зафиксировать immutability terminal `didSave` cycle: поздние profile completions не должны воскрешать новый trace.
- Добавить bounded fastlane queue-wait attribution в authoritative trace first publish.
- Сделать request summary явным для active cycles: `in_flight`, а не `unknown`.

## Impact

- Affected specs: `bsl-intellisense-v2`, `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleDiagnosticsSave.ts`
  - targeted tests in backend and vscode-extension
