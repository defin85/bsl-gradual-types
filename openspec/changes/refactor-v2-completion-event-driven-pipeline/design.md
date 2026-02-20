## Context

Существующий completion pipeline уже содержит важные строительные блоки:
- centralized facade (`prepare_stateful_operation`);
- stale fallback и bounded wait budget;
- singleflight для дорогих query;
- CPU fairness между interactive/background.

Но orchestration completion в LSP всё ещё процедурный: hot path остаётся длинной цепочкой adapter-local шагов. Это усложняет детерминизм под burst `didChange`/completion и не дает явного контракта отмены/коалесцирования.

Observed baseline (2026-02-20, conf_big, warm run):
- completion duration p95 = 3685ms
- completion snapshot p95 = 3623ms
- completion wait-for-version p95 = 121ms
- background queue-wait p95 = 2788ms (p99 = 3303ms)
- syntax diagnostics p95 = 2858ms
- semantic diagnostics p95 = 584ms
- interactive wait budget exhausted = 2 events

## Target Architecture

Целевой design для этого change:
- per-file event-driven orchestrator actor на `file_id`;
- bounded `tokio::mpsc` queue с latest-wins/coalescing;
- явные события `DidOpen/DidChange/CompletionRequest/Cancel/DidClose`;
- cancellation propagation по stage-checkpoints;
- dual-mode rollout (`off|shadow|canary|on`) и kill-switch rollback.

## Architecture Lock

В этом change implementation target фиксируется однозначно как описанная выше целевая архитектура.
Отклонение от нее не входит в implementation scope.

## Goals / Non-Goals

- Goals:
  - Перевести completion hot path на per-file actor orchestration.
  - Формально закрепить deterministic ordering и latest-wins.
  - Встроить явную cancellation propagation по стадиям.
  - Обеспечить dual-mode rollout и безопасный kill-switch rollback.
  - Снизить интерактивный tail latency до rollout SLO.
- Non-Goals:
  - Изменение семантики completion candidates.
  - Изменение ranking/score модели.
  - Автоматическое изменение editor settings пользователя.

## Decisions

### Decision 1: Event envelope contract

Каждое событие перед постановкой в per-file queue MUST оборачиваться в envelope:
- `file_id`;
- `file_seq` (monotonic, per-file);
- `received_at`;
- `payload`.

Для `CompletionRequest` payload дополнительно MUST включать:
- `request_id` (LSP request id);
- `request_epoch` (monotonic, per-file);
- `version_hint`;
- `trigger_mode`.

Инварианты:
- внутри одного `file_id` события обрабатываются строго FIFO по `file_seq`;
- completion response publish допустим только для актуального `request_epoch`;
- `request_epoch` увеличивается только на `CompletionRequest`.

### Decision 2: Dispatcher topology и lifecycle

На границе LSP/runtime вводится per-file dispatcher:
- key: `file_id`;
- execution unit: actor task;
- inbox: bounded `tokio::mpsc`;
- lifecycle:
  - create on first `DidOpen/DidChange/CompletionRequest`,
  - retain while file активен,
  - drain and shutdown on `DidClose`.

Это устраняет глобальную конкуренцию между файлами и позволяет локально применять backpressure/coalescing.

### Decision 3: Bounded queue и overflow policy

Queue capacity MUST быть конфигурируемой (`BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY`) с безопасным clamp и default.

Overflow policy MUST быть детерминированной:
- `DidChange`: коалесцируется до latest revision (хранить только самую новую pending правку).
- `CompletionRequest`: устаревшие pending completion для меньшего `request_epoch` вытесняются.
- `Cancel`: не должен теряться; при saturation допускается вытеснение oldest non-cancel события.

Ни один режим работы не должен приводить к неограниченному росту per-file backlog.

### Decision 4: Latest-wins scheduling

Scheduler обязан:
- назначать интерактивный бюджет только latest `request_epoch`;
- отменять/останавливать superseded задачи до тяжёлых стадий;
- блокировать публикацию late response для не-latest epoch.

### Decision 5: Cancellation propagation contract

