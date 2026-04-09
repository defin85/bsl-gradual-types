## MODIFIED Requirements
### Requirement: Same-file save-triggered auxiliary churn does not regress current-revision readiness fast lane (MUST)
После того как current-revision handoff для requested revision уже зарегистрирован через `didOpen` или `didChange`, same-file `didSave`-triggered refresh и другой auxiliary same-file churn MAY продолжаться в фоне, но MUST NOT возвращать interactive completion к `prepare_timeout@wait_for_file_version`, если truthful transport seams уже остаются в интерактивном бюджете.

Для этого requirement readiness regression считается отдельным failure mode:
- bounded wait на `wait_for_file_version` не должен исчерпываться только потому, что newest same-file readiness все еще стоит позади save-triggered auxiliary backlog;
- readiness waiter registration MUST становиться observable без seconds-scale residence в generic background writer/runtime FIFO до самого факта passive waiting;
- passive readiness waiting MUST NOT требовать дополнительных blocking CPU permits только ради наблюдения за requested revision;
- healthy `client_to_transport_wait_ms`, `service_future_to_first_poll_wait_ms` и `response_output_handoff_send_wait_ms` MUST NOT использоваться как оправдание для `prepare_timeout@wait_for_file_version`;
- cold semantic/query-body latency после успешного readiness рассматривается отдельно и не считается объяснением readiness timeout.

#### Scenario: Same-file save refresh не держит newest completion в `wait_for_file_version`
- **GIVEN** `didChange` уже зарегистрировал current-revision handoff для requested revision `V`
- **AND** same-file `didSave` или другой auxiliary refresh запускает дополнительную background работу для того же файла
- **WHEN** IDE запрашивает completion для revision `V`
- **THEN** readiness fast lane не деградирует в `prepare_timeout@wait_for_file_version` только из-за этого same-file auxiliary backlog
- **AND** completion либо получает current-revision first response, либо завершается по другой truthful причине, не связанной с post-handoff `wait_for_file_version` starvation

#### Scenario: Waiter registration не сидит за unrelated apply backlog перед passive wait
- **GIVEN** writer/runtime уже обрабатывает unrelated apply backlog для того же или другого файла
- **AND** интерактивный completion request должен дождаться requested revision `V`
- **WHEN** request переходит к readiness observation
- **THEN** request становится passive waiter без seconds-scale residency в generic background FIFO до регистрации wait
- **AND** дальнейшая latency truthfully атрибутируется либо actual apply lag, либо другой readiness cause, а не raw registration backlog

### Requirement: didSave heavy follow-up избегает apply-lag как primary gate (MUST)
После successful same-version `save_fastlane` first publish система MUST стремиться к richer heavy follow-up того же `save_cycle_sequence` без unbounded зависимости от writer/apply lag как primary gate, если same-version ready artifacts уже доступны.

Система MAY использовать writer-owned applied state, когда он уже готов, но MUST:

- предпочитать same-version ready artifacts поверх blind `wait_for_file_version`;
- переиспользовать same-version syntax artifacts в `didSave + idle_heavy`, если их freshness доказана для данного save cycle;
- truthfully fall back to syntax recompute, когда reuse невозможно или stale;
- не публиковать older-version diagnostics;
- сохранять supersession semantics для newer save cycles;
- если follow-up всё же ждёт requested applied revision или semantically equivalent ready state, регистрация этого ожидания MUST происходить через low-latency passive readiness path, а не через seconds-scale generic runtime FIFO residency до becoming observable waiter;
- сохранять request-centric distinction между passive readiness wait, actual apply/writer execution contention и downstream semantic work.

#### Scenario: delayed apply не держит heavy follow-up hostage при наличии ready save artifacts
- **GIVEN** `didSave` already materialized same-version ready artifacts
- **AND** writer apply path всё ещё отстаёт
- **WHEN** heavy follow-up пытается построить richer diagnostics
- **THEN** система не использует unbounded apply-lag как primary gating step
- **AND** либо публикует richer follow-up, либо truthful trace attribution показывает residual blocker

#### Scenario: Follow-up readiness wait регистрируется до seconds-scale generic FIFO residency
- **GIVEN** `didSave` cycle уже прошёл bounded first publish
- **AND** richer follow-up должен дождаться requested applied revision
- **WHEN** follow-up переходит к readiness observation
- **THEN** он становится passive waiter без seconds-scale generic runtime FIFO residency до самой регистрации wait
- **AND** trace отдельно показывает passive wait и actual apply/writer contention вместо смешанного residual tail
