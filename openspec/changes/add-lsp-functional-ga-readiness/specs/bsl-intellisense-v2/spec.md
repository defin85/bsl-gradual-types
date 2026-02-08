## ADDED Requirements

### Requirement: v2 snapshot обеспечивает сквозную согласованность implicit symbols и value-table schema-effects (MUST)
В рамках одного и того же v2 snapshot система MUST обеспечивать согласованный результат для:
- резолва context implicit symbols,
- schema-effects `ТаблицаЗначений.Колонки.Добавить`,
во всех v2 consumers (semantic diagnostics, completion, hover/type-at-position).

Это требование является quality-контрактом верхнего уровня и SHOULD реализовываться поверх детальных правил, определенных отдельными change:
- `update-v2-contextual-implicit-variables`,
- `add-v2-valuetable-column-resolution`.

#### Scenario: Один snapshot даёт одинаковый смысл для всех IDE операций
- **GIVEN** документ с context implicit symbols и typed `ТаблицаЗначений` колонками
- **WHEN** сервер обрабатывает completion, hover и diagnostics в одной ревизии документа
- **THEN** все операции используют один и тот же смысл символов/колонок без взаимных противоречий
