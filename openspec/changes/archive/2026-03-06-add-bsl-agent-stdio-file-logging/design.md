## Context
`bsl-agent` уже пишет `tracing` в `stderr`, но для MCP stdio это плохой операторский канал: лог живёт только пока жив transport клиента. В alpha-тесте с клиентским проектом `DO_Rolf_PT` процесс доходит до `workspace_open` / async diagnostics job и затем теряет transport (`Transport closed`), после чего локально в проекте не остаётся стабильного файла, который можно открыть и прочитать.

Ключевое ограничение: default log path должен вычисляться от `process cwd`, а не от `workspace_open`, чтобы logging работал даже если startup или transport упал до открытия workspace.

## Goals
- Дать стабильный и предсказуемый logfile для MCP stdio в клиентском проекте.
- Начинать логирование до `workspace_open`.
- Сохранить `stdout` полностью чистым для MCP transport.
- Сделать операторский путь максимально простым: один стабильный файл, который можно открыть после сбоя.

## Non-Goals
- Rotation / retention / compression.
- Новая transport surface для чтения логов через MCP.
- Привязка log path к `workspace_open` session roots.

## Decisions

### 1. Default base определяется от process cwd
Default path: `<cwd>/.bsl-agent/mcp.log`.

Причина:
- cwd известен в момент bootstrap и не зависит от MCP lifecycle;
- это соответствует operator mental model "лог лежит рядом с клиентским проектом";
- это работает и для падений до `workspace_open`.

### 2. Stable single-file path с env overrides
Precedence:
1. `BSL_AGENT_LOG_FILE`
2. `BSL_AGENT_LOG_DIR` + `/mcp.log`
3. `<cwd>/.bsl-agent/mcp.log`

Причина:
- оператор всегда знает один стабильный path;
- overrides не ломают default discoverability;
- `BSL_AGENT_LOG_FILE` покрывает явный integration path, а `BSL_AGENT_LOG_DIR` удобен для каталогов runtime/log collection.

### 3. File sink обязателен, stdout запрещён, stderr допустим как secondary
`stdout` не используется для логов вообще. File log является primary sink. `stderr` может остаться secondary sink для живой диагностики.

Если file sink не удалось инициализировать, процесс должен fail-fast до обычного stdio startup. При этом `stderr` обязан явно показать:
- effective path,
- системную причину ошибки.

Причина:
- "тихий" переход на stderr-only ломает операторский контракт "лог обязателен и лежит на диске";
- fail-fast безопаснее, чем запуск MCP сервера без promised diagnostics path.

### 4. Startup record пишется сразу после bootstrap logger
В startup record фиксируются:
- version/build info,
- `pid`,
- `cwd`,
- effective log path,
- `BSL_CACHE_DIR`,
- `BSL_AGENT_HTTP_ADDR`.

Причина:
- оператору нужно быстро понять, какой инстанс он открыл и с какой конфигурацией;
- это помогает разбирать transport/process crashes без доступа к live stderr.

### 5. Ранние startup/server/transport ошибки должны попадать в тот же файл
Bootstrapping логгера должен происходить до инициализации stdio server/runtime, чтобы ошибки startup и transport teardown оказывались в том же stable log file.

## Testing Strategy
- Unit tests на resolution effective log path и precedence overrides.
- Bootstrap-oriented test на создание default `.bsl-agent/` каталога и файла.
- Если существующий stdio integration harness позволяет, smoke/integration assertion на наличие file log после старта процесса.

## Open Questions
- Нужно ли в этом же change возвращать `log_file_path` через `build_info`/help surface. Сейчас это оставлено как optional follow-up, чтобы не расширять публичный MCP contract без необходимости.
