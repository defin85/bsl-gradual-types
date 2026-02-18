## ADDED Requirements

### Requirement: MCP observability drilldown semantic pipeline согласован с LSP по operation/stage контракту (MUST)
Система MUST обеспечивать, что `workspace_get_observability_metrics` в `bsl-agent` содержит тот же drilldown-контракт semantic pipeline, что и LSP:
- одинаковые `operation`/`stage`/`outcome`/`reason` классификаторы;
- одинаковую интерпретацию cancellation/skip причин;
- различие между интерфейсами допускается только по `origin`.

Dual-write представление в `bsl-agent` MUST использовать тот же канонический контракт, что и LSP:
- drilldown keys MUST быть primary representation;
- legacy fixed keys MUST быть compatibility-проекцией через тот же deterministic mapping;
- adapter-local reinterpretation/пересчёт legacy семантики MUST NOT применяться.

#### Scenario: Один semantic сценарий даёт сопоставимые drilldown-метрики в LSP и MCP
- **GIVEN** эквивалентный сценарий запросов выполняется через LSP и MCP
- **WHEN** клиент сравнивает observability snapshots
- **THEN** значения сопоставимы по `operation+stage+outcome/reason`
- **AND** различия объясняются только `origin`, а не расхождением orchestration логики
- **AND** legacy fixed keys в MCP и LSP согласованы как проекции одного канонического контракта

### Requirement: MCP adapter эмитит только канонические observability события (MUST)
`bsl-agent` MUST формировать observability через тот же канонический event model, что и LSP/web, с `origin=agent`.

Adapter-layer MUST NOT:
- вводить отдельные semantic категории outcome/reason, отсутствующие в канонической schema;
- публиковать legacy fixed keys напрямую в обход projection-слоя.

#### Scenario: MCP не добавляет adapter-local observability семантику
- **GIVEN** MCP semantic tool выполняет stage pipeline
- **WHEN** формируется observability snapshot
- **THEN** все значения объясняются каноническими событиями с `origin=agent`
- **AND** отсутствуют метрики, появившиеся только из adapter-local reinterpretation

### Requirement: Batch semantic инструменты `bsl-agent` используют background CPU class (MUST)
Долгие MCP операции со сканированием множества файлов MUST выполняться как background workload class в shared runtime budget.

Минимально это требование распространяется на `bsl_symbol_search` и `bsl_references`; эквивалентные file-scan semantic операции MUST следовать той же политике.

Latency-critical инструменты (`bsl_type_at_position`, `bsl_members`, `bsl_definition`) MUST сохранять interactive-priority path.

#### Scenario: Интерактивный запрос не блокируется длинным batch-сканированием
- **GIVEN** запущен долгий `bsl_symbol_search` или `bsl_references`
- **WHEN** параллельно приходит `bsl_type_at_position`
- **THEN** интерактивный запрос получает runtime слот без starvation
- **AND** observability метрики отражают разделение нагрузок на background и interactive классы

### Requirement: `bsl-agent` имеет mixed-load perf regression guard для interactive прогресса (MUST)
Система MUST иметь regression-проверку, которая запускает смешанную MCP нагрузку (batch + interactive) и подтверждает, что interactive path делает прогресс до завершения всего batch хвоста.

Guard MUST опираться на observability и/или детерминированные временные ожидания без flaky сетевых зависимостей.

#### Scenario: Mixed-load smoke выявляет starvation
- **GIVEN** одновременно выполняются batch и interactive MCP инструменты
- **WHEN** запускается perf regression guard
- **THEN** тест подтверждает прогресс interactive path
- **AND** при starvation тест детерминированно падает
