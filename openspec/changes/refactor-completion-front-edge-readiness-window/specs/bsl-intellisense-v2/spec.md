## ADDED Requirements

### Requirement: Immediate same-file post-edit/save completion window does not regress into `prepare_timeout` (MUST)
После того как same-file current-revision handoff уже зарегистрирован через `didChange` или `didSave`, первые interactive completion requests в immediate post-edit/save window MUST NOT завершаться `prepare_timeout` только потому, что fully prepared current-revision path ещё не стала наблюдаемой на request path, если truthful transport seams остаются в интерактивном бюджете.

Для этого requirement front-edge regression surface включает:
- `prepare_timeout@wait_for_file_version`;
- `prepare_timeout@snapshot_with_deps`, если timeout происходит в том же immediate post-edit/save window и не объясняется transport ingress/output backlog.

#### Scenario: Same-file front-edge completion не умирает на `wait_for_file_version`
- **GIVEN** `didChange` или `didSave` уже зарегистрировал same-file handoff для revision `V`
- **AND** IDE запрашивает completion почти сразу после этого handoff
- **WHEN** truthful transport seams остаются в интерактивном бюджете
- **THEN** completion не завершается `prepare_timeout@wait_for_file_version` только из-за front-edge readiness lag
- **AND** outcome либо остаётся bounded current-revision first response, либо завершается по другой truthful причине, не связанной с front-edge starvation

#### Scenario: Same-file front-edge completion не маскирует timeout на `snapshot_with_deps`
- **GIVEN** same-file handoff для revision `V` уже зарегистрирован
- **AND** `wait_for_file_version` уже не объясняет timeout
- **WHEN** completion всё равно исчерпывает prepare budget на `snapshot_with_deps` в immediate post-edit/save window
- **THEN** такой outcome считается front-edge readiness regression
- **AND** не считается допустимым bounded fail-closed поведением

### Requirement: Representative front-edge gate separates immediate `prepare_timeout` regressions from cold `query_bundle_pool_wait` (MUST)
Representative real-module acceptance для current-revision completion MUST иметь отдельный immediate post-edit/save front-edge profile, который проверяет первые same-file completion samples сразу после handoff независимо от downstream cold query-body latency.

Этот gate MUST:
- использовать same-file профиль `didChange + didSave + immediate completion burst` на representative large-module fixture;
- собирать truthful transport/readiness fields как минимум `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms`, `response_output_handoff_send_wait_ms`, `fail_closed_cause` и `timeout_attribution.phase`;
- fail-ить на любом `prepare_timeout` в front-edge samples при healthy truthful transport seams;
- report-ить successful samples с cold `query_bundle_pool_wait` отдельным diagnostic bucket после успешного readiness.

#### Scenario: Gate валится на front-edge timeout и отдельно отражает downstream pool wait
- **GIVEN** representative immediate post-edit/save front-edge profile на real module
- **AND** truthful transport seams measured samples остаются в бюджете
- **WHEN** один sample завершается `prepare_timeout`, а другой sample успешно проходит readiness и тратит время в `query_bundle_pool_wait`
- **THEN** gate завершается ошибкой из-за front-edge `prepare_timeout`
- **AND** `query_bundle_pool_wait` отражается отдельным diagnostic signal, а не объяснением readiness failure
