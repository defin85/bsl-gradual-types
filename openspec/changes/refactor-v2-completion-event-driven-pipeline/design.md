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

## Architecture Options

### Option 1: Укрепить текущий adapter-local pipeline (не выбран)
- Плюс: минимальные изменения в коде.
- Минус: масштабирование сложности в `language_server.rs`, слабый контракт latest-wins/cancel.

### Option 2: Per-file event-driven orchestrator actor (выбран)
- Для каждого `file_id` отдельный dispatcher/actor и bounded queue.
- Явные события `DidOpen/DidChange/CompletionRequest/Cancel/DidClose`.
- Latest-wins и cancellation применяются на уровне scheduler до тяжелых стадий.
- Dual-mode rollout (`legacy` + `event-driven`) с shadow-режимом.

### Option 3: Единый глобальный orchestrator на весь workspace (не выбран)
- Плюс: один центр принятия решений.
- Минус: риск cross-file head-of-line blocking и сложнее локальная изоляция нагрузки.

## Goals / Non-Goals

- Goals:
  - Перевести completion hot path на вариант 2 (per-file actor orchestration).
  - Формально закрепить deterministic ordering и latest-wins.
  - Встроить явную cancellation propagation по стадиям.
  - Обеспечить dual-mode rollout и безопасный kill-switch rollback.
  - Снизить интерактивный tail latency до rollout SLO.
- Non-Goals:
  - Изменение семантики completion candidates.
  - Изменение ranking/score модели.
  - Автоматическое изменение editor settings пользователя.

## Decisions

### Decision 1: Dispatcher topology

На границе LSP/runtime вводится per-file dispatcher:
- key: `file_id`;
- execution unit: actor task;
- inbox: bounded `mpsc` queue (размер конфигурируемый, фиксированный верхний предел).

Это устраняет глобальную конкуренцию между файлами и позволяет локально применять backpressure/coalescing.

### Decision 2: Event contract и инварианты порядка

События:
- `DidOpen(version, text)`;
- `DidChange(version, diff_or_full_text)`;
- `CompletionRequest(request_id, position, trigger_mode, revision_hint)`;
- `Cancel(request_id)`;
- `DidClose`.

Инварианты:
- внутри одного `file_id` события обрабатываются строго FIFO;
- каждый completion request получает monotonic `request_epoch`;
- результат может быть опубликован только если `request_epoch == latest_epoch`.

### Decision 3: Latest-wins + coalescing policy

Scheduler обязан:
- коалесцировать устаревшие completion задачи;
- отменять задачи, потерявшие актуальность по `version/epoch`, до тяжелых стадий;
- гарантировать, что интерактивный бюджет тратится только на latest-запрос.

### Decision 4: Cancellation propagation contract

`Cancel(request_id)` MUST:
- привязываться к активному `CompletionRequest` через registry `request_id -> token`;
- прекращать дальнейшее выполнение между stage-checkpoints (`wait`, `snapshot`, `ir`, `collect`, `rank`, `format`);
- завершать запрос согласно LSP cancellation semantics без подвисания.

### Decision 5: Fallback policy централизуется в orchestrator/runtime

Stale/degraded policy (`isIncomplete=true`, `fallback_unavailable`, terminal-empty guard) остается единой runtime policy и не дублируется adapter-local ветками.

### Decision 6: Dual-mode rollout через runtime key

Вводится feature flag mode (runtime-config) с фиксированными значениями:
- `off` (legacy только),
- `shadow` (event-driven исполняется параллельно для метрик/сравнения),
- `canary` (частичный ответ event-driven),
- `on` (event-driven default path).

Rollback: моментальное переключение mode назад в `off`.

### Decision 7: Observability contract для сравнения режимов

Для completion-контура добавляется mode-aware измерение (`mode=legacy|event_driven|shadow`) с низкой кардинальностью, включая operation-scoped stages:
- `runtime_wait_for_file_version`,
- `runtime_snapshot_with_deps`,
- `ir_query`,
- `parse_result_query`.

Это дает формальное сравнение legacy vs event-driven без изменения семантики existing keys.

### Decision 8: Rollout quality gates

Переход `shadow -> canary -> on` разрешается только при выполнении SLO:
- `completion_duration_ms` p95 <= 1500ms;
- `wait_for_file_version_completion_ms` p95 <= wait_budget + 20ms;
- `runtime_queue_wait_interactive_ms` p95 <= wait_budget + 250ms;
- `interactive_wait_budget_exhausted / completion_total <= 1%`;
- `completion_result_total_ok_empty / completion_total <= 5%` на фиксированном smoke-профиле.

## Architecture Sketch

1. `LSP Adapter` преобразует transport-сигналы в канонические события и отправляет их в `PerFileDispatcher`.
2. `PerFileOrchestrator` применяет ordering, latest-wins, cancellation, coalescing.
3. `Runtime Query Executor` выполняет bounded стадии (singleflight/fairness сохраняются).
4. `Response Assembler` формирует итоговый LSP response и mode-aware observability.

## Migration Plan

1. Добавить runtime mode key + dual-path wiring (без смены default поведения).
2. Ввести per-file dispatcher/actor с bounded queue, пока без user-facing switch.
3. Подключить `shadow` режим: event-driven исполняется, ответ пользователю остается legacy.
4. Добавить сравнение mode-aware метрик и parity checks.
5. Включить `canary`, затем `on` только при прохождении quality gates.
6. Держать kill-switch rollback до завершения стабилизации.

## Risks / Trade-offs

- Рост архитектурной сложности.
  - Митигация: четкие границы ownership (`adapter`, `orchestrator`, `runtime`), контрактные тесты.
- Ошибки в latest-wins/coalescing могут вызвать starvation.
  - Митигация: bounded queue, fairness guards, saturation метрики.
- Временная деградация latency в dual-mode.
  - Митигация: shadow-first rollout, жесткие SLO gates, быстрый rollback.

## Open Questions

- Ограничивать первый этап только `completion` или сразу включать `hover/signatureHelp` в тот же orchestrator.
- Нужны ли отдельные per-file лимиты backpressure для очень больших модулей (по размеру/частоте событий).
