## MODIFIED Requirements
### Requirement: Incident bundle экспортирует authoritative diagnostics save timeline (MUST)
Incident bundle MUST экспортировать diagnostics save timeline как отдельный authoritative source,
а не реконструировать его из aggregate metrics.

Diagnostics save section MUST:

- сохранять `save_cycle_sequence` рядом с `requested_version` и `diagnostics_generation`;
- рендерить operator-facing save ordering через `save_cycle_sequence`;
- не подменять save-cycle identity `diagnostics_generation`, если они расходятся по смыслу.

#### Scenario: summary различает два save-cycle одного requested_version
- **GIVEN** в bundle есть два `didSave` traces для одного `requested_version`
- **WHEN** summary строит diagnostics save section
- **THEN** он показывает distinct `save_cycle_sequence`
- **AND** не требует читать save ordering через `diagnostics_generation`
