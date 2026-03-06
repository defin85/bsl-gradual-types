## ADDED Requirements

### Requirement: Persistent file logging для `bsl-agent` в MCP stdio-режиме
Система SHALL создавать persistent file log для `bsl-agent`, когда он запускается как MCP stdio server.

Этот file log SHALL быть primary operator-visible diagnostic path после startup/runtime/transport failures. `stdout` SHALL оставаться зарезервированным только под MCP transport frames и SHALL NOT содержать log noise.

Default log path SHALL вычисляться от `process cwd`, а не от `workspace_open` или `roots[]`, и SHALL быть равен `<cwd>/.bsl-agent/mcp.log`.

Система SHALL поддерживать env overrides со следующим precedence:
1. `BSL_AGENT_LOG_FILE`
2. `BSL_AGENT_LOG_DIR` + `/mcp.log`
3. `<cwd>/.bsl-agent/mcp.log`

#### Scenario: Default log path создаётся из process cwd до `workspace_open`
- **GIVEN** `bsl-agent` запущен как MCP stdio process с `cwd=/path/to/project` и env overrides для log path не заданы
- **WHEN** процесс стартует и инициализирует logging bootstrap
- **THEN** на диске создаётся каталог `/path/to/project/.bsl-agent/`
- **AND** создаётся или открывается файл `/path/to/project/.bsl-agent/mcp.log`
- **AND** логирование начинает работать до первого `workspace_open`

#### Scenario: Env override выбирает нестандартный log path
- **GIVEN** `BSL_AGENT_LOG_FILE` задан как абсолютный путь `/tmp/custom-agent.log`
- **AND** `BSL_AGENT_LOG_DIR` также задан
- **WHEN** `bsl-agent` стартует как MCP stdio process
- **THEN** effective log path равен `/tmp/custom-agent.log`
- **AND** `BSL_AGENT_LOG_DIR` не используется

#### Scenario: Directory override использует стабильное имя файла
- **GIVEN** `BSL_AGENT_LOG_FILE` не задан
- **AND** `BSL_AGENT_LOG_DIR=/tmp/bsl-agent-logs`
- **WHEN** `bsl-agent` стартует как MCP stdio process
- **THEN** effective log path равен `/tmp/bsl-agent-logs/mcp.log`

### Requirement: Startup и transport diagnostics пишутся в file log максимально рано
Система SHALL инициализировать file logging до обычного stdio MCP lifecycle, чтобы startup/server/transport diagnostics не зависели от успешного `workspace_open`.

Формат log record SHALL включать как минимум:
- timestamp,
- level,
- target/module.

Startup log SHALL фиксировать:
- version/build info,
- `pid`,
- `cwd`,
- effective log path,
- `BSL_CACHE_DIR`,
- `BSL_AGENT_HTTP_ADDR`.

#### Scenario: Startup record содержит операторский контекст инстанса
- **GIVEN** `bsl-agent` стартует как MCP stdio process
- **WHEN** logging bootstrap успешно завершён
- **THEN** в file log появляется startup record с version/build info, `pid`, `cwd`, effective log path, `BSL_CACHE_DIR` и `BSL_AGENT_HTTP_ADDR`

#### Scenario: Transport/process failure оставляет log доступным на диске
- **GIVEN** `bsl-agent` уже записал startup/runtime records в default или overridden file log
- **WHEN** stdio transport закрывается или процесс аварийно завершает работу после MCP операций
- **THEN** ранее записанный log остаётся доступным на диске по стабильному пути
- **AND** оператор может открыть этот файл без восстановления MCP transport

### Requirement: Неуспешная инициализация file log является явной startup ошибкой
Система SHALL считать невозможность инициализировать file log ошибкой startup для MCP stdio режима.

Если file log не удалось создать или открыть, `bsl-agent` SHALL напечатать диагностическое сообщение в `stderr`, содержащее attempted path и причину ошибки, и SHALL NOT продолжать обычный stdio MCP startup без file log.

#### Scenario: File log bootstrap failure явно виден в stderr
- **GIVEN** effective log path указывает в недоступную для записи директорию
- **WHEN** `bsl-agent` стартует как MCP stdio process
- **THEN** процесс печатает в `stderr` сообщение с attempted path и системной причиной ошибки
- **AND** stdio MCP server не продолжает normal startup "вслепую"
