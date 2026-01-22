## ADDED Requirements

### Requirement: Read-only HTTP UI для диагностики MCP состояния (единый SPA)
Система SHALL предоставлять опциональный локальный HTTP UI для `bsl-agent`, предназначенный для разработчиков, чтобы визуально проверить состояние MCP (сессии, jobs, кэш, загрузка platform docs/config).

Система SHALL использовать существующий UI‑артефакт проекта (SPA из `frontend → target/site`) как единую точку ответственности UI, без введения отдельного “второго” UI для MCP.

HTTP UI для `bsl-agent` SHALL быть **строго read-only**: SHALL не предоставлять mutating endpoints (POST/PUT/PATCH/DELETE) и SHALL не модифицировать workspace roots.

HTTP UI SHALL быть выключен по умолчанию и SHALL быть доступен только на `127.0.0.1` (localhost-only). Попытка привязки к `0.0.0.0` SHALL быть отвергнута как ошибка конфигурации.

#### Scenario: Включённый UI поднимается локально и отдаёт SPA
- **GIVEN** запущен `bsl-agent` с включённым HTTP UI и корректным путём к `target/site`
- **WHEN** разработчик открывает `http://localhost:<port>/`
- **THEN** сервер отдаёт SPA (fallback на `index.html`) и UI отображается в браузере

#### Scenario: UI не предоставляет write endpoints
- **GIVEN** включён HTTP UI `bsl-agent`
- **WHEN** клиент делает `POST` (или `PUT/PATCH/DELETE`) запрос к `/api/mcp/status`
- **THEN** сервер возвращает `405` (или `404`) и не изменяет никакое состояние

### Requirement: Capability detection режима UI (web-server vs mcp-agent)
Система SHALL предоставить read-only endpoint `GET /api/mcp/status`, который позволяет UI детектировать backend режим и корректно деградировать.

`bsl-web-server` SHALL реализовать совместимый `GET /api/mcp/status`, возвращающий `supported=false` и `mode=web-server`, чтобы единый SPA мог одинаково работать как в web-server, так и в mcp-agent окружении.

#### Scenario: UI переключается в MCP режим
- **GIVEN** UI загружен из `bsl-agent`
- **WHEN** UI выполняет `GET /api/mcp/status` и получает `supported=true` и `mode=mcp-agent`
- **THEN** UI показывает read-only “MCP Dashboard” и не вызывает web-server mutating API (например `POST /api/snapshot/reload`)

#### Scenario: UI корректно деградирует в web-server режиме
- **GIVEN** UI загружен из `bsl-web-server`
- **WHEN** UI выполняет `GET /api/mcp/status` и получает `supported=false` и `mode=web-server`
- **THEN** UI не показывает MCP-дашборд (или показывает “недоступно”), и продолжает работу в web-server режиме

