## ADDED Requirements

### Requirement: MCP tools для discovery типов платформы и конфигурации
Система SHALL предоставлять в `bsl-agent` (stdio MCP) read-only tools для навигации по типам, чтобы MCP-клиент (в т.ч. LLM) мог:
- получить список типов с фильтрами и пагинацией,
- найти тип(ы) по строке,
- получить детали конкретного типа (включая реквизиты и табличные части).

Tools MUST следовать существующему паттерну `*_start` и выполняться асинхронно через job API.

#### Scenario: Клиент получает реквизиты документа через MCP
- **GIVEN** workspace-сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_type_get_start` для типа документа конфигурации
- **THEN** сервер возвращает `TypeDto`, где заполнены `properties[]` и `tabularSections[]`, достаточные для перечисления реквизитов и табличных частей

### Requirement: `bsl_types_list_start` поддерживает пагинацию и фильтры
Система SHALL предоставлять tool `bsl_types_list_start(session_id, page?, limit?, source?, category?, certainty_level?, flow_sensitive_only?, view?)` для получения типов с контролируемым размером выдачи.

#### Scenario: Пагинация ограничивает размер результата
- **GIVEN** workspace-сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_types_list_start` с `page=1` и `limit=50`
- **THEN** сервер возвращает `AnalysisResultDto` с не более чем 50 типами и заполненным `pagination`

#### Scenario: `view="names_only"` возвращает только имена типов
- **GIVEN** workspace-сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_types_list_start` с `view="names_only"`
- **THEN** сервер возвращает JSON массив строк `string[]`, где каждый элемент является именем типа

#### Scenario: `source` фильтрует типы по происхождению
- **GIVEN** workspace-сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_types_list_start` с `source="configuration"`
- **THEN** сервер возвращает только конфигурационные типы

### Requirement: `bsl_types_search_start` ищет типы по строке
Система SHALL предоставлять tool `bsl_types_search_start(session_id, query, limit?, source?, view?)`, который возвращает релевантные типы.

#### Scenario: Поиск возвращает ограниченный набор типов
- **GIVEN** workspace-сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_types_search_start` с `query="Документ"` и `limit=200`
- **THEN** сервер возвращает `AnalysisResultDto` с типами, релевантными запросу, и не превышает лимит

### Requirement: `bsl_type_get_start` возвращает детали типа с управляемым размером
Система SHALL предоставлять tool `bsl_type_get_start(session_id, type_name, source?, include_methods?)`, который возвращает `TypeDto` для точного имени типа.

Если `include_methods=false`, сервер MUST возвращать метаданные типа без полного списка методов (payload-friendly), при этом `methodsCount` MUST быть заполнен.

#### Scenario: Детали типа возвращаются без методов по умолчанию
- **GIVEN** workspace-сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_type_get_start` с `include_methods=false` (или без параметра)
- **THEN** сервер возвращает `TypeDto` с заполненными `properties`/`tabularSections` и пустым `methods[]`, но с заполненным `methodsCount`

### Requirement: `mcp_help` содержит примеры вызовов type tools
Система SHALL обновлять on-demand справку `mcp_help`, добавляя примеры вызовов для `bsl_types_list_start`, `bsl_types_search_start`, `bsl_type_get_start`.

#### Scenario: Клиент получает канонический пример вызова
- **GIVEN** MCP-клиент поддерживает on-demand справку
- **WHEN** клиент вызывает `mcp_help` с `tool_name="bsl_type_get_start"`
- **THEN** сервер возвращает пример вызова с параметрами, достаточными для получения реквизитов и табличных частей
