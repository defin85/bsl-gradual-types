## ADDED Requirements

### Requirement: Immediate same-file front-edge completion does not regress into hidden `exact_deadline` (MUST)
После того как same-file current-revision handoff уже зарегистрирован через `didChange` или `didSave`, первые interactive completion requests в immediate post-edit/save window MUST NOT исчерпывать bounded `wait_exact_type_index` и затем завершаться generic fail-closed outcome только потому, что exact current-revision artifact ещё не стал наблюдаемым, если truthful transport seams остаются в интерактивном бюджете.

Для этого requirement:
- `wait_exact_type_index` exhaustion с `type_index_wait_outcome=deadline` в том же front-edge окне считается unresolved readiness regression;
- такой outcome не должен маскироваться под generic `missing_semantic_index` без explicit regression attribution.

#### Scenario: Front-edge exact wait deadline не маскируется как generic availability miss
- **GIVEN** `didChange` или `didSave` уже зарегистрировал same-file handoff для revision `V`
- **AND** completion request входит в immediate post-edit/save window почти сразу после handoff
- **WHEN** truthful transport seams остаются в интерактивном бюджете
- **THEN** completion не завершает front-edge path с hidden `wait_exact_type_index=deadline`
- **AND** operator-facing evidence не схлопывает такой regression в generic `missing_semantic_index` без отдельной attribution

### Requirement: Representative front-edge gate requires successful current-revision sample before separating cold `query_bundle_pool_wait` (MUST)
Representative real-module acceptance для current-revision completion MUST считать remediation незавершённой, если immediate post-edit/save front-edge profile не даёт ни одного successful current-revision sample, даже когда `prepare_timeout` уже устранён.

Этот gate MUST:
- использовать same-file профиль `didChange + didSave + immediate completion burst` на representative large-module fixture;
- fail-ить на любом front-edge `prepare_timeout` или hidden `exact_deadline` при healthy truthful transport seams;
- требовать как минимум один successful current-revision sample в measured front-edge window;
- report-ить cold `query_bundle_pool_wait` отдельным diagnostic bucket только для successful samples после readiness.

#### Scenario: Gate не проходит на all-fail-closed front-edge profile
- **GIVEN** representative immediate post-edit/save front-edge profile на real module
- **AND** truthful transport seams measured samples остаются в бюджете
- **WHEN** measured samples не содержат `prepare_timeout`, но все measured traces завершаются fail-closed до successful current-revision response
- **THEN** gate завершается ошибкой
- **AND** remediation не считается завершённой только на основании bounded fail-closed outcomes
