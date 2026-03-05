## 1. Specification
- [ ] 1.1 Добавить delta в `bsl-intellisense` для user-facing completion timeline панели в Observability.
- [ ] 1.2 Добавить delta в `bsl-intellisense-v2` для versioned per-request timeline контракта LSP.
- [ ] 1.3 Зафиксировать fail-closed совместимость extension с legacy LSP без `bsl.getCompletionTimeline`.

## 2. Backend (LSP) Timeline Contract
- [ ] 2.1 Добавить contract types `CompletionTimelineRequest/Response` и DTO для trace/stage (v1) в LSP слой.
- [ ] 2.2 Реализовать bounded ring-buffer retention (`max_entries=200` default) для completion trace history.
- [ ] 2.3 Инструментировать completion pipeline для записи stage timeline с terminal outcome (`ok_non_empty|ok_empty|cancelled|superseded|handler_error|fallback_*`).
- [ ] 2.4 Реализовать server-driven request `bsl.getCompletionTimeline` через `workspace/executeCommand` (latest list + optional lookup/filter по `request_id`) как единственный источник timeline для extension.
- [ ] 2.5 Добавить dominant-stage вычисление (max duration среди terminal stage entries) и сериализацию в response.
- [ ] 2.6 Добавить backend тесты:
- [ ] 2.6.1 Контракт response (`version`, `traces`, `stages`, outcome/status enums).
- [ ] 2.6.2 Cancelled/superseded запрос возвращает partial timeline с корректным terminal status.
- [ ] 2.6.3 Retention deterministic eviction oldest-first при переполнении.

## 3. VS Code Extension Timeline Panel
- [ ] 3.1 Добавить LSP request client `getCompletionTimeline` с вызовом через `workspace/executeCommand` (`command: bsl.getCompletionTimeline`).
- [ ] 3.2 Добавить timeline view в container `bslAnalyzer` как `webview` (`WebviewViewProvider`) с визуальными барами этапов; `TreeDataProvider` не использовать для timeline.
- [ ] 3.3 Реализовать dominant-stage highlight и отображение total duration/outcome/request metadata.
- [ ] 3.4 Добавить автообновление панели (асинхронное, bounded polling) только когда view активна.
- [ ] 3.5 Добавить graceful degradation для legacy LSP (`Method not found`): явный UX-статус, без падения view.
- [ ] 3.6 Добавить extension tests:
- [ ] 3.6.1 Mapping LSP timeline payload -> UI model.
- [ ] 3.6.2 Выделение dominant stage.
- [ ] 3.6.3 Legacy unsupported path.

## 4. Validation
- [ ] 4.1 Выполнить минимальный релевантный набор тестов backend + extension для timeline контракта и UI.
- [ ] 4.2 Выполнить `openspec validate add-completion-per-request-timeline-panel --strict --no-interactive`.
