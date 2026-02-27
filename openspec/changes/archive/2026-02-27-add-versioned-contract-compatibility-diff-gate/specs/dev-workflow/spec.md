## ADDED Requirements

### Requirement: Versioned contracts проходят compatibility-diff проверку как manual gate (MUST)
Система MUST иметь compatibility-diff проверку для `contracts/**`, которая сравнивает baseline и candidate версии контрактов на semantic совместимость.

Проверка MUST:
- классифицировать изменения как `non_breaking` или `breaking` по формальной policy;
- выдавать machine-readable отчёт (`pass/fail`, `violations`, `compared_versions`);
- запускаться в manual режиме (`workflow_dispatch`/ручная команда) на текущем этапе rollout.

#### Scenario: Manual compatibility-diff gate формирует детерминированный отчёт
- **GIVEN** разработчик меняет контракт в `contracts/<surface>/vN/...`
- **WHEN** запускается manual compatibility-diff gate
- **THEN** система формирует детерминированный отчёт с классификацией изменений
- **AND** отчёт содержит `pass/fail` и список нарушений policy

### Requirement: Breaking compatibility diff требует major bump и migration note (MUST)
Если compatibility-diff классифицирует изменение как `breaking`, система MUST требовать major bump (`vN -> vN+1`).

Если major bump выполнен, система MUST требовать migration note в `contracts/<surface>/vN/changelog.md`.

#### Scenario: Breaking изменение без major bump отклоняется
- **GIVEN** baseline и candidate контракт имеют breaking diff
- **WHEN** major версия не увеличена
- **THEN** compatibility-diff gate завершается fail
- **AND** отчёт явно указывает причину: `breaking_without_major_bump`

#### Scenario: Major bump без migration note отклоняется
- **GIVEN** для contract surface выполнен major bump
- **WHEN** в `changelog.md` отсутствует migration note
- **THEN** compatibility-diff gate завершается fail
- **AND** отчёт явно указывает причину: `missing_migration_note`
