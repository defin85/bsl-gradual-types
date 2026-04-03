## Контекст

После `investigate-completion-transport-gap` completion timeline уже честно показывает coarse server egress tail как интервал между `response_sent_at_ms` и `response_flush_completed_at_ms`.

Однако текущий payload не отделяет:

- enqueue wait до успешной постановки completion response в outbound response path;
- queue wait до начала фактического write;
- encode/serialize exec;
- write+flush exec.

При этом архитектурное ревью показало, что truthful backlog attribution требует отдельного transport refactor и не должно смешиваться с этим шагом.

## Versioning Note

Текущий target-state после `investigate-completion-transport-gap`:

- `response.version = 21`;
- `contracts/lsp-completion-timeline/v18`.

Эта фаза строится поверх него и целится в:

- `response.version = 22`;
- `contracts/lsp-completion-timeline/v19`.

## Goals

- Разложить `response_ready_to_flush_wait_ms` на bounded server-only clocks и derived waits.
- Сохранить additive/backward-compatible semantics для уже shipped `v21` полей.
- Избежать partial `v22` state при immediate follow-up `bsl.getCompletionTimeline`.

## Non-Goals

- Не публиковать queue backlog snapshot в этой фазе.
- Не рефакторить merged outbound path в unified queue/envelope.
- Не менять scheduling/fairness output writer.

## Решения

### 1. `response_sent_at_ms` и `response_flush_completed_at_ms` остаются без переосмысления

`response_sent_at_ms` остаётся handler-local response-ready boundary.
`response_flush_completed_at_ms` остаётся flush completion boundary.

`v22` добавляет только intermediate milestones:

- `response_output_enqueue_completed_at_ms`;
- `response_output_write_started_at_ms`;
- `response_output_encode_completed_at_ms`.

Derived поля:

- `response_ready_to_output_enqueue_wait_ms`;
- `response_output_queue_wait_ms`;
- `response_output_encode_exec_ms`;
- `response_output_write_and_flush_exec_ms`.

### 2. Все egress milestones публикуются одним atomic patch carrier

Текущий `v21` path уже синхронно патчит flush completion в trace store.
Для `v22` этого недостаточно: несколько разрозненных callbacks создадут partial trace state.

Поэтому transport path должен передавать в trace store единый bounded patch carrier, содержащий:

- `request_id`;
- observed enqueue/write/encode/flush milestones для completion response.

Trace store применяет patch синхронно и idempotent, чтобы immediate `bsl.getCompletionTimeline` видел целостный `v22` split.

### 3. Queue wait в этой фазе остаётся timing-only, без blocker attribution

`response_output_queue_wait_ms` в этой фазе означает только delay между completion enqueue completion и фактическим write start.

Payload и human-readable surfaces:

- MAY продолжать показывать umbrella `response_ready_to_flush_wait_ms`;
- MUST NOT называть queue wait “backlog от notification”, “executeCommand blocker” или иным более точным culprit без новых bounded snapshot fields.

### 4. `v21` degradation остаётся fail-closed

На `v21` surfaces:

- не выдумывают enqueue/queue/encode/write buckets;
- прямо говорят, что finer output-egress split unavailable by design;
- не переименовывают coarse `response_ready_to_flush_wait_ms` в queue backlog или flush bottleneck.

## Alternatives Considered

### Сразу включить backlog snapshot в `v22`

Отклонено. Текущий merged outbound path не даёт truthful blocker metadata без отдельного transport refactor.

### Ограничиться только новыми timestamps без derived waits

Отклонено. Тогда consumers снова вынуждены вручную вычитать timestamp'ы.

## Риски и Trade-offs

### Риск: transport callback surface распухнет

Смягчение:

- единый bounded patch carrier вместо набора несвязанных hooks;
- только completion response path, без generic payload inspection.

### Риск: encode timestamp исказит наблюдение

Смягчение:

- фиксировать `response_output_encode_completed_at_ms` вокруг реального `serde_json::to_vec(...)`;
- включать header/body write и flush только в `response_output_write_and_flush_exec_ms`.

### Риск: surfaces станут перегруженными

Смягчение:

- показывать finer split только при `response.version >= 22`;
- сохранять umbrella `response_ready_to_flush_wait_ms` как компактную сводку.
