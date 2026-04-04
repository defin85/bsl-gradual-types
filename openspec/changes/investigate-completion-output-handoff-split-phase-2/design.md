## Контекст

Phase 1 (`investigate-completion-output-egress-split-phase-1`) довёл completion egress split до truthful `v23`, где timeline уже умеет отделять:

- `response_ready_to_output_enqueue_wait_ms`;
- `response_output_queue_wait_ms`;
- `response_output_encode_exec_ms`;
- `response_output_write_and_flush_exec_ms`.

Но свежий дамп `2026-04-04` показал, что dominant blind spot теперь сидит до legacy writer-selection boundary:

- `request=77`: `response_ready_to_output_enqueue_wait_ms=6144`, а queue/encode/write buckets равны `0/0/1`;
- `request=70`: `response_ready_to_output_enqueue_wait_ms=2480`, а queue/encode/write buckets равны `0/0/0`;
- `request=60`: `response_ready_to_output_enqueue_wait_ms=2442`, а queue/encode/write buckets равны `0/0/1`.

Runtime audit показал ещё одну важную деталь: `response_output_enqueue_completed_at_ms` уже shipped с misleading именем. Timestamp ставится в output loop после `outbound.next()`, то есть на фактической границе writer selection/dequeue, а не на send-side enqueue acceptance.

Значит следующий truthful шаг должен расследовать не writer backlog, а post-handler handoff path между `response_sent_at_ms` и legacy seam `response_output_enqueue_completed_at_ms`.

## Versioning Note

Текущий shipped state после phase 1:

- `response.version = 23`;
- `contracts/lsp-completion-timeline/v20`.

Новый change строится поверх него и целится в:

- `response.version = 24`;
- `contracts/lsp-completion-timeline/v21`.

## Goals

- Разложить dominant post-handler handoff blind spot на truthful `v24` server-only clocks и derived waits.
- Сохранить shipped `v23` как compatibility surface без retroactive reinterpretation.
- Избежать guessed backlog attribution и не смешивать handoff timing с writer-queue culprit claims.
- Убрать риск drift между live trace-store patch path и helper-built completion traces.

## Non-Goals

- Не добавлять writer-backlog snapshot (`output_messages_ahead_count`, `output_bytes_ahead_estimate`, `output_head_blocker_class`) в этой фазе.
- Не менять merged outbound path fairness или admission policy.
- Не лечить сам latency tail в рамках этого change.
- Не переопределять `response_ready_to_output_enqueue_wait_ms` или `response_output_enqueue_completed_at_ms` как новые truthful buckets вместо compatibility surface.
- Не делать in-flight partial publication, пока response ещё не дошёл до flush completion.

## Решения

### 1. `response_output_enqueue_completed_at_ms` фиксируется как legacy writer-selection seam

`response_sent_at_ms` остаётся handler-local response-ready boundary.

`response_output_enqueue_completed_at_ms` сохраняется в `v24`, но явно документируется как legacy compatibility boundary: это момент, когда completion response уже выбран output writer'ом из merged outbound stream.

`v24` MUST NOT переосмыслять это поле как truthful send-side enqueue acceptance. Новые consumers должны рассматривать его как writer-selection seam, несмотря на историческое имя.

### 2. Truthful handoff split требует двух новых send-side boundaries

`v24` добавляет два новых timestamp:

- `response_output_handoff_started_at_ms`;
- `response_output_handoff_enqueued_at_ms`.

Они означают:

- `response_output_handoff_started_at_ms` = момент, когда completion response впервые начинает проходить через send-side outbound handoff path после подготовки handler'ом;
- `response_output_handoff_enqueued_at_ms` = момент, когда `responses_tx.send(...)` успешно завершается и response принят outbound response path.

`response_output_enqueue_completed_at_ms` остаётся legacy upper boundary для следующего segment и не конкурирует с новыми truthful handoff clocks.

### 3. `v24` вводит три disjoint derived waits для legacy pre-writer umbrella

Если `response_output_handoff_started_at_ms` присутствует, payload MUST включать и `response_output_handoff_enqueued_at_ms`.

Если оба новых timestamp присутствуют, payload MUST включать:

- `response_ready_to_output_handoff_wait_ms = response_output_handoff_started_at_ms - response_sent_at_ms`;
- `response_output_handoff_send_wait_ms = response_output_handoff_enqueued_at_ms - response_output_handoff_started_at_ms`;
- `response_output_handoff_to_writer_wait_ms = response_output_enqueue_completed_at_ms - response_output_handoff_enqueued_at_ms`.

Интерпретация:

- `response_ready_to_output_handoff_wait_ms` отражает только server-side delay до входа completion response в outbound handoff path;
- `response_output_handoff_send_wait_ms` отражает только server-side delay внутри send-side handoff path до успешного завершения `responses_tx.send(...)`;
- `response_output_handoff_to_writer_wait_ms` отражает только server-side delay после успешного handoff acceptance до фактического выбора completion response output writer'ом.

