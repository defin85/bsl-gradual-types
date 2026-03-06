# Change: add-bsl-agent-stdio-file-logging

## Why
В alpha-тесте `bsl-agent` в MCP stdio-режиме может завершиться через `Transport closed` уже после `workspace_open` и запуска async job. Сейчас диагностика процесса зависит от live `stderr` потока клиента, поэтому после падения transport/process оператор не может открыть лог из клиентского проекта и понять, что произошло.

Нужен простой и стабильный операторский путь к логу, который начинает работать до `workspace_open`, не трогает `stdout` (MCP transport) и переживает завершение процесса.

## What Changes
- Добавить persistent file logging для `bsl-agent` в MCP stdio-режиме.
- Зафиксировать default log path как `<process cwd>/.bsl-agent/mcp.log`.
- Добавить env overrides `BSL_AGENT_LOG_DIR` и `BSL_AGENT_LOG_FILE` с явным precedence.
- Зафиксировать startup logging contract: build/version info, `pid`, `cwd`, effective log path, `BSL_CACHE_DIR`, `BSL_AGENT_HTTP_ADDR`.
- Логировать startup/server/transport ошибки максимально рано, независимо от `workspace_open`.
- Зафиксировать fail-fast поведение при невозможности инициализировать file log: stderr обязан содержать path + причину, а stdio сервер не должен стартовать "вслепую".
- Обновить документацию и тесты на разрешение log path / bootstrap file logging.

## Non-Goals
- Не вводить log rotation, retention policy или multi-file session layout.
- Не менять MCP transport protocol или stdout framing.
- Не делать `build_info.log_file_path` обязательной частью публичного контракта в этом change; это допустимый follow-up, если реализация окажется безопасной.

## Impact
- Affected specs: `mcp-bsl-agent`
- Affected code: `bsl-agent` bootstrap/logging, stdio startup path, tests, `bsl-agent/README.md`
- Behavior change: `bsl-agent` stdio startup становится зависимым от успешной инициализации file log
