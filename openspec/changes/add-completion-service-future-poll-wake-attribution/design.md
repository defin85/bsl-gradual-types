## Контекст
`Completion Timeline v10` уже дал bounded cut до `RequestContextService` и внутри request-context handoff:
- `jsonrpc_dispatch_received -> RequestContextService::call`;
- `RequestContextService::call -> service_future_created`;
- `service_future_created -> service_scope_entered`;
- `service_scope_entered -> method_entered`.

Incident bundle от `2026-03-21T16:29:20.495Z` показывает, что этого уже достаточно, чтобы сузить root-cause search до участка после возврата service future и до первого входа в request scope.

Для traces `50`, `55`, `70` и `74` authoritative payload сообщает:
- `dispatch_to_request_context_wait_ms=0`;
- `transport_to_service_future_wait_ms=0`;
- `service_future_to_scope_wait_ms=11945/14715/5896/8881`;
- `service_scope_to_method_wait_ms=0`;
- `dispatcher_resolution_latency_ms=0`;
- `turn_wait_outcome=ready`.

То есть:
- lag до `RequestContextService::call` уже исключён;
- sync путь до возврата future уже исключён;
- pre-method prelude не объясняет секунды ожидания;
- hidden backlog всё ещё может жить либо до первого `poll()` returned future, либо уже после первого `Pending` на пути до первого `wake`.

Сейчас это неразличимо, потому что текущий payload видит только coarse факт `service_future_created -> service_scope_entered`.

## Цели
- Дать следующий bounded split внутри сегмента `service_future_created -> service_scope_entered`.
- Отделить "future ещё не poll'или" от "future уже poll'или, но она висела в `Pending` без первого wake".
- Сохранить additive contract discipline и truthful semantics `v10`.
- Оставить operator-facing surfaces self-contained и человекочитаемыми.

## Не-цели
- Не обещать финальный root cause внутри tower/runtime scheduler.
- Не добавлять unbounded event history.
- Не менять completion результат, handler routing или observability probe schema.
- Не подменять authoritative payload клиентской реконструкцией.

## Решение

### 1. Instrumented service future вокруг `inner.call(request)`
Нужен bounded wrapper вокруг future, возвращаемой `inner.call(request)`.

Wrapper должен уметь фиксировать:
- момент первого входа в `poll()` как `service_future_first_poll_entered_at_ms`;
- bounded outcome первого poll:
  - `ready`;
  - `pending`.

Если первый poll вернул `Pending`, wrapper должен один раз зафиксировать момент первого `wake` / `wake_by_ref` как `service_future_first_wake_scheduled_at_ms`.

Цель instrumentation не в том, чтобы собрать полную историю scheduler events, а в том, чтобы дать минимальный bounded cut:
- дошла ли future до первого poll быстро;
- был ли первый poll сразу `Ready` или `Pending`;
- если был `Pending`, когда случился первый wake.

### 2. `v11` остаётся additive и bounded
Новый contract нужен, потому что меняется authoritative payload shape и interpretation rules.

`server_edge_details` должен дополнительно включать:
- optional `service_future_first_poll_entered_at_ms`;
- optional `service_future_to_first_poll_wait_ms`;
- optional `service_future_first_poll_outcome`;
- optional `service_future_first_wake_scheduled_at_ms`;
- optional `first_poll_to_first_wake_wait_ms`.

`service_future_first_poll_outcome` MUST использовать только bounded vocabulary:
- `ready`;
- `pending`.

Derived semantics:
- если присутствует `service_future_first_poll_entered_at_ms`, payload MUST включать `service_future_to_first_poll_wait_ms`;
- если присутствует `service_future_first_wake_scheduled_at_ms`, payload MUST включать `first_poll_to_first_wake_wait_ms`;
- если `service_future_first_poll_outcome=ready`, payload MUST NOT выдумывать first-wake fields;
- если первый wake не наблюдался, payload MUST NOT реконструировать его по косвенным timestamp'ам.

Репозиторий должен поддерживать contiguous versioned contract baseline:
- `contracts/lsp-completion-timeline/v8` для shipped `response.version=11`;
- `contracts/lsp-completion-timeline/v7` остаётся baseline для `response.version=10`.

### 3. Existing semantics не ослабляются
Новый change не должен ломать уже существующие trustworthy rules:
- `v10` dispatch provenance;
- `v9` pre-service-scope split;
- `v8` pre-method provenance;
- fail-closed semantics для overlap/fallback request paths.

Если future wrapper не может доказать bounded факт, payload обязан оставить поле отсутствующим.

### 4. Existing completion surfaces
Panel, clipboard и request-centric incident bundle summary должны:
- показывать first-poll / first-wake split отдельными fact lines;
- сохранять уже существующие `v10` / `v9` / `v8` facts;
- не выдумывать `v11` fields на `v10`;
- явно называть limitation на `v10`, а не скрывать её в нейтральном summary.

Новый change не требует новую verdict taxonomy. Достаточно, чтобы human-readable output позволял отличить:
- lag до первого poll returned future;
- lag между первым `Pending` poll и первым wake;
- lag уже после первого wake внутри дальнейшего future path.

## Риски и trade-offs

### Риск: первый wake не всегда равен "полезному прогрессу"
Смягчение:
- change не обещает финальный root cause;
- он обещает ещё один bounded narrowing cut, достаточный для следующего цикла расследования.

### Риск: wrapper вокруг future внесёт шум в async path
Смягчение:
- instrumentation остаётся минимальной и одноразовой;
- shape bounded и additive;
- нет полного event log и нет свободного текста.

### Риск: operator-facing surfaces начнут смешивать `v10` и `v11` semantics
Смягчение:
- change требует explicit `v10` degradation note;
- panel, clipboard и incident bundle обновляются вместе с contract;
- smoke/runbook ожидания обновляются одновременно.

## Acceptance-направление
- Trace с большим `service_future_to_scope_wait_ms` показывает, сидит ли lag до первого poll future или уже после первого `Pending`.
- `v11` payload сохраняет existing `v10` / `v9` / `v8` trustworthy semantics и bounded discipline.
- Panel / clipboard / incident bundle переносят новый split без invented reconstruction.
- `v10` payload деградирует явно и не маскирует missing first-poll / first-wake split как отсутствие gaps.
