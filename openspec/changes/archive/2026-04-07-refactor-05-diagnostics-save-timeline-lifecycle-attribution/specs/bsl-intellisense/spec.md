## MODIFIED Requirements
### Requirement: Incident bundle summary показывает didSave refresh как request-centric diagnostics cycle (MUST)
`summary.md` и `incident.json` MUST переносить diagnostics save timeline в человекочитаемом request-centric виде.

Human-readable projection MUST:

- показывать `uri`, `requested_version` и bounded first-publish outcome;
- различать `save_fastlane` first publish и `idle_heavy` follow-up;
- показывать, был ли first publish `syntax_only` или `full`;
- не переименовывать aggregate metrics p95/p99 в request-level факты.

Дополнительно projection MUST:

- явно различать active `in_flight` cycles и terminal cycles;
- не рендерить pending profile facts для active cycle как `unknown`, если lifecycle уже известен.

#### Scenario: Summary показывает first publish и follow-up без guesswork
- **GIVEN** diagnostics save timeline trace содержит `save_fastlane` first publish и `idle_heavy` follow-up
- **WHEN** extension формирует `summary.md`
- **THEN** summary показывает оба bounded факта внутри одного save refresh cycle
- **AND** оператор может отличить first freshness boundary от final richer publish

#### Scenario: Summary не заменяет request trace cumulative histogram-ом
- **GIVEN** bundle содержит и diagnostics save timeline, и cumulative observability metrics
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary использует authoritative diagnostics save trace для request-level фактов
- **AND** cumulative metrics остаются только snapshot supplement

#### Scenario: Summary помечает active save cycle как in_flight
- **GIVEN** bundle содержит active diagnostics save cycle без terminal outcome
- **WHEN** extension рендерит human-readable summary
- **THEN** cycle помечается как `in_flight`
- **AND** pending profile outcome рендерится как `pending`, а не `unknown`
