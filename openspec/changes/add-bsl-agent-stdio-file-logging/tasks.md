## 1. Logging bootstrap
- [ ] 1.1 Спроектировать и реализовать resolution effective log path с precedence `BSL_AGENT_LOG_FILE` > `BSL_AGENT_LOG_DIR` > `<cwd>/.bsl-agent/mcp.log`.
- [ ] 1.2 Гарантировать создание каталога `<base>/.bsl-agent/` для default path до старта stdio MCP lifecycle.
- [ ] 1.3 Подключить persistent file logging как primary sink для stdio MCP startup/runtime ошибок, не затрагивая `stdout`.
- [ ] 1.4 Зафиксировать fail-fast поведение: если file logger не инициализируется, `stderr` печатает path + причину, а процесс не продолжает обычный stdio startup.

## 2. Logging contract
- [ ] 2.1 Добавить startup record с `version/build info`, `pid`, `cwd`, effective log path, `BSL_CACHE_DIR`, `BSL_AGENT_HTTP_ADDR`.
- [ ] 2.2 Гарантировать, что startup/server/transport ошибки логируются в файл максимально рано и не зависят от успешного `workspace_open`.
- [ ] 2.3 Оставить `stderr` допустимым дополнительным каналом диагностики, но file log сделать основным операторским источником.

## 3. Validation
- [ ] 3.1 Добавить минимальные automated tests на default log path и env overrides.
- [ ] 3.2 Добавить coverage на bootstrap/use of log path в stdio startup path.
- [ ] 3.3 Обновить `bsl-agent/README.md` с operator guidance: где искать лог, какие env overrides поддерживаются, как сделать smoke-check.
- [ ] 3.4 Прогнать `openspec validate add-bsl-agent-stdio-file-logging --strict --no-interactive`.