`$/cancelRequest` MUST маппиться в событие `Cancel(request_id)` через request-level registry `request_id -> (file_id, request_epoch, token)`.

Отмена MUST проверяться на checkpoint-этапах:
- `wait_for_file_version`;
- `snapshot_with_deps`;
- `ir_query`;
- `collect`;
- `rank`;
- `format`;
- `publish`.

Если запрос отменён или superseded, поздний user-facing completion publish MUST NOT происходить.

### Decision 6: Fallback policy централизуется в orchestrator/runtime

Stale/degraded policy (`isIncomplete=true`, `fallback_unavailable`, terminal-empty guard) остается единой runtime policy и не дублируется adapter-local ветками.

### Decision 7: Dual-mode rollout state machine

Вводятся runtime keys:
- `BSL_INTELLISENSE_V2_COMPLETION_MODE` (`off|shadow|canary|on`);
- `BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT` (`0..100`).

Семантика:
- `off`: legacy path only;
- `shadow`: legacy response + event-driven execution для telemetry/parity;
- `canary`: детерминированная доля трафика в event-driven response path;
- `on`: event-driven default.

Canary routing MUST быть детерминированным (stable hash по request identity), чтобы результаты можно было воспроизводимо сравнивать.

Rollback: моментальное переключение mode назад в `off`.

### Decision 8: Observability contract для сравнения режимов

Для completion-контура добавляется mode-aware low-cardinality измерение:
- `mode=legacy|event_driven|shadow`.

Mode MUST присутствовать в drilldown событиях completion-контура минимум для стадий:
- `runtime_wait_for_file_version`;
- `runtime_snapshot_with_deps`;
- `ir_query`;
- `parse_result_query`.

Legacy fixed-key метрики MUST оставаться совместимыми через deterministic projection из canonical event model.

### Decision 9: Ownership boundaries

- `LSP adapter`: transport parsing, request registry, mode routing, event ingest.
- `Per-file orchestrator`: ordering, coalescing, cancellation/supersede, publish guard.
- `Runtime facade`: snapshot/query execution и shared policies.
- `Observability`: canonical events + drilldown/legacy dual-write projection.

## Architecture Sketch

1. `LSP Adapter` преобразует transport-сигналы в канонические события и отправляет их в `PerFileDispatcher`.
2. `PerFileOrchestrator` применяет ordering, latest-wins, cancellation, coalescing.
3. `Runtime Query Executor` выполняет bounded стадии (singleflight/fairness сохраняются).
4. `Response Assembler` формирует итоговый LSP response и mode-aware observability.

## Migration Plan

1. Добавить runtime mode/canary keys + dual-path wiring (без смены default поведения).
2. Ввести per-file dispatcher/actor с bounded queue и deterministic overflow policy.
3. Подключить request-level cancellation registry и propagation до stage checkpoints.
4. Подключить `shadow` режим: event-driven исполняется, ответ пользователю остается legacy.
5. Добавить mode-aware метрики и parity сравнение.
6. Включить `canary`, затем `on` только при прохождении quality gates.
7. Держать kill-switch rollback до завершения стабилизации.

## Rollout Quality Gates

Переход `shadow -> canary -> on` разрешается только при выполнении SLO:
- `completion_duration_ms` p95 <= 1500ms;
- `wait_for_file_version_completion_ms` p95 <= wait_budget + 20ms;
- `runtime_queue_wait_interactive_ms` p95 <= wait_budget + 250ms;
- `completion_cancelled_rate <= 0.10`;
- `completion_parity_drift_rate <= 0.01`;
- `member_access_terminal_empty_missing_ir_rate <= 0.005`.

## Risks / Trade-offs

- Рост архитектурной сложности.
  - Митигация: четкие границы ownership и контрактные тесты.
- Ошибки в latest-wins/coalescing могут вызвать starvation.
  - Митигация: bounded queue, saturation метрики, overflow tests.
- Временная деградация latency в dual-mode.
  - Митигация: shadow-first rollout, жесткие SLO gates, быстрый rollback.

## Open Questions

- Нужно ли на первом этапе включать `hover/signatureHelp` в тот же per-file orchestrator или ограничиться completion-only rollout.
