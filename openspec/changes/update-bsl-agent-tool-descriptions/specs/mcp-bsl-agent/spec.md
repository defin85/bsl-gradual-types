## ADDED Requirements

### Requirement: On-demand справка/примеры для MCP tool-ов (`mcp_help`)
Система SHALL предоставлять read-only tool `mcp_help` (или эквивалент), который позволяет MCP‑клиенту получить канонические примеры вызовов и правила форматирования параметров **по запросу**, чтобы не раздувать `tools/list`.

`mcp_help` SHALL поддерживать:
- общий quickstart сценарий (workspace_open → wait ready → documents_set → diagnostics/search → job_result),
- выдачу 2–3 типичных примеров payload’ов по `tool_name`,
- краткие правила для multi-root путей и scope,
- список типичных причин `INVALID_PARAMS`.

#### Scenario: Клиент получает примеры payload’ов для конкретного tool-а
- **GIVEN** MCP клиент подключён к `bsl-agent`
- **WHEN** клиент вызывает `mcp_help` с `tool_name="workspace_documents_set"`
- **THEN** сервер возвращает короткий набор примеров payload’ов и пояснение ключевых ограничений (например, version required with text)

## MODIFIED Requirements

### Requirement: Описания tool-ов в `tools/list` краткие и однозначные
Система SHALL обеспечивать, что `tools/list` содержит `description`, достаточный для однозначного использования tool-ов без “угадывания”, но при этом остаётся компактным (без многострочных JSON-примеров в каждом tool).

Описания tool-ов SHALL:
- быть однострочными (1 line),
- фиксировать ключевые форматы и ограничения (пути/roots, scope, позиция, version/text),
- для async tool-ов явно указывать паттерн `*_start → job_wait/job_result`,
- при наличии on-demand справки упоминать `mcp_help` как источник примеров.

#### Scenario: `tools/list` не содержит многострочных примеров в tool.description
- **GIVEN** клиент вызывает `tools/list`
- **WHEN** клиент читает `description` каждого tool-а
- **THEN** `description` является однострочным и не содержит встроенных многострочных примеров JSON

