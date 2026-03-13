## ADDED Requirements

### Requirement: Change completion MUST NOT завышать readiness относительно MUST backlog
Система MUST иметь readiness gate, который запрещает считать OpenSpec change фактически complete, если по его MUST-требованиям остаётся открытый критический follow-up backlog.

Gate MUST сверять как минимум:
- статус checklist / validation в change;
- traceability matrix;
- review-gate verdict или эквивалентный audit artifact;
- связанный критический Beads backlog, созданный для закрытия тех же MUST-требований.

Если критический follow-up backlog существует, change MUST быть явно помечен как `partial`, `not ready` или эквивалентно незавершённый до закрытия этого backlog либо до утверждённого superseding delivery path.

#### Scenario: Open follow-up epic блокирует честный verdict `complete`
- **GIVEN** review change выявил недоставленные MUST-требования
- **AND** для них создан критический Beads epic/task graph
- **WHEN** команда пытается считать исходный change complete только по checklist и validation
- **THEN** readiness gate отклоняет verdict `complete`
- **AND** требует явного partial/not-ready статуса или approved superseding delivery path

### Requirement: Traceability и review artifacts MUST отражать реальные gaps без optimistic overclaim (MUST)
Traceability matrix, review-gate и связанные acceptance artifacts MUST отражать реальный статус MUST-требований без optimistic overclaim.

Если evidence показывает `partial` или `gap`, артефакты MUST NOT маркировать требование как `covered` или `pass` без дополнительного подтверждённого delivery evidence.

#### Scenario: Conflicting evidence не допускает optimistic `covered`
- **GIVEN** traceability или review artifact утверждает `covered/pass`
- **AND** другой approved evidence artifact показывает открытый gap по тому же MUST-требованию
- **WHEN** readiness gate сверяет evidence
- **THEN** optimistic verdict отклоняется
- **AND** artefact должен быть исправлен до handoff или archive
