## ADDED Requirements
### Requirement: Auxiliary `documentSymbol` traffic не starving interactive semantic admission (MUST)
Система MUST рассматривать `textDocument/documentSymbol` как auxiliary IDE companion request и MUST изолировать его admission/execution path от interactive semantic запросов (`completion`, `hover`, `signatureHelp`, `definition`).

Изоляция MUST обеспечивать:
- outstanding `documentSymbol` refresh не задерживает первый `poll()` interactive запроса из-за strict current-version wait;
- auxiliary path не потребляет interactive reserve при наличии interactive waiters;
- same-file newer `documentSymbol` refresh MAY supersede older outstanding refresh, если старый ещё не принёс user-visible value;
- `documentSymbol` outcome (`current_ready`, `latest_ready`, `unavailable`, `superseded`) не влияет на strict current-revision contract interactive semantic ответов.

#### Scenario: Outline refresh не блокирует completion ingress
- **GIVEN** для того же файла одновременно идут `didChange`/`didSave` churn и refresh Outline через `textDocument/documentSymbol`
- **AND** пользователь запрашивает member-access completion
- **WHEN** сервер обрабатывает mixed load
- **THEN** completion получает first `poll()` без ожидания завершения outstanding `documentSymbol` current-version wait
- **AND** `documentSymbol` обслуживается как auxiliary outcome, а не как gate для interactive completion

#### Scenario: Более новый outline refresh supersede-ит старый
- **GIVEN** для одного `file_id` в очереди уже есть outstanding `documentSymbol` refresh
- **AND** приходит более новый `documentSymbol` refresh после следующего `didChange`
- **WHEN** сервер выбирает, какой refresh исполнять
- **THEN** older refresh может быть superseded в пользу newest refresh
- **AND** supersession фиксируется как явный auxiliary outcome

### Requirement: Mixed-load gate детерминированно ловит outline-induced starvation (MUST)
Система MUST иметь representative live gate, который прогоняет same-file real-module mixed load из:
- `didChange`/`didSave`;
- `textDocument/documentSymbol`;
- `textDocument/completion`.

Gate MUST собирать authoritative server-side evidence минимум по:
- completion `service_future_to_first_poll_wait_ms`;
- completion `transport_to_handler_wait_ms`;
- completion route/outcome;
- `documentSymbol` outcome class (`current_ready`, `latest_ready`, `unavailable`, `superseded`).

Gate MUST fail:
- если `p95(service_future_to_first_poll_wait_ms)` у measured completion samples выше `intellisense_v2_interactive_wait_budget_ms`;
- если любой measured completion sample имеет `service_future_to_first_poll_wait_ms > 4 * intellisense_v2_interactive_wait_budget_ms`;
- если measured completion sample становится ingress-dominant из-за concurrent auxiliary `documentSymbol` load.

#### Scenario: Representative gate падает при starvation от outline traffic
- **GIVEN** real-module mixed-load profile с active `documentSymbol` refresh и completion на том же файле
- **WHEN** auxiliary outline path снова начинает удерживать interactive completion до входа в handler
- **THEN** representative gate завершается ошибкой
- **AND** evidence указывает на concurrent outline outcome/load, а не маскирует regression как generic completion slowdown
