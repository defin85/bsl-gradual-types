## ADDED Requirements

### Requirement: Временный `didSave` exact-wait relief valve является evidence-gated и self-attributing (MUST)

Система MUST ограничивать любое временное дополнительное bounded wait window поверх базового
`didSave` ready-snapshot wait budget только случаями, где runtime может доказать, что:

- ожидание идёт на exact still-current producer для matching
  `(file_id, requested_version, text_hash)`;
- producer не был retargeted/coalesced away;
- наблюдаемый blocker не объясняется runtime queue wait или apply lag;
- exact-path phase attribution показывает late exact readiness, а не generic fallback path.

Если это доказательство отсутствует, система MUST сохранить текущее базовое bounded wait behavior
и MUST перейти к существующему truthful fallback без дополнительного wait window.

Использование временного relief valve MUST оставаться строго bounded, MUST быть явно отражено в
observability / incident bundle export и MUST различать как минимум:

- valve engaged and helped;
- valve skipped because proof was absent;
- valve engaged but still timed out.

#### Scenario: Late exact producer успевает в дополнительное временное окно

- **GIVEN** базовый `didSave` bounded wait исчерпан
- **AND** runtime всё ещё видит тот же exact still-current producer
- **AND** phase attribution показывает late exact readiness без queue/apply-lag признаков
- **WHEN** включён временный relief valve
- **THEN** runtime MAY ждать только в пределах дополнительного bounded relief window
- **AND** если producer materializes внутри этого окна, publish идёт через `ready_artifacts`
- **AND** bundle явно показывает, что relief valve был задействован

#### Scenario: Queue/apply-lag или coalesced-away producer не получают relief window

- **GIVEN** базовый `didSave` bounded wait исчерпан
- **AND** runtime видит apply lag, runtime queue wait или producer уже retargeted/coalesced away
- **WHEN** heavy follow-up выбирает дальнейший путь
- **THEN** runtime MUST NOT включать дополнительное relief wait window
- **AND** использует существующий truthful fallback / attribution path
