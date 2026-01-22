## ADDED Requirements

### Requirement: Read-only MCP tool `ui_url` для получения URL локального HTTP UI
Система SHALL предоставлять read-only MCP tool `ui_url`, который позволяет MCP-клиенту получить адрес и порт локального HTTP UI текущего инстанса `bsl-agent`.

Tool `ui_url` SHALL НЕ модифицировать никакое состояние и SHALL НЕ запускать HTTP UI: он только возвращает уже доступный URL (если UI включён и успешно стартовал).

Формат ответа SHALL включать:
- `enabled: bool`
- `ui_url: string | null` (вида `http://localhost:<port>`)

#### Scenario: UI включён, tool возвращает URL
- **GIVEN** `bsl-agent` запущен с включённым HTTP UI (например, `BSL_AGENT_HTTP_ADDR=127.0.0.1:0`) и UI успешно стартовал
- **WHEN** MCP-клиент вызывает tool `ui_url`
- **THEN** tool возвращает `enabled=true` и `ui_url` вида `http://localhost:<port>`

#### Scenario: UI выключен, tool не падает
- **GIVEN** `bsl-agent` запущен без HTTP UI
- **WHEN** MCP-клиент вызывает tool `ui_url`
- **THEN** tool возвращает `enabled=false` и `ui_url=null`

