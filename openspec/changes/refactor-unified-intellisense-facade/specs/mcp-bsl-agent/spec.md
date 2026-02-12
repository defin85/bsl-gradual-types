## ADDED Requirements

### Requirement: `bsl-agent` semantic tools используют общий v2 facade/runtime (MUST)
Система MUST выполнять production semantic tools `bsl-agent` через тот же shared v2 facade/runtime контракт, который используется в LSP и web.

В рамках этого требования migration MUST быть полной (без частичного MVP): после завершения change в `bsl-agent` не должно оставаться production semantic tools с ad-hoc orchestration через локально собранный `AnalysisHostV2` pipeline.

#### Scenario: Semantic tools MCP выполняются через shared orchestration path
- **GIVEN** активная ready-сессия `bsl-agent`
- **WHEN** клиент вызывает `bsl_diagnostics`, `bsl_type_at_position`, `bsl_members` и `bsl_definition`
- **THEN** все операции выполняются через общий facade/runtime path
- **AND** поведение cancellation/performance policy совпадает с контрактом LSP/web

### Requirement: MCP observability метрики semantic pipeline согласованы с LSP (MUST)
Система MUST обеспечивать, что `workspace_get_observability_metrics` для semantic операций, выполненных через shared facade, использует тот же stage-level контракт метрик, что и LSP observability.

Согласование MUST включать одинаковую классификацию outcome/cancellation для эквивалентных semantic стадий.

#### Scenario: Метрики semantic стадий совпадают по контракту между MCP и LSP
- **GIVEN** одинаковый сценарий semantic запросов выполняется через LSP и MCP
- **WHEN** клиент получает observability snapshot из обоих интерфейсов
- **THEN** stage-level метрики интерпретируются по одному контракту
- **AND** различия ограничены источником запроса, а не логикой стадии