Existing `response_ready_to_output_enqueue_wait_ms` сохраняется как compatibility umbrella:

- `response_ready_to_output_enqueue_wait_ms = response_output_enqueue_completed_at_ms - response_sent_at_ms`.

Consumers MUST NOT ретроактивно трактовать `v23` payload как будто новые truthful handoff boundaries уже были наблюдаемы.

### 4. Все `v24` handoff и egress milestones публикуются одним atomic patch carrier

Как и для `v23`, partial publication недопустима: immediate follow-up `bsl.getCompletionTimeline` не должен видеть half-populated `v24` state.

Поэтому transport/output path должен переносить в authoritative trace store единый bounded patch carrier, содержащий:

- `request_id`;
- `response_output_handoff_started_at_ms`;
- `response_output_handoff_enqueued_at_ms`;
- existing enqueue/encode/write/flush milestones для completion response.

Trace store применяет patch синхронно и idempotent, чтобы consumer видел либо целостный `v24` split, либо legacy surface без частичной смеси.

### 5. Post-response derivation должна жить в одном shared calculator

Сейчас те же egress waits вычисляются в нескольких местах: live trace-store patch path и helper-built terminal trace path.

Для `v24` это становится слишком рискованно, поэтому новый handoff math должен жить в одном shared helper, который переиспользуется:

- при синхронном patch apply в authoritative trace store;
- при сборке terminal/server-edge details из completion capture;
- в focused tests на ordering и derived buckets.

Иначе один и тот же request сможет показывать разные post-response buckets в зависимости от code path публикации.

### 6. Handoff split в этой фазе остаётся timing-only, без culprit attribution

`response_output_handoff_send_wait_ms` и `response_output_handoff_to_writer_wait_ms` означают только timing seams.

Payload и surfaces:

- MAY показывать compatibility umbrella `response_ready_to_output_enqueue_wait_ms`;
- MUST NOT переименовывать `response_output_handoff_send_wait_ms` или `response_output_handoff_to_writer_wait_ms` в `notification backlog`, `executeCommand blocker`, `writer queue backlog` или иной более точный culprit без новых authoritative snapshot fields.

### 7. Atomic publication остаётся post-hoc only

Этот change сохраняет atomic publish после flush completion.

Следствие:

- immediate follow-up после завершённого response видит целостный `v24` split;
- но in-flight poll во время зависшего handoff может ещё не видеть новые поля.

Это допустимо в этой фазе и не должно маскироваться как live partial observability.

### 8. `v23` degradation остаётся fail-closed

На `v23` surfaces:

- не выдумывают `response_output_handoff_started_at_ms`;
- не выдумывают `response_output_handoff_enqueued_at_ms`;
- прямо говорят, что truthful pre-enqueue handoff split unavailable by design;
- не делят `response_ready_to_output_enqueue_wait_ms` на invented finer buckets.

## Alternatives Considered

### Делать writer backlog attribution раньше handoff split

Отклонено. Наблюдаемые traces показывают нулевой или почти нулевой `response_output_queue_wait_ms`, поэтому этот change не попадёт в текущий dominant blind spot.

### Переосмыслить existing `response_output_enqueue_completed_at_ms` как truthful enqueue acceptance

Отклонено. Runtime уже поставил это поле на другую seam, поэтому получится скрытое переименование shipped compatibility boundary.

### Ограничиться одним новым `response_output_handoff_started_at_ms`

Отклонено. Одного нового timestamp недостаточно, чтобы отделить send-side handoff delay от последующего wait до writer selection; blind spot останется частично opaque.

### Публиковать free-text handoff reason вместо bounded clocks

Отклонено. Это нарушит bounded contract discipline и быстро уйдёт в high-cardinality evidence.

## Риски и Trade-offs

### Риск: transport adapter придётся протянуть две send-side boundary через generic output path

Смягчение:

- ограничить новый carrier двумя timestamp и bounded derived waits;
- оставить existing enqueue/encode/write/flush path и ordering policy без refactor.

### Риск: surfaces станут перегруженными ещё двумя finer bucket'ами

Смягчение:

- показывать finer split только при `response.version >= 24`;
- сохранять `response_ready_to_output_enqueue_wait_ms` как компактную umbrella summary.

### Риск: legacy naming будет продолжать путать downstream

Смягчение:

- в contract/changelog/surfaces явно назвать `response_output_enqueue_completed_at_ms` legacy compatibility seam;
- для `v24` consumers рекомендовать опираться на новые `response_output_handoff_*` поля и derived waits.

### Риск: следующий backlog-attribution change всё равно понадобится

Смягчение:

- явно трактовать этот change как закрытие текущего dominant blind spot;
- выносить writer-backlog attribution в отдельную последующую фазу только если новые traces покажут ненулевой `response_output_queue_wait_ms` как главный tail.
