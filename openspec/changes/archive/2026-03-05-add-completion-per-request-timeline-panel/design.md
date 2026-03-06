## Context
В текущем состоянии:
- backend уже собирает богатые stage-level completion latency метрики;
- extension уже умеет читать агрегированный observability snapshot (`bsl.getObservabilityMetrics`);
- Observability UI в extension реализован как tree view и не предоставляет per-request timeline.

Это оставляет gap: пользователь не видит "операционную дорожку" конкретного completion-запроса и не может быстро определить самый тяжёлый этап для единичного slow case.

## Goals / Non-Goals
- Goals:
  - Дать пользователю наглядный per-request timeline completion в extension.
  - Сохранить machine-readable, versioned, bounded контракт для timeline данных.
  - Не ухудшить completion latency и не добавить тяжёлую синхронную работу в request path.
- Non-Goals:
  - Редизайн всего observability/perf pipeline.
  - Timeline для non-completion операций.
  - Внешняя централизованная телеметрия.

## Architecture Drivers
- Debuggability: root-cause slow completion должен быть виден в одном UI.
- Low overhead: запись timeline должна быть bounded и лёгкой.
- Determinism: стабильная stage taxonomy и response schema.
- Compatibility: extension должен деградировать предсказуемо на legacy LSP.

## Options Considered

### Option A: Строить pseudo-timeline из агрегированных p95/p99 метрик
- Плюсы: минимальные backend изменения.
- Минусы: не per-request, не показывает конкретный slow incident, невозможна корректная визуализация cancelled/superseded операции.

### Option B: Парсить текстовые логи LSP в extension
- Плюсы: можно быстро прототипировать.
- Минусы: хрупко, без контракта, зависит от log formatting, плохо тестируется, ломает deterministic UX.

### Option C (Recommended): Server-side per-request trace buffer + `workspace/executeCommand` request + webview UI
- Идея:
  - LSP формирует per-request completion traces в bounded ring buffer.
  - Extension запрашивает traces через `workspace/executeCommand` с `command: bsl.getCompletionTimeline`.
  - Webview рисует timeline и выделяет dominant stage.
- Плюсы:
  - точные per-request данные;
  - deterministic contract;
  - UX как operation timeline.
- Минусы:
  - нужны изменения в backend и extension одновременно.

## Decisions
- Decision 1: выбрать Option C.
  - Why: только этот вариант обеспечивает корректный per-request UX и формальный контракт.

- Decision 2: контракт `bsl.getCompletionTimeline` фиксируется в `v1`.
  - Контракт в extension MUST вызываться через `workspace/executeCommand` (`command: bsl.getCompletionTimeline`) и является единственным источником per-request timeline данных.
  - Response v1:
    - `version: 1`;
    - `traces: CompletionTrace[]`.
  - `CompletionTrace` включает:
    - `trace_id`, `request_id`, `uri`, `trigger_mode`;
    - `outcome`, `started_at_ms`, `total_duration_ms`;
    - `dominant_stage`;
    - `stages: CompletionStageTrace[]`.
  - `CompletionStageTrace` включает:
    - `name`;
    - `status` (`completed|cancelled|failed|skipped`);
    - `started_offset_ms`;
    - `duration_ms`.

- Decision 3: retention count-based и bounded.
  - Default `max_entries=200`, oldest-first eviction.
  - Retention применяется на trace уровне, не на stage уровне.

- Decision 4: stage taxonomy bounded.
  - Stage names MUST использовать bounded словарь, совместимый с completion stage observability taxonomy.
  - High-cardinality labels (пути, динамические фрагменты текста) в stage names запрещены.

- Decision 5: dominant-stage вычисляется в backend.
  - Единый алгоритм для всех клиентов: максимальный `duration_ms` среди terminal stage entries.

- Decision 6: UI — только webview внутри существующего `bslAnalyzer` контейнера.
  - Timeline capability MUST быть реализован через VS Code `WebviewViewProvider`; tree-based (`TreeDataProvider`) вариант не используется для этой capability.

- Decision 7: legacy compatibility fail-closed.
  - Если `bsl.getCompletionTimeline` недоступен, extension показывает явное сообщение "unsupported by server" и не падает.

## Data Flow (Target)
1. Completion request enters `lsp_completion`.
2. Trace collector создаёт новый trace и фиксирует старт.
3. На ключевых checkpoints/стадиях пишутся stage entries.
4. При terminal outcome trace финализируется и кладётся в ring buffer.
5. Extension запрашивает timeline только через `workspace/executeCommand` с `command: bsl.getCompletionTimeline` (latest N или lookup по `request_id`) и не реконструирует timeline из логов/агрегатов.
6. Webview отображает timeline, dominant stage и outcome.

## Failure/Edge Cases
- Cancelled/superseded completion:
  - trace фиксируется как terminal (`cancelled`/`superseded`), с частичным набором stages.
- Internal instrumentation error:
  - completion response пользователю не блокируется и не меняет семантику.
- Empty traces:
  - UI показывает "no recent completion traces" без ошибки.

## Test Strategy
- Backend:
  - unit/integration tests на contract serialization и retention eviction;
  - tests на cancelled/superseded terminal traces;
  - tests на dominant-stage computation.
- Extension:
  - unit tests на payload-to-view-model mapping;
  - tests на dominant-stage highlight;
  - tests на unsupported-method fallback.
- End-to-end smoke:
  - видимость нового trace после completion;
  - корректный dominant stage для искусственно замедленной стадии.

## Risks / Trade-offs
- Риск overhead от trace capture в hot path.
  - Mitigation: bounded in-memory trace model, без тяжёлой сериализации на request path.
- Риск drift между observability stage names и timeline stage names.
  - Mitigation: единый bounded taxonomy + контрактные тесты.
- Риск UI шума при высокой частоте completion.
  - Mitigation: ограничение количества отображаемых traces и ручной/периодический refresh.
