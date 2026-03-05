# Change: Добавить per-request timeline completion в панели расширения

## Why
Сейчас пользователь видит только агрегированные observability-метрики (p50/p95/p99), но не видит профиль конкретного completion-запроса.

Из-за этого root-cause диагностика "почему именно этот completion был медленным" требует ручного разбора логов и не даёт наглядного UX, сопоставимого с operation timeline.

Для production-диагностики нужен полноценный user-facing per-request timeline:
- с этапами, длительностями и статусами по конкретному completion;
- с явным выделением dominant (самого тяжёлого) этапа;
- с корректной обработкой cancelled/superseded запросов.

## What Changes
- Добавить versioned server-driven LSP-контракт per-request timeline для completion (`bsl.getCompletionTimeline`) с bounded retention последних операций; контракт доступен клиенту через `workspace/executeCommand` (`command: bsl.getCompletionTimeline`).
- Зафиксировать stage taxonomy и статусную модель timeline (completed/cancelled/failed/skipped) как machine-readable contract.
- Добавить в VS Code extension отдельное timeline-представление в Observability именно как `webview` (WebviewViewProvider) с визуальным сравнением этапов и dominant-stage highlight; tree-based реализация для timeline не используется.
- Зафиксировать fail-closed совместимость с legacy LSP (метод отсутствует): явное сообщение в UI без падения панели.

## Resolved Decisions (2026-03-05)
- Источник данных для UI: только server-driven timeline контракт LSP, без парсинга текстовых логов и без реконструкции per-request timeline из агрегированных метрик.
- Транспорт timeline-контракта в текущей архитектуре: `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
- Контракт timeline фиксируется в версии `v1`.
- Retention политики timeline: count-based bounded ring buffer, default `max_entries=200`.
- Dominant stage вычисляется на стороне LSP и возвращается в payload для консистентного UX между клиентами.
- UI timeline фиксируется как `webview` в контейнере `bslAnalyzer`; `TreeDataProvider` не является допустимой реализацией этой capability.
- Scope change: только completion timeline (без расширения на hover/signatureHelp/diagnostics в этом change).

## Impact
- Affected specs:
  - `bsl-intellisense`
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion_helpers.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_features_c.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/package.json`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/`
  - `vscode-extension/src/extension.ts`
  - `vscode-extension/src/commands/observability.ts`

## Relation To Existing Changes
- Change `rewrite-v2-observability-perf-pipeline` остаётся отдельным большим rewrite-треком.
- Данный change фиксирует конкретный user-facing capability (per-request completion timeline) и может быть реализован в текущей архитектуре без ожидания полного rewrite.

## Non-Goals
- Полная переработка всех observability/perf контрактов.
- Добавление per-request timeline для hover/signatureHelp/diagnostics.
- Вынос timeline в внешнюю телеметрию/облачный backend.
