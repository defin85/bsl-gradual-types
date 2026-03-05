## 1. Specification
- [x] 1.1 Добавить delta в `bsl-intellisense` для user-facing completion timeline панели в Observability.
- [x] 1.2 Добавить delta в `bsl-intellisense-v2` для versioned per-request timeline контракта LSP.
- [x] 1.3 Зафиксировать fail-closed совместимость extension с legacy LSP без `bsl.getCompletionTimeline`.

## 2. Backend (LSP) Timeline Contract
- [x] 2.1 Добавить contract types `CompletionTimelineRequest/Response` и DTO для trace/stage (v1) в LSP слой.
- [x] 2.2 Реализовать bounded ring-buffer retention (`max_entries=200` default) для completion trace history.
- [x] 2.3 Инструментировать completion pipeline для записи stage timeline с terminal outcome (`ok_non_empty|ok_empty|cancelled|superseded|handler_error|fallback_*`).
- [x] 2.4 Реализовать server-driven request `bsl.getCompletionTimeline` через `workspace/executeCommand` (latest list + optional lookup/filter по `request_id`) как единственный источник timeline для extension.
- [x] 2.5 Добавить dominant-stage вычисление (max duration среди terminal stage entries) и сериализацию в response.
- [x] 2.6 Добавить backend тесты:
- [x] 2.6.1 Контракт response (`version`, `traces`, `stages`, outcome/status enums).
- [x] 2.6.2 Cancelled/superseded запрос возвращает partial timeline с корректным terminal status.
- [x] 2.6.3 Retention deterministic eviction oldest-first при переполнении.

## 3. VS Code Extension Timeline Panel
- [x] 3.1 Добавить LSP request client `getCompletionTimeline` с вызовом через `workspace/executeCommand` (`command: bsl.getCompletionTimeline`).
- [x] 3.2 Добавить timeline view в container `bslAnalyzer` как `webview` (`WebviewViewProvider`) с визуальными барами этапов; `TreeDataProvider` не использовать для timeline.
- [x] 3.3 Реализовать dominant-stage highlight и отображение total duration/outcome/request metadata.
- [x] 3.4 Добавить автообновление панели (асинхронное, bounded polling) только когда view активна.
- [x] 3.5 Добавить graceful degradation для legacy LSP (`Method not found`): явный UX-статус, без падения view.
- [x] 3.6 Добавить extension tests:
- [x] 3.6.1 Mapping LSP timeline payload -> UI model.
- [x] 3.6.2 Выделение dominant stage.
- [x] 3.6.3 Legacy unsupported path.

## 4. Validation
- [x] 4.1 Выполнить минимальный релевантный набор тестов backend + extension для timeline контракта и UI.
- [x] 4.2 Выполнить `openspec validate add-completion-per-request-timeline-panel --strict --no-interactive`.
