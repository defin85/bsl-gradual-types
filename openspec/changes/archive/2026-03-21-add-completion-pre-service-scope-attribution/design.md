## Контекст
`Completion Timeline v8` уже даёт оператору trustworthy pre-method attribution:
- `transport_received -> service_scope_entered -> method_entered`;
- bounded `pre_method_attribution_provenance`;
- truthfulness rules для strong ingress verdict и incident summary.

По incident bundle этого уже достаточно, чтобы локализовать текущий dominant lag в сегмент `transport_received -> service_scope_entered`.

В bundle, экспортированном `2026-03-21T12:24:04.933Z`, completion timeline всё ещё имеет `contract=v8`, а request summary показывает:
- trace `790`: `transport_to_service_scope_wait_ms=11875`, `service_scope_to_method_wait_ms=0`;
- trace `815`: `transport_to_service_scope_wait_ms=2957`, `service_scope_to_method_wait_ms=0`;
- trace `819`: `transport_to_service_scope_wait_ms=5930`, `service_scope_to_method_wait_ms=0`.

То есть оператор уже знает, что lag сидит до `service_scope_entered`, но не может ответить, сидит ли он:
- до возврата `inner.call(request)` и создания service future;
- или уже после создания future, но до первого poll request scope.

Но внутри этого сегмента сейчас есть только один итоговый wait:
- `transport_to_service_scope_wait_ms`

Этого недостаточно, чтобы различить две разные гипотезы:
1. задержка на синхронном сервисном проходе до возврата `inner.call(request)`;
2. задержка после создания service future, но до первого poll в request scope.

Текущий код уже имеет естественную точку для дополнительного bounded timestamp:
- `request_received_at_ms` ставится до `inner.call(request)`;
- `service_scope_entered_at_ms` ставится внутри async future при первом входе в request context.

Между ними отсутствует только timestamp момента, когда service future уже создан и возвращён из `inner.call(request)`.

## Цели
- Сузить pre-service-scope blind spot без нового диагностического канала.
- Сохранить bounded/additive contract discipline.
- Оставить trustworthy semantics из `v8` неизменными.
- Дать человеку и incident bundle следующий диагностический cut без чтения сырого кода.

## Не-цели
- Не лечить сам scheduler/executor/service backlog.
- Не добавлять unbounded event log.
- Не менять completion результат или LSP transport semantics.
- Не ослаблять `same_request_authoritative` / `best_effort_fallback` semantics из `v8`.

## Решение

### 1. Новый bounded timestamp внутри `RequestContextService`
Добавить одну дополнительную метку:
- `service_future_created_at_ms`

Она должна сниматься сразу после `inner.call(request)` возвращает future, но до первого await/poll этого future.

На её основе authoritative payload сможет сериализовать:
- `transport_to_service_future_wait_ms`
- `service_future_to_scope_wait_ms`

где:
- `transport_to_service_future_wait_ms = service_future_created_at_ms - transport_received_at_ms`
- `service_future_to_scope_wait_ms = service_scope_entered_at_ms - service_future_created_at_ms`

### 2. Contract `v9` остаётся additive и bounded
Новый contract нужен, потому что меняется authoritative payload shape. При этом:
- existing `v8` fields сохраняются;
- новые поля optional и bounded;
- никаких free-text объяснений или high-cardinality identifiers не добавляется.
- versioned contract baseline в репозитории должен эволюционировать как contiguous major
  `contracts/lsp-completion-timeline/v5 -> v6`, а policy checker обязан ожидать
  именно этот новый baseline вместо старого `response.version=3`.

Если `service_future_created_at_ms` присутствует, payload должен включать и оба derived waits, чтобы operator и summary не вычисляли их вручную.

### 3. Trustworthy semantics не меняются
`v9` не вводит новую provenance vocabulary. Новый split наследует уже существующую `v8` integrity semantics:
- same-request authoritative attribution остаётся strong;
- best-effort fallback остаётся weak;
- `v8` server не должен интерпретироваться как если бы он уже знал `service_future_created_at_ms`.

### 4. Existing completion surfaces
Panel, clipboard и request-centric incident bundle summary должны:
- показывать новый pre-service-scope split отдельными fact lines;
- не выдумывать `service_future_created_at_ms` или derived waits на `v8`;
- сохранять уже существующие truthful verdict rules.

Новый change не обязан вводить новый verdict taxonomy. Достаточно, чтобы human-readable output позволял отличить:
- lag до `service_future_created`;
- lag после `service_future_created`, но до первого poll request scope.

### 5. Explicit `v8` degradation должна быть человекочитаемой
Текущий incident bundle уже пишет `contract=v8`, но этого недостаточно: request summary и `No gaps were recorded` не проговаривают, что pre-service-scope split отсутствует by design, а не просто не попал в конкретный capture.

Поэтому change должен требовать, чтобы human-readable surfaces:
- явно называли отсутствие `v9` split на `v8` как expected limitation;
- не подменяли эту limitation нейтральным отсутствием gaps;
- не превращали отсутствие `service_future_created_at_ms` в guessed reconstruction.

## Риски и trade-offs

### Риск: новый split всё ещё не укажет root cause внутри executor
Смягчение:
- change не обещает финальный root cause;
- он обещает ещё один bounded narrowing cut внутри уже доказанного сегмента.

### Риск: часть traces не сможет заполнить новый split
Смягчение:
- поля optional;
- `v8` degradation остаётся explicit;
- отсутствие `v9` данных не реконструируется эвристикой.

### Риск: drift между raw contract и human-readable projection
Смягчение:
- change включает regression coverage для panel, clipboard и incident bundle summary;
- smoke/runbook ожидания обновляются вместе с projection;
- versioned contract artifacts и canonical OpenSpec truth синхронизируются с shipped
  `v9` payload, а не остаются на старом `v5` / `response.version=3`.

### Риск: `v8` limitation останется видимой только через `contract=v8`
Смягчение:
- change явно требует operator-facing note `split unavailable by design` для `v8`;
- incident bundle не должен одновременно скрывать limitation и писать `No gaps were recorded`.

## Acceptance-направление
- Trace с большим `transport_to_service_scope_wait_ms` показывает, где сидит лаг: до `service_future_created` или уже после создания future.
- `v9` payload сохраняет trustworthy pre-method semantics из `v8`.
- Panel/clipboard/incident bundle переносят новый split без invented reconstruction.
- `v8` payload деградирует явно, не выдаёт фиктивные `v9` поля и не оставляет operator-facing limitation неозвученной.
