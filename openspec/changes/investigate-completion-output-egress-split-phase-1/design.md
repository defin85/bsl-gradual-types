## Контекст

После `investigate-completion-transport-gap` completion timeline уже честно показывает coarse server egress tail как интервал между `response_sent_at_ms` и `response_flush_completed_at_ms`.

Первый shipped шаг `v22` добавил finer split, но acceptance-review обнаружил semantic mismatch:

- `response_output_write_started_at_ms` сейчас фиксируется до `serde_json::to_vec(...)`;
- `response_output_queue_wait_ms` поэтому включает encode time, хотя promised wording говорит про literal write start;
- исправить это задним числом в `v22` нельзя без скрытого breaking reinterpretation.

При этом truthful backlog attribution по-прежнему требует отдельного transport refactor и не должно смешиваться с этим шагом.

## Versioning Note

Текущий shipped state после первой реализации этого change:

- `response.version = 22`;
- `contracts/lsp-completion-timeline/v19`.

Доведение до 100% строится поверх него и целится в:

- `response.version = 23`;
- `contracts/lsp-completion-timeline/v20`.

## Goals

- Разложить coarse server egress tail на truthful `v23` server-only clocks и derived waits.
- Сохранить shipped `v22` как compatibility surface без retroactive reinterpretation.
- Избежать partial `v23` state при immediate follow-up `bsl.getCompletionTimeline`.

## Non-Goals

- Не публиковать queue backlog snapshot в этой фазе.
- Не рефакторить merged outbound path в unified queue/envelope.
- Не менять scheduling/fairness output writer.
- Не делать hidden breaking reinterpretation для already shipped `v22`.

## Решения

### 1. `response_sent_at_ms` и `response_flush_completed_at_ms` остаются без переосмысления

`response_sent_at_ms` остаётся handler-local response-ready boundary.
`response_flush_completed_at_ms` остаётся flush completion boundary.

`v23` добавляет intermediate milestones:

- `response_output_enqueue_completed_at_ms`;
- `response_output_encode_started_at_ms`;
- `response_output_encode_completed_at_ms`;
- `response_output_write_started_at_ms`;

`response_output_encode_started_at_ms` означает старт output encode phase.
`response_output_write_started_at_ms` означает первый фактический write в transport writer.

Derived `v23` поля:

- `response_ready_to_output_enqueue_wait_ms`;
- `response_output_queue_wait_ms`;
- `response_output_encode_exec_ms`;
- `response_output_write_and_flush_exec_ms`.

Границы вычисления:

- `response_ready_to_output_enqueue_wait_ms = enqueue_completed - response_sent`;
- `response_output_queue_wait_ms = encode_started - enqueue_completed`;
- `response_output_encode_exec_ms = encode_completed - encode_started`;
- `response_output_write_and_flush_exec_ms = flush_completed - write_started`.

Compatibility umbrella `response_ready_to_flush_wait_ms` сохраняется, но consumers MUST NOT использовать его как exact checksum суммы finer buckets.

### 2. `v22` остаётся shipped compatibility surface, truthful redesign идёт только в `v23`

`v22` уже опубликован и остаётся поддерживаемым legacy surface.
Новый truthful split должен появиться только в additive `v23`.

На `v22` surfaces и evidence bundles:

- прямо говорят, что literal encode-start/write-start split unavailable by design;
- не переименовывают shipped `response_output_write_started_at_ms` задним числом;
- не пытаются выводить truthful first-write semantics из `v22`.

### 3. Все `v23` egress milestones публикуются одним atomic patch carrier

Текущий path уже синхронно патчит flush completion в trace store.
Для `v23` несколько разрозненных callbacks всё ещё недопустимы: они создадут partial trace state.

Поэтому transport path должен передавать в trace store единый bounded patch carrier, содержащий:

- `request_id`;
- observed enqueue/encode-start/encode-complete/write-start/flush milestones для completion response.

Trace store применяет patch синхронно и idempotent, чтобы immediate `bsl.getCompletionTimeline` видел целостный `v23` split.

### 4. Queue wait в этой фазе остаётся timing-only, без blocker attribution

`response_output_queue_wait_ms` в `v23` означает только delay между completion enqueue completion и encode-start boundary.

Payload и human-readable surfaces:

- MAY продолжать показывать umbrella `response_ready_to_flush_wait_ms`;
- MUST NOT называть queue wait “backlog от notification”, “executeCommand blocker” или иным более точным culprit без новых bounded snapshot fields.

### 5. `v22` degradation остаётся fail-closed

На `v22` surfaces:

- не выдумывают literal encode-start/write-start split;
- прямо говорят, что truthful `v23` split unavailable by design;
- не переименовывают shipped `response_output_write_started_at_ms` в literal first write boundary.

## Alternatives Considered

### Переосмыслить `v22` задним числом

Отклонено. Это превратит additive shipped fields в скрыто переопределённый контракт.

### Ограничиться только исправлением docs без нового contract bump

Отклонено. Тогда accepted semantic mismatch останется в runtime payload и human-readable surfaces.

## Риски и Trade-offs

### Риск: transport callback surface распухнет

Смягчение:

- единый bounded patch carrier вместо набора несвязанных hooks;
- только completion response path, без generic payload inspection.

### Риск: finer buckets не будут суммироваться в compatibility umbrella

Смягчение:

- трактовать `response_ready_to_flush_wait_ms` как compatibility umbrella, а не checksum;
- тестировать truthful ordering и individual bucket derivation отдельно.

### Риск: surfaces станут перегруженными

Смягчение:

- показывать truthful split только при `response.version >= 23`;
- сохранять umbrella `response_ready_to_flush_wait_ms` как компактную сводку.
