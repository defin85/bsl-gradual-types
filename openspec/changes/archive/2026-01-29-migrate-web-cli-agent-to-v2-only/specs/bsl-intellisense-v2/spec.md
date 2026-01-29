## ADDED Requirements

### Requirement: Non-LSP клиенты используют v2-only источники данных (MUST)
Система MUST обеспечивать, что Web API, CLI и `bsl-agent` используют v2-only источники данных
и не имеют альтернативных inference путей вне `bsl-analysis-v2`/deps snapshot.

#### Scenario: Web API не использует отдельный inference фасад
- **GIVEN** пользователь вызывает Web API эндпоинт (search/types/details/etc.)
- **WHEN** сервер формирует ответ
- **THEN** сервер использует только v2 deps snapshot (`SemanticDeps`) и/или v2 queries
- **AND** в коде Web API нет использования `TypeInferenceService`

#### Scenario: CLI не использует legacy AnalysisEngine
- **GIVEN** пользователь запускает CLI команду анализа
- **WHEN** CLI вычисляет diagnostics/семантику
- **THEN** используется `AnalysisHostV2`/`AnalysisV2` и v2 queries
- **AND** CLI не использует `bsl_shared::engine::AnalysisEngine`

#### Scenario: bsl-agent не использует отдельный inference фасад
- **GIVEN** клиент вызывает операции агента, требующие типовой информации
- **WHEN** агент вычисляет ответ
- **THEN** используется v2 deps snapshot (`SemanticDeps`) и/или v2 queries
- **AND** в коде агента нет использования `TypeInferenceService`
