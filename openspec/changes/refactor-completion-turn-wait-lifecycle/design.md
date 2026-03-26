## Контекст
Новый incident bundle `2026-03-26T11:13:29Z` с уже добавленной `turn_wait` telemetry сузил root cause.

Наблюдаемое поведение:
- `completion-trace-1` накапливает `14070ms` в `service_future_created -> first_poll`, но его собственный `turn_wait` резолвится мгновенно;
- в authoritative contenders для этого же trace присутствует same-file older completion в `phase=turn_wait` с возрастом `24037ms`;
- client-side `probe-1` становится superseded через `803ms`, но живёт до terminal state `24561ms`;
- `completion-trace-2` показывает `turn_wait` stage длиной `3510ms`, но абсолютные timestamps `turn_wait_entered_at_ms`, `turn_wait_resolved_at_ms` и `wake_after_turn_resolution_at_ms` совпадают.

Практический смысл: новый completion ждёт не current-request `turn_wait` и не heavy completion logic. Stall создаёт older same-file request, который уже вышел из queue, но ещё не считается active и потому выпадает из текущего queued/active supersession lifecycle.

Текущий код подтверждает этот разрыв:
- queue eviction и supersession работают для pending `CompletionRequest` в per-file queue;
- proactive cancel смотрит на `stale_active_request_ids(...)`, то есть только на уже active completion;
- `mark_completion_active(...)` вызывается после `turn_waiter.wait().await`.

Следовательно, request может оказаться в окне между queue removal и active registration, где он ещё inflight, но уже плохо виден runtime contract.

## Цели
- Гарантировать, что same-file completion request в `turn_wait` lifecycle не может стать orphaned после supersession/cancel.
- Обеспечить bounded stop older request до active registration и убрать multi-second pre-poll backlog для нового same-file completion.
- Сделать authoritative observability достаточной, чтобы отдельно видеть:
  - current-request `turn_wait`;
  - stale contender в `phase=turn_wait`;
  - корректные absolute timestamps для реально наблюдаемого wait lifecycle.

## Не-цели
- Не перепроектировать общий transport scheduler или admission policy всего LSP.
- Не переоткрывать `documentSymbol` change и не смешивать этот follow-up с auxiliary traffic.
- Не заменять root-cause fix приоритизацией, concurrency bump или stale fallback.

## Решения

### 1. `turn_wait` становится отдельным lifecycle state, а не скрытым переходом
Для runtime contract недостаточно деления только на queued и active completion.

Нужен явный промежуточный state:
- request уже вышел из queue или ожидает dispatcher turn;
- request ещё не считается active owner;
- request всё равно обязан участвовать в same-file latest-wins/cancel semantics.

Observable consequence:
- newer same-file completion или explicit cancel MUST уметь boundedly остановить stale request в этом состоянии;
- stale request MUST NOT висеть seconds-scale только потому, что он не успел перейти в active registry.

### 2. Supersession/cancel must cover queued, turn-waiting и active states единообразно
Текущий blind spot возникает потому, что queued eviction и active cancellation покрывают разные куски lifecycle.

Новый change фиксирует требование:
- queued stale requests по-прежнему вытесняются/отменяются до heavy path;
- `turn_wait` requests MUST иметь тот же latest-wins/cancel coverage;
- active requests MUST сохранять уже существующий prompt release contract из `refactor-completion-superseded-active-turn-release`.

Это сохраняет existing completion path и устраняет локальный lifecycle gap вместо нового scheduler bypass.

### 3. Observability должна различать current wait и stale contender без invented timestamps
`v16` telemetry уже полезна, но последний incident показывает ещё один изъян: multi-second `turn_wait` stage может сопровождаться схлопнутыми absolute timestamps.

Новый contract должен требовать:
- если current request реально ждал в `turn_wait`, absolute lifecycle MUST быть согласован со stage duration в пределах bounded measurement tolerance;
- если current request resolved immediately, но stale contender остаётся в `phase=turn_wait`, payload MUST показывать именно это разделение, а не смешивать оба состояния;
- baseline contract и incident export MUST оставаться bounded и backward-compatible для старых payload.

### 4. Acceptance должен ловить pre-active overlap, а не только active overlap
Предыдущий change уже закрыл stale active completion в `response_build`, но не сценарий, где stale request застревает ещё до active registration.

Новый acceptance layer должен проверять отдельный overlap profile:
- request `A` уже ушёл в `turn_wait`;
- приходит newer same-file request `B` или explicit cancel для `A`;
- `A` boundedly сворачивается без перехода в долгоживущий orphaned waiter;
- `B` не копит seconds-scale pre-poll backlog из-за stale `turn_wait` predecessor.

Representative real-module gate должен fail-ить не только на общий ingress stall, но и на same-file contender в `phase=turn_wait`, который переживает bounded supersession window.

## Рассмотренные альтернативы

### Поднять приоритет completion относительно других request classes
Отклонено. Это может уменьшить симптом, но не исправляет stale same-file request, который уже потерял актуальность и продолжает жить в completion lifecycle.

### Увеличить concurrency / slots
Отклонено. Это pressure relief, а не устранение orphaned waiter gap.

### Считать текущую telemetry достаточной и чинить только runtime
Отклонено. Последний incident уже показывает, что без truthful `turn_wait` timestamps часть различий между real wait и instrumentation drift остаётся неоднозначной.

## Риски и trade-offs

### Риск: новый lifecycle registry усложнит dispatcher
Допустимо, если state machine останется локальной для per-file completion dispatcher и не потечёт в другие LSP methods.

### Риск: overlap gate станет flaky
Смягчение:
- использовать same-file deterministic harness;
- опираться на server-side contender phase и bounded timeline fields, а не только на wall-clock клиента;
- держать отдельный real-module profile с checked-in evidence.

### Риск: change снова разрастётся в большой scheduler redesign
Смягчение:
- scope ограничен only completion `turn_wait` lifecycle;
- новые решения должны встраиваться в existing completion path;
- любые cross-method fairness/policy discussions остаются отдельным follow-up.
