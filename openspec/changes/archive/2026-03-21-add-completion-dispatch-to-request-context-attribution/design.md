## Контекст
`Completion Timeline v9` уже дал bounded split внутри `RequestContextService`:
- `transport_received -> service_future_created`;
- `service_future_created -> service_scope_entered`;
- `service_scope_entered -> method_entered`.

Incident bundle от `2026-03-21T15:03:18Z` показывает, что этого уже достаточно, чтобы сузить root-cause search до ingress участка перед method path.

Для traces `249`, `253` и `272` authoritative payload сообщает:
- `transport_to_service_future_wait_ms=0`;
- `service_future_to_scope_wait_ms=12264/28726/9464`;
- `service_scope_to_method_wait_ms=0`;
- `dispatcher_resolution_latency_ms=0`;
- `turn_wait_outcome=ready`.

То есть:
- completion dispatcher не является видимым bottleneck;
- pre-method prelude не объясняет десятки секунд задержки;
- hidden backlog всё ещё может жить либо до входа в `RequestContextService::call`, либо уже после `inner.call(request)`.

Сейчас это неразличимо, потому что текущий ingress anchor в payload собирается внутри `RequestContextService::call`.

## Цели
- Дать следующий bounded ingress cut до `RequestContextService::call`.
- Сохранить additive contract discipline и trustworthy semantics `v9`.
- Не заставлять оператора читать runtime code, чтобы интерпретировать ingress anchor.
- Сохранить operator-facing surfaces self-contained и человекочитаемыми.

## Не-цели
- Не обещать финальный root cause внутри tower/jsonrpc/executor.
- Не добавлять unbounded event history.
- Не менять completion результат, handler routing или observability probe schema.
- Не подменять authoritative payload клиентской реконструкцией.

## Решение

### 1. Outer ingress timestamp до `RequestContextService`
Нужен новый bounded hook вокруг LSP service до `RequestContextService::new(service)`.

Этот hook должен фиксировать момент, когда request уже попал в jsonrpc/tower dispatch path, но ещё не вошёл в `RequestContextService::call`.

Из этого hook authoritative producer path сможет получить:
- optional `jsonrpc_dispatch_received_at_ms`;
- optional `dispatch_to_request_context_wait_ms`.

### 2. Legacy ingress anchor остаётся читаемым только вместе с provenance
Новый contract должен явно фиксировать, что `transport_received_at_ms` без provenance больше нельзя интерпретировать "по имени поля".

Поэтому `server_edge_details` должен включать bounded `transport_received_at_ms_provenance` со значениями:
- `request_context_call_entry`;
- `jsonrpc_dispatch_received`.

Если outer dispatch timestamp доступен и используется как authoritative ingress anchor, payload MUST:
- выставлять `transport_received_at_ms_provenance=jsonrpc_dispatch_received`;
- включать `jsonrpc_dispatch_received_at_ms`;
- включать `dispatch_to_request_context_wait_ms`.

Если outer dispatch timestamp недоступен, payload MUST:
- сохранять honest fallback `transport_received_at_ms_provenance=request_context_call_entry`;
- не выдумывать `jsonrpc_dispatch_received_at_ms`;
- не выдумывать `dispatch_to_request_context_wait_ms`.

Дублирование `transport_received_at_ms` и `jsonrpc_dispatch_received_at_ms` допустимо как осознанный trade-off:
- existing readers продолжают читать старый ingress anchor;
- новые readers и incident bundle получают явный raw field для outer dispatch split;
- provenance не остаётся implicit knowledge из runtime implementation.

### 3. `v10` остаётся additive и bounded
Новый contract нужен, потому что меняется authoritative payload shape и interpretation rules.

При этом:
- existing `v9` fields сохраняются;
- новые поля bounded и optional;
- free-text explanation, stack traces и high-cardinality identifiers не добавляются.

Репозиторий должен поддерживать contiguous versioned contract baseline:
- `contracts/lsp-completion-timeline/v7` для shipped `response.version=10`;
- `contracts/lsp-completion-timeline/v6` остаётся baseline для `response.version=9`.

### 4. Existing completion surfaces
Panel, clipboard и request-centric incident bundle summary должны:
- показывать dispatch-to-request-context split отдельными fact lines;
- сохранять уже существующий `v9` pre-service-scope split;
- не выдумывать outer dispatch fields на `v9`;
- явно называть limitation на `v9`, а не скрывать её в нейтральном summary.

Новый change не требует новый verdict taxonomy. Достаточно, чтобы human-readable output позволял отличить:
- lag до входа в `RequestContextService::call`;
- lag после `RequestContextService::call`, но до `service_future_created`;
- lag после `service_future_created`, но до первого poll request scope.

## Риски и trade-offs

### Риск: новый split всё ещё не докажет, где именно тормозит tower/jsonrpc
Смягчение:
- change не обещает финальный root cause;
- он обещает ещё один bounded narrowing cut, достаточный для следующего цикла расследования.

### Риск: outer dispatch hook будет доступен не во всех runtime path
Смягчение:
- поля optional;
- provenance обязателен;
- отсутствие outer timestamp не реконструируется из других timestamp'ов.

### Риск: operator-facing surfaces начнут смешивать `v9` и `v10` semantics
Смягчение:
- change требует explicit `v9` degradation note;
- panel, clipboard и incident bundle обновляются вместе с contract;
- smoke/runbook ожидания обновляются одновременно.

### Риск: `transport_received_at_ms` останется misleading без provenance
Смягчение:
- `transport_received_at_ms_provenance` становится обязательным для `v10`;
- docs/spec/runbook должны проговаривать, какой ingress anchor видит оператор.

## Acceptance-направление
- Trace с большим ingress wait показывает, сидит ли lag до `RequestContextService::call` или уже после middleware entry.
- `v10` payload сохраняет existing `v9` trustworthy semantics и bounded discipline.
- Panel / clipboard / incident bundle переносят новый split без invented reconstruction.
- `v9` payload деградирует явно и не маскирует missing dispatch split как отсутствие gaps.
