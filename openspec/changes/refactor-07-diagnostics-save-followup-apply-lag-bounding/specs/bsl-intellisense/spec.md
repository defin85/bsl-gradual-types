## MODIFIED Requirements
### Requirement: Incident bundle экспортирует authoritative diagnostics save timeline (MUST)
Incident bundle MUST экспортировать diagnostics save timeline как отдельный authoritative source,
а не реконструировать его из aggregate metrics.

Diagnostics save section MUST:

- сохранять `save_cycle_sequence` рядом с `requested_version` и `diagnostics_generation`;
- рендерить operator-facing save ordering через `save_cycle_sequence`;
- объяснять active heavy follow-up через explicit request-centric wait reason, если сервер его уже
  знает.

#### Scenario: summary показывает причину stalled heavy follow-up
- **GIVEN** `didSave` cycle уже имеет first publish, но heavy follow-up ещё не завершён
- **WHEN** summary строит diagnostics save section
- **THEN** он показывает explicit follow-up wait reason
- **AND** не оставляет operator workflow на одном только `pending`
