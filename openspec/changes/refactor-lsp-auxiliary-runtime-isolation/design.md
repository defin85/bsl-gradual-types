## Контекст
Новые timeline splits уже показали, что проблема не в UI pre-send path:
- `client_before_transport_write_wait_ms` остаётся маленьким;
- пользовательская задержка уходит в `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms`.

Кодовое расследование выявило два server-side offender path:
- background parse worker после bounded parse возвращается на async runtime и там вызывает `build_document_symbols(...)`;
- `bsl.getCurrentContext` делает parse/context derivation inline внутри async handler.

Оба path попадают в тот же runtime, который должен продолжать читать transport frames, first-poll-ить service futures и продвигать completion handoff/output loops.

## Goals
- Убрать CPU-heavy auxiliary LSP work с async transport/runtime loop.
- Сохранить `documentSymbol` как bounded auxiliary navigation surface.
- Сделать mixed-load starvation regression-проверяемой через truthful ingress/egress seams.
- Не размыть расследование обратно в UI/client path без новых доказательств.

## Non-Goals
- Не переписывать `transport_adapter` в отдельную transport runtime architecture.
- Не менять fairness/ordering policy outbound queue.
- Не менять semantic contract `documentSymbol`, `completion` или `bsl.getCurrentContext`.
- Не оптимизировать сам `query_bundle_ir_query` в этом change.

## Решения

### 1. Async LSP runtime остаётся orchestration boundary, а не местом для тяжёлой auxiliary CPU work
CPU-heavy auxiliary work, не являющаяся primary semantic body текущего interactive ответа, должна уходить в bounded blocking path или эквивалентную isolated CPU boundary.

На async runtime остаются:
- transport read/write loops;
- admission / scheduling;
- cancellation / coalescing orchestration;
- async ожидания и lightweight state checks.

### 2. Outline materialization после document sync не должна возвращать CPU-heavy symbol building на async runtime
Background parse snapshot apply после `didOpen`/`didChange`/`didSave` может продолжать использовать bounded parse path, но последующий `build_document_symbols(...)` тоже должен оставаться на auxiliary CPU boundary.

То же относится к same-version outline refresh после `didSave`: этот path остаётся auxiliary и не должен монополизировать runtime, который обслуживает newer interactive requests.

### 3. `bsl.getCurrentContext` остаётся bounded auxiliary command
Команда может оставаться request/response API без изменения результата, но её parse/context derivation не должна выполняться inline на async handler path.

Точный выбор CPU class может быть уточнён на implementation этапе, но hard requirement один: такой path не должен starvation-ить transport ingress, service future first poll или completion output handoff.

### 4. Representative mixed-load acceptance MUST смотреть на truthful seams, а не только на legacy pre-dispatch bucket
После новых метрик regression gate обязан смотреть минимум на:
- `client_to_transport_wait_ms`;
- `service_future_to_first_poll_wait_ms`;
- `response_output_handoff_send_wait_ms`.

Иначе auxiliary-runtime starvation может маскироваться ситуацией, где `adapter_to_dispatch_wait_ms` остаётся в бюджете, но user-visible latency уже ушла в другой truthful seam.

### 5. Existing observability attribution changes считаются prerequisite evidence, а не remediation substitute
Change не расширяет observability scope сам по себе. Он использует уже появившиеся splits как acceptance/diagnostic truth source для remediation.

## Alternatives Considered

### Оставить auxiliary paths на async runtime и просто добавить больше метрик
Отклонено. Это полезно для расследования, но не устраняет starvation и не закрывает ide-grade intent.

### Переписать весь transport path на отдельный dedicated runtime
Отклонено как слишком широкий scope для текущего root cause. Сначала нужно убрать явные CPU-heavy offenders с existing runtime boundary.

### Считать `documentSymbol` request-path основной причиной
Отклонено. Сам request-path mostly cache-backed; найденный offender живёт в background/same-version outline materialization после document sync.

## Риски и trade-offs

### Риск: bounded CPU path сам станет bottleneck
Смягчение:
- использовать уже существующую bounded execution policy;
- опираться на truthful queue-wait/exec metrics и representative mixed-load gate.

### Риск: перенос `getCurrentContext` в auxiliary CPU boundary ухудшит его median latency
Смягчение:
- change целится в отсутствие starvation interactive path;
- если понадобится, work class можно подобрать отдельно, не возвращая inline async parse.

### Риск: gate станет чувствительным к unrelated backlog noise
Смягчение:
- использовать representative same-file profile;
- budget-ить truthful seams вместе с existing correctness invariants, а не в отрыве от route/outcome.

## Migration / Rollout
1. Вынести CPU-heavy outline materialization с async runtime.
2. Вынести `bsl.getCurrentContext` parse/context derivation с async runtime.
3. Обновить representative mixed-load perf gate и artifacts на truthful ingress/egress seams.
4. Переснять incident bundle и подтвердить, что regression переместилась или исчезла, а не только лучше атрибутируется.

## Open Questions
- Нужен ли `bsl.getCurrentContext` отдельный work class относительно other auxiliary CPU work, или достаточно existing bounded policy с корректным class selection на implementation этапе?
