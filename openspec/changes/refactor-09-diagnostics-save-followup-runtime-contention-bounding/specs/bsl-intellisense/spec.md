## MODIFIED Requirements
### Requirement: Incident bundle summary показывает didSave refresh как request-centric diagnostics cycle (MUST)
`summary.md` и `incident.json` MUST переносить diagnostics save timeline в человекочитаемом request-centric виде.

Human-readable projection MUST:

- показывать `uri`, `requested_version` и bounded first-publish outcome;
- сохранять `save_cycle_sequence` рядом с `requested_version` и `diagnostics_generation`;
- различать `save_fastlane` first publish и `idle_heavy` follow-up;
- показывать, был ли first publish `syntax_only` или `full`;
- не переименовывать aggregate metrics p95/p99 в request-level факты.

Дополнительно projection MUST:

- рендерить operator-facing save ordering через `save_cycle_sequence`, а не через `diagnostics_generation`, если два save-cycle делят один `requested_version`;
- явно различать active `in_flight` cycles и terminal cycles;
- не рендерить pending profile facts для active cycle как `unknown`, если lifecycle уже известен;
- объяснять active heavy follow-up через explicit request-centric wait reason, если сервер его уже знает;
- показывать request-centric follow-up runtime/apply breakdown, когда backend уже публикует эти facts, вместо того чтобы оставлять оператору только общий follow-up elapsed и cumulative histograms.

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

#### Scenario: Summary различает два save-cycle одного requested_version
- **GIVEN** в bundle есть два `didSave` traces для одного `requested_version`
- **WHEN** summary строит diagnostics save section
- **THEN** он показывает distinct `save_cycle_sequence`
- **AND** не требует читать save ordering через `diagnostics_generation`

#### Scenario: Summary показывает runtime/apply breakdown тяжелого follow-up
- **GIVEN** diagnostics save timeline trace уже содержит request-centric follow-up runtime/apply facts
- **WHEN** summary строит diagnostics save section
- **THEN** он показывает runtime queue wait, apply/writer contention, `wait_for_file_version` и semantic work по trace, когда они доступны
- **AND** не заставляет оператора выводить эти причины только из cumulative p95/p99 histograms
