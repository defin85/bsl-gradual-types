## ADDED Requirements

### Requirement: Single-session политика MCP (не более одной workspace-сессии на процесс)
Система SHALL запрещать одновременную работу более чем одной workspace-сессии в одном процессе `bsl-agent`.

Если в процессе уже существует активная workspace-сессия, повторный вызов `workspace_open` SHALL обрабатываться так:
- Если параметры вызова совпадают с уже открытой сессией (roots + входы platform/config), `workspace_open` SHALL быть идемпотентным и SHALL возвращать `session_id` этой существующей сессии (не создавая новую).
- Если параметры отличаются, `workspace_open` SHALL отклоняться как `INVALID_PARAMS` (HTTP 400 или эквивалент) с понятным сообщением (например, “only one session is allowed; close the existing session first”).

Это ограничение вводится для устранения неоднозначности выбора “текущей” сессии для parity UI и упрощения поведения MCP-клиентов в модели per-agent.

#### Scenario: Открытие первой сессии разрешено
- **GIVEN** в процессе `bsl-agent` нет активных workspace-сессий
- **WHEN** клиент вызывает `workspace_open` с `roots` и входами platform/config
- **THEN** сервер возвращает `session_id` и начинает startup (job), как обычно

#### Scenario: Повторный `workspace_open` с теми же параметрами идемпотентен
- **GIVEN** в процессе `bsl-agent` уже существует активная workspace-сессия, открытая с некоторыми `roots` и входами platform/config
- **WHEN** клиент вызывает `workspace_open` повторно с теми же `roots` и теми же входами platform/config
- **THEN** сервер возвращает тот же `session_id` и не создаёт новую сессию

#### Scenario: Попытка открыть вторую сессию с другими параметрами отклоняется
- **GIVEN** в процессе `bsl-agent` уже существует активная workspace-сессия
- **WHEN** клиент вызывает `workspace_open` повторно с другими `roots` или другими входами platform/config (без предварительного `workspace_close`)
- **THEN** сервер возвращает `INVALID_PARAMS` (HTTP 400 или эквивалент) и сообщение о том, что разрешена только одна сессия

### Requirement: Parity HTTP API для UI в режиме `bsl-agent` (types/search/metrics)
Система SHALL предоставлять read-only parity HTTP API в `bsl-agent` для отображения тех же экранов UI, что и `bsl-web-server`, но на данных workspace-сессии MCP.

Parity API SHALL предоставляться в namespace `/api/mcp/*` и SHALL возвращать те же DTO, что и web-server:
- `GET /api/mcp/types` → `AnalysisResultDto`
- `GET /api/mcp/search` → `AnalysisResultDto`
- `GET /api/mcp/metrics` → `MetricsDto`

Параметры запросов SHALL соответствовать web-server API (пагинация/фильтры/поиск). Parity API MUST быть строго read-only.

#### Scenario: Получение типов из MCP сессии через parity API
- **GIVEN** существует ровно одна MCP сессия с `ready=true`
- **WHEN** UI делает `GET /api/mcp/types?page=1&limit=50`
- **THEN** сервер возвращает `200` и `AnalysisResultDto`, отражающий типы, доступные в этой MCP сессии

#### Scenario: Поиск типов через parity API
- **GIVEN** существует ровно одна MCP сессия с `ready=true`
- **WHEN** UI делает `GET /api/mcp/search?q=ТаблицаЗначений`
- **THEN** сервер возвращает `200` и `AnalysisResultDto` с результатами поиска для этой MCP сессии

### Requirement: Правило выбора сессии для parity API (ровно одна ready без sessionId)
Система SHALL применять единое правило выбора workspace-сессии для parity API:
- Если `sessionId` передан, сервер SHALL использовать указанную сессию.
- Если `sessionId` не передан, сервер SHALL требовать, чтобы существовала ровно одна сессия с `ready=true`, и SHALL использовать её.

Если правило не выполнено, сервер SHALL возвращать `INVALID_PARAMS` (HTTP 400) с понятным сообщением:
- при 0 ready: “no ready sessions” (или эквивалент)
- при >1 ready: “exactly one ready session is required” (или эквивалент)

Сервер SHALL отклонять запросы к parity API для не-ready сессии (`ready=false`) как `INVALID_PARAMS` (HTTP 400).

#### Scenario: Нет ready сессии — parity API отклоняется
- **GIVEN** сессий с `ready=true` нет (startup ещё не завершён)
- **WHEN** UI делает `GET /api/mcp/types` без `sessionId`
- **THEN** сервер возвращает `400 INVALID_PARAMS` и сообщение о том, что нет ready сессий

#### Scenario: Несколько ready сессий — parity API отклоняется
- **GIVEN** существует две или более сессии с `ready=true`
- **WHEN** UI делает `GET /api/mcp/search?q=Документы` без `sessionId`
- **THEN** сервер возвращает `400 INVALID_PARAMS` и сообщение о необходимости ровно одной ready сессии

### Requirement: UI в MCP режиме использует parity API и сохраняет MCP диагностику
Единый SPA (`frontend → target/site`) в MCP режиме SHALL:
- показывать те же экраны (Dashboard/Карточки/Таблица/Граф + поиск/фильтры), что и в `bsl-web-server`;
- использовать parity API `/api/mcp/types|search|metrics` и `/api/mcp/deps/meta`;
- сохранять доступ к MCP диагностике (сессии/jobs) как к MCP-специфическому экрану/разделу;
- оставаться строго read-only (не вызывать mutating endpoints).

Если условие “ровно одна ready сессия” не выполнено, UI SHALL показывать понятную инструкцию и SHALL не делать parity вызовы без `sessionId`.

#### Scenario: UI показывает web-server экраны в MCP режиме при одной ready сессии
- **GIVEN** UI загружен из `bsl-agent` и `GET /api/mcp/status` возвращает `mode=mcp-agent`
- **AND** существует ровно одна сессия с `ready=true`
- **WHEN** пользователь открывает вкладку “Таблица”
- **THEN** UI отображает таблицу типов, используя `GET /api/mcp/types`

#### Scenario: UI показывает инструкцию при 0 или >1 ready сессии
- **GIVEN** UI загружен из `bsl-agent`
- **AND** ready-сессий 0 или >1
- **WHEN** UI пытается загрузить данные для таблицы типов
- **THEN** UI показывает инструкцию “оставьте ровно одну ready сессию” и не делает parity вызовы без `sessionId`
