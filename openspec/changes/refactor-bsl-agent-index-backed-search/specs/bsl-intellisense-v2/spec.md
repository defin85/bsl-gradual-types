## ADDED Requirements
### Requirement: Shared runtime предоставляет index-backed search contract для adapters (MUST)
v2 runtime MUST предоставлять adapter-agnostic search contract поверх `IndexSnapshot` для type/symbol/references queries, чтобы MCP и LSP использовали единый источник поисковых candidates и единые правила детерминированной сортировки.

Контракт MUST быть revision-bound и MUST поддерживать merge с overlay-изменениями активной сессии.

Связанный capability: `mcp-bsl-agent` (MCP tools migration на shared search path).

#### Scenario: MCP и LSP получают согласованный candidate set из одного runtime snapshot
- **GIVEN** один и тот же runtime snapshot и одинаковый search query
- **WHEN** MCP-adapter и LSP-adapter вызывают shared runtime search contract
- **THEN** оба адаптера получают согласованный candidate set и детерминированный порядок
- **AND** различие ответов (если есть) объясняется только adapter-level payload shaping

#### Scenario: Overlay revision обновляет effective search state без полной пересборки всего workspace
- **GIVEN** в сессии применён `documents_set` для subset файлов
- **WHEN** выполняется следующий search query через shared runtime contract
- **THEN** search учитывает effective overlay state для последней revision
- **AND** не требует полного workspace scan как обязательного пути
