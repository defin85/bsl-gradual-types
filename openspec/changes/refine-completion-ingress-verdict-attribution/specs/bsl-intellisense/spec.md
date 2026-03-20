## ADDED Requirements
### Requirement: Incident bundle findings агрегируют ingress verdicts truthfully (MUST)
`incident.json` и `summary.md` MUST агрегировать ingress-related findings только из truthful positive-only verdicts и MUST NOT переоценивать ingress bottleneck на hot traces, где положительный ingress wait отсутствует.

Request-centric bundle summary MUST:
- использовать тот же смысл ingress verdicts, что и другие completion projections extension;
- считать client-side и server-side ingress отдельно, если соответствующие verdicts доступны;
- не формулировать общий ingress bottleneck только на основании traces с нулевыми ingress waits;
- сохранять request summary валидным, даже если client correlation unavailable.

#### Scenario: Summary не переоценивает hot traces как ingress bottleneck
- **GIVEN** capture window содержит hot completion trace с нулевыми ingress/prelude waits
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** этот trace не учитывается в ingress findings
- **AND** summary не заявляет ingress bottleneck для него

#### Scenario: Summary различает client-side и server-side ingress
- **GIVEN** capture window содержит как минимум один correlated trace с доминирующим `client_to_transport_wait_ms`
- **AND** содержит trace с доминирующим `transport_to_method_wait_ms`
- **WHEN** extension формирует derived request-centric summary
- **THEN** findings и request entries различают client-side и server-side ingress verdicts
- **AND** оператору не нужно открывать raw JSON, чтобы увидеть этот split

#### Scenario: Correlation gap не превращается в guessed ingress finding
- **GIVEN** request summary не имеет deterministic client correlation
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** summary не создаёт client-side ingress finding для такого request
- **AND** request остаётся server-centric или без ingress finding, если положительный server-side ingress wait отсутствует
