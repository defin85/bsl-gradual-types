## 1. Cancellation Model
- [x] 1.1 Зафиксировать ключ supersession (`file_id/profile/generation/version`) и lifecycle in-flight diagnostics задачи.
- [x] 1.2 Ввести cancellation token/checkpoint контракт для heavy diagnostics стадий.

## 2. Scheduler Integration
- [x] 2.1 Обновить diagnostics scheduler: новая ревизия MUST отменять superseded in-flight задачу.
- [x] 2.2 Обеспечить корректное поведение на `didClose` (cancel + cleanup).

## 3. Stage Propagation
- [x] 3.1 Протащить cancellation checkpoints в parse/syntax/semantic heavy stages.
- [x] 3.2 Гарантировать, что superseded задача не доходит до publish.

## 4. Observability
- [x] 4.1 Добавить low-cardinality counters/histograms для superseded cancellation причин.
- [x] 4.2 Добавить drilldown признаков для различения superseded cancel и обычного client cancel.

## 5. Validation
- [x] 5.1 Добавить integration тесты: burst `didChange` отменяет старые heavy diagnostics без stale publish.
- [x] 5.2 Добавить regression тесты на monotonic diagnostics publish после cancel.
- [x] 5.3 Выполнить `openspec validate add-cancellable-diagnostics-supersession --strict --no-interactive`.
