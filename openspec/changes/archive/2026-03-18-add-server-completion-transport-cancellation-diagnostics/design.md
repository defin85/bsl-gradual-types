## Контекст
Текущий dual-view observability уже разделяет authoritative `Server Timeline` и local-only `Client Probe Feed`. После change `update-client-completion-probe-diagnostics` это позволило локализовать следующий класс проблем:
- `Client Probe Feed` показывает, что длинный completion почти целиком уходит в `lsp_roundtrip_ms`;
- быстрые кейсы подтверждают, что extension-side overhead близок к нулю;
- `Server Timeline` при этом видит только handler-side stages (`prepare_stateful`, `wait_exact_type_index`, `query_bundle`, `collect`) и не объясняет transport receive -> handler entry и момент, когда cancellation реально замечается сервером.

Значит следующая blind spot находится на server edge: между JSON-RPC/LSP transport и текущим `CompletionTimelineCapture`, который стартует уже около входа в handler.

## Goals / Non-Goals

### Goals
- Добавить authoritative server-side breakdown для transport receive, handler execution и cancel observed.
- Сделать длинные `ok_empty` и late-cancel completion traces объяснимыми без client/server correlation guesswork.
- Сохранить bounded contract shape и low-cardinality observability.
- Обновить VS Code UI так, чтобы новые server-edge diagnostics были видны пользователю и не ломали legacy payloads.

### Non-Goals
- Не добавлять protocol-level `client_probe_id` или trace-level join с `Client Probe Feed`.
- Не менять ranking, serving, routing или cancellation behavior completion pipeline.
- Не расширять общую observability surface вне completion path.
- Не экспортировать raw transport logs, high-cardinality labels или free-form cancellation reasons.

## Решения

### Decision: authoritative server-edge diagnostics живут в новом bounded `server_edge_details`
Новые поля добавляются только в server-generated completion trace и остаются bounded:
- `transport_received_at_ms`
- `handler_entered_at_ms`
- `response_sent_at_ms`
- optional `cancel_observed_at_ms`
- `transport_to_handler_wait_ms`
- `server_handler_exec_ms`
- optional `cancel_observed_after_handler_enter_ms`

Это решает две задачи:
- не перегружает существующий `prepare_details`, который уже отвечает за split-prepare routing;
- не заставляет UI или tests восстанавливать server-edge фазы по stage-name эвристике.

### Decision: `started_at_ms` сохраняет текущую семантику, а transport edge приходит отдельным объектом
Текущий `started_at_ms` уже используется существующими consumers и tests как trace start около входа в handler.

Менять его семантику на `transport_received_at_ms` было бы более широким breaking change, чем требуется для этой диагностики. Поэтому:
- `started_at_ms` сохраняет текущую семантику;
- server-edge timestamps приходят отдельным optional object;
- derived deltas дают user-facing сигнал без пересчёта старых stages.

### Decision: authoritative payload эволюционирует до `response.version=3`
Новый `server_edge_details` меняет public server-generated payload shape, поэтому change оформляется как versioned contract evolution:
- новый surface: `contracts/lsp-completion-timeline/v5`;
- response envelope: `version=3`;
- migration note обязателен для tooling/dashboard consumers.

При этом extension должен поддерживать dual-read:
- `version=2` без `server_edge_details`;
- `version=3` с новыми diagnostics.

### Decision: cancellation diagnostics фиксируют `cancel observed`, а не выдумывают obsolete timestamp
Сервер authoritative только в моменте, когда cancellation реально замечена на backend path.

Без protocol-level correlation сервер не знает точный client-side obsolete timestamp. Поэтому change сознательно НЕ вводит поле вроде `cancel_after_obsolete_ms`.

Вместо этого server trace фиксирует только то, что backend действительно знает:
- optional `cancel_observed_at_ms`;
- optional `cancel_observed_after_handler_enter_ms`;
- bounded completion-specific counters/histograms для late cancel analysis.

### Decision: transport/cancellation metrics остаются completion-specific и bounded
Новые observability signals ограничиваются completion path и low-cardinality vocabulary:
- latency samples для `transport_to_handler_wait`;
- latency samples для `server_handler_exec`;
- counters/histograms для `cancel observed`.

Никаких URI, request payload fragments или свободных reason labels в metric keys не добавляется.

## Альтернативы

### Option A: расширить только `Client Probe Feed`
Плюсы:
- нулевой backend contract churn.

Минусы:
- client probes уже показывают, что проблема после dispatch, но не могут авторитетно разделить queue-before-handler и server handler exec;
- не отвечают на вопрос, когда cancellation реально увидел backend.

Отклонено.

### Option B: попытаться вывести server-edge breakdown из stage names
Плюсы:
- не нужен новый trace object.

Минусы:
- стадийная модель сегодня начинается после handler entry;
- появятся хрупкие эвристики в UI/tests;
- трудно отделить transport receive и cancel observed от обычных handler stages.

Отклонено.

### Option C: сразу добавить exact client/server correlation
Плюсы:
- можно было бы измерять cancel-obsolete path end-to-end.

Минусы:
- это уже cross-stack correlation change, а не узкая server-edge instrumentation задача;
- scope резко вырастает.

Отклонено для этого change.

## Риски / Trade-offs
- Response `version=3` потребует обновить consumers/tests и добавить migration note для contract surface.
- `cancel_observed_at_ms` останется best-effort signal: он фиксирует момент первого backend observation, а не абсолютную истину о том, когда запрос стал obsolete на клиенте.
- Capture points должны использовать согласованный clock source, иначе derived deltas станут недостоверными.
- Instrumentation не должна добавлять blocking work в hot path; если server-edge capture ломается, completion response должен остаться fail-open для пользователя.

## Validation
- Backend tests MUST проверять `response.version=3`, `server_edge_details`, bounded transport/cancellation fields и отсутствие contract drift для старых обязательных полей.
- Extension tests MUST проверять:
  - rendering/export новых server-edge diagnostics;
  - backward-compatible rendering payload `version=2` без `server_edge_details`.
- Контрактный слой MUST включать новый versioned directory `contracts/lsp-completion-timeline/v5` и migration note.
- `openspec validate add-server-completion-transport-cancellation-diagnostics --strict --no-interactive`.
