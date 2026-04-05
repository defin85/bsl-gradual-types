## ADDED Requirements

### Requirement: Same-file save-triggered auxiliary churn does not regress current-revision readiness fast lane (MUST)
После того как current-revision handoff для requested revision уже зарегистрирован через `didOpen` или `didChange`, same-file `didSave`-triggered refresh и другой auxiliary same-file churn MAY продолжаться в фоне, но MUST NOT возвращать interactive completion к `prepare_timeout@wait_for_file_version`, если truthful transport seams уже остаются в интерактивном бюджете.

Для этого requirement readiness regression считается отдельным failure mode:
- bounded wait на `wait_for_file_version` не должен исчерпываться только потому, что newest same-file readiness все еще стоит позади save-triggered auxiliary backlog;
- healthy `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms` MUST NOT использоваться как оправдание для `prepare_timeout@wait_for_file_version`;
- cold semantic/query-body latency после успешного readiness рассматривается отдельно и не считается объяснением readiness timeout.

#### Scenario: Same-file save refresh не держит newest completion в `wait_for_file_version`
- **GIVEN** `didChange` уже зарегистрировал current-revision handoff для requested revision `V`
- **AND** same-file `didSave` или другой auxiliary refresh запускает дополнительную background работу для того же файла
- **WHEN** IDE запрашивает completion для revision `V`
- **THEN** readiness fast lane не деградирует в `prepare_timeout@wait_for_file_version` только из-за этого same-file auxiliary backlog
- **AND** completion либо получает current-revision first response, либо завершается по другой truthful причине, не связанной с post-handoff `wait_for_file_version` starvation

#### Scenario: Healthy truthful seams не маскируют readiness timeout
- **GIVEN** representative completion sample показывает `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms` внутри интерактивного бюджета
- **WHEN** тот же sample все равно завершает prepare как `prepare_timeout@wait_for_file_version`
- **THEN** outcome считается current-revision readiness regression
- **AND** не считается допустимым bounded fail-closed поведением

### Requirement: Representative post-edit/save churn gate separates readiness regressions from cold query-body cost (MUST)
Representative real-module acceptance для current-revision completion MUST иметь отдельный post-edit/save churn profile, который проверяет readiness fast lane независимо от latency дальнейшего semantic/query-body execution.

Этот gate MUST:
- использовать same-file профиль `didChange + didSave + auxiliary same-file noise + completion` на representative large-module fixture;
- собирать truthful transport/readiness fields как минимум `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms`, `response_output_handoff_send_wait_ms` и `prepare_timeout` phase/cause;
- fail-ить, если measured sample получает `prepare_timeout@wait_for_file_version` при truthful transport seams внутри бюджета;
- report-ить cold `query_bundle_ir_query` / `collect` latency отдельным diagnostic bucket после успешного readiness, а не как оправдание readiness failure.

#### Scenario: Gate отдельно ловит readiness timeout и отдельно cold query-body
- **GIVEN** representative same-file post-edit/save churn profile на real module
- **AND** truthful transport seams measured sample остаются в бюджете
- **WHEN** один sample завершает prepare как `prepare_timeout@wait_for_file_version`, а другой sample успешно проходит readiness и тратит время в `query_bundle_ir_query`
- **THEN** gate завершается ошибкой из-за readiness timeout sample
- **AND** cold query-body latency отражается отдельным diagnostic signal, а не как причина acceptance failure по readiness contract
