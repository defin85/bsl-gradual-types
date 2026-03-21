## Контекст
После `v5` bottleneck drilldown стало проще понимать, где completion-path ломается в целом, но практический анализ incident bundle всё ещё упирается в три необъяснённых класса задержек:

- большой `transport_to_handler_wait_ms` не отделяет service/dispatch lag от async prelude внутри самого `lsp_completion`;
- `prepare_timeout` показывает только terminal outcome и coarse phase, но не фиксирует timeout-layer и величину overshoot;
- `exact_deadline` на artifact-readiness path не объясняет, ушло ли время на polling snapshot'ов до waiter/task-state path.

Новый change не чинит сами latency-проблемы. Он делает authoritative payload достаточным для того, чтобы следующий debugging step был data-driven, а не опирался на чтение кода и ad-hoc логов.

## Goals / Non-Goals

### Goals
- Отделить ingress/service lag от async prelude внутри completion handler.
- Сделать late timeout wake наблюдаемым через bounded structured fields, а не через ручной расчёт по raw timestamp'ам.
- Сделать artifact-polling deadline различимым от type-index waiter deadline.
- Сохранить единый source of truth: `bsl.getCompletionTimeline`.
- Сохранить `v5` compatibility path для extension и incident handoff.

### Non-Goals
- Не исправлять в этом change фактические latency-проблемы `prepare_timeout`, scheduler starvation или exact precompute readiness.
- Не добавлять unbounded log/NDJSON pipeline.
- Не вводить новый observability API поверх timeline.
- Не делать новый UI surface; используются уже существующие Completion Timeline и incident handoff flows.

## Решения

### 1. Авторитетной поверхностью остаётся `bsl.getCompletionTimeline`
Новый root-cause attribution остаётся частью versioned per-request completion timeline. Это позволяет:

- коррелировать новые поля с конкретным completion request;
- использовать уже существующий extension transport и incident bundle;
- избежать semantic drift между timeline, raw logs и cumulative metrics.

Следствие: change bump'ает contract version до `6`, а extension обязан явно деградировать на `v5`.

### 2. Server-edge attribution должен отделять method entry от handler prelude
Текущий `handler_entered_at_ms` ставится слишком поздно для анализа ingress lag. Поэтому `server_edge_details` должен дополнительно выражать:

- момент первого входа в `lsp_completion` до async prelude;
- bounded split между `transport_received -> method_entered` и `method_entered -> handler_entered`.

Минимально полезная модель:
- `method_entered_at_ms`;
- `transport_to_method_wait_ms`;
- `method_prelude_exec_ms`.

Эти поля additive и не заменяют существующие `handler_entered_at_ms` и `transport_to_handler_wait_ms`.

### 3. Prepare timeout должен фиксировать timeout-layer и overshoot
Сейчас по payload видно только budget и terminal elapsed, но не ясно, какой timeout-layer завершился поздно:

- outer `prepare_guard` в completion handler;
- inner interactive wait budget на `wait_for_file_version`.

Нужен bounded `timeout_attribution` внутри `prepare_details`, который при timeout-path'е фиксирует:

- `source` из fixed vocabulary `prepare_guard|interactive_wait_budget`;
- `phase` из bounded prepare phase vocabulary;
- `budget_ms`;
- `elapsed_ms`;
- `overshoot_ms`.

Важно: runtime reply details не должны выдумываться. Если reply к timeout-моменту не наблюдался, runtime split остаётся `undefined`, а timeout attribution всё равно даёт полезную диагностику.

### 4. Exact artifact wait должен показывать polling evidence
Текущий `exact_wait` уже может показывать outcome waiter/task-state path, но не покрывает случай, когда deadline тратится на artifact polling до этого пути.

Нужен bounded `artifact_poll` block, который фиксирует:

- `poll_count`;
- `poll_elapsed_ms`;
- terminal readiness flags (`head_ready`, `exact_ready`);
- optional `observed_file_version` на последнем polling snapshot.

Этого достаточно, чтобы понять:
- успел ли path вообще дойти до type-index waiter;
- не уходит ли budget на repeated `snapshot_with_deps()` polling.

### 5. Existing extension surfaces должны переносить новые fact lines без нового UI
Change не создаёт новый UI. Вместо этого existing surfaces должны:

- принимать `v6` payload;
- переносить ключевые v6 fact lines в Completion Timeline panel, clipboard и incident handoff summary;
- явно помечать отсутствие `v6` полей на `v5`, а не синтезировать их.

Принцип тот же, что и для `v5`: raw timeline остаётся source of truth, derived surfaces только делают authoritative fields читаемыми.

## Предлагаемая модель данных

### Timeline contract `v6`
Контракт `v6` остаётся additive superset относительно `v5`.

Новые bounded поля:
- в `server_edge_details`:
  - `method_entered_at_ms`;
  - `transport_to_method_wait_ms`;
  - `method_prelude_exec_ms`;
- в `prepare_details`:
  - optional `timeout_attribution`;
- в `prepare_details.exact_wait`:
  - optional `artifact_poll`.

Все новые state-поля используют только bounded vocabulary, а numeric поля выражаются в миллисекундах или версиях файлов.

### Human-readable перенос
Existing surfaces должны уметь выразить как минимум следующие authoritative verdict'ы, если `v6` поля присутствуют:

- `ingress_before_method_entry`;
- `handler_prelude_dominant`;
- `prepare_timeout@prepare_guard`;
- `prepare_timeout@interactive_wait_budget`;
- `exact_deadline@artifact_poll`.

Если payload старее `v6`, surfaces остаются рабочими, но явно помечают отсутствие этих verdict details как unavailable.

## Совместимость и rollout
- Backend bump'ает contract version до `6`.
- Extension принимает `v5` и `v6`.
- `v5` остаётся валидным partial source: existing `v5` drilldown сохраняется, но новые v6 fact lines не выводятся и помечаются как unavailable.
- Incident bundle не должен ломаться на `v5`: raw attachments и summary остаются валидными.

## Риски / Trade-offs
- Дополнительные timeline поля увеличивают размер payload. Смягчение: только bounded numeric/state fields и только для already-instrumented request traces.
- Timestamp capture в request path не должен сам стать latency source. Смягчение: только дешёвые monotonic/unix timestamp'ы без IO.
- Есть риск, что extension начнёт интерпретировать поля собственной эвристикой. Смягчение: derived verdict'ы строятся только из authoritative structured fields и не invent'ят данные.

## Validation Strategy
- Backend contract tests:
  - `v6` version bump и additive shape;
  - bounded vocabulary для `timeout_attribution.source` и новых readiness fields;
  - late timeout wake path фиксирует `budget_ms`, `elapsed_ms`, `overshoot_ms`;
  - artifact polling path фиксирует `poll_count`/`poll_elapsed_ms` без перехода в waiter/task-state, если budget закончился раньше.
- Extension tests:
  - `v6` payload парсится и переносится в panel/clipboard/incident summary;
  - `v5` payload деградирует явно без invented data.
- Smoke/docs:
  - runbook и readiness assets отражают `contract=v6` и новые root-cause fact lines.
