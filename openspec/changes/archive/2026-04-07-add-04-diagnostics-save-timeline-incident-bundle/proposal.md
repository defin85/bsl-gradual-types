# Change: добавить diagnostics save timeline в incident bundle

## Why

Текущий observability incident bundle остаётся completion-centric. Для `didSave`/diagnostics он экспортирует
только cumulative `observability_metrics`, поэтому оператор видит p95/p99 по процессу, но не может разобрать
конкретный `didSave -> first_publish -> idle_heavy follow-up` цикл как bounded request-centric trace.

После `refactor-03-diagnostics-save-freshness-fastlane` это стало главным blind spot:

- first publish после `didSave` уже bounded и truthfully отделён от apply-lag;
- но bundle по-прежнему не показывает, какой именно `didSave` использовал `save_fastlane`,
  сколько занял first publish, дождался ли follow-up `idle_heavy`, и где именно сидела задержка;
- из одного cumulative snapshot нельзя восстановить это без guesswork.

## What Changes

- Добавить новый authoritative server-side diagnostics save timeline для request-centric `didSave` refresh.
- Экспортировать его в incident bundle отдельным raw attachment и human-readable summary рядом с existing completion timeline.
- Явно различать `save_fastlane` first publish и optional `idle_heavy` follow-up внутри одного save refresh.
- Деградировать fail-closed на старом сервере: bundle MUST помечать diagnostics save timeline как `unsupported`/`unavailable`
  и MUST NOT реконструировать его из cumulative metrics.

## Impact

- Affected specs:
  - `bsl-intellisense`
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/types.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
