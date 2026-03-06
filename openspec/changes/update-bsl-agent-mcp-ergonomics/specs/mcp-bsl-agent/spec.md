## ADDED Requirements

### Requirement: `workspace_open` user-facing contract явно различает platform docs и configuration metadata
Система SHALL в user-facing описаниях `workspace_open`, `mcp_help(tool_name="workspace_open")` и README явно объяснять назначение основных входов:
- `platform_docs_archive` загружает platform types и method signatures;
- без `platform_docs_archive` full platform type lookup MAY быть недоступен, и доступны только fallback/basic platform capabilities;
- `configuration_path` добавляет configuration metadata types;
- `configuration_path` SHALL NOT позиционироваться как замена `platform_docs_archive` для platform types.

#### Scenario: `workspace_open` help объясняет platform-only сценарий
- **GIVEN** MCP-клиент запрашивает `mcp_help(tool_name="workspace_open")`
- **WHEN** сервер строит on-demand help
- **THEN** help явно говорит, что `platform_docs_archive` нужен для full platform types и method signatures

#### Scenario: `workspace_open` help объясняет platform + configuration сценарий
- **GIVEN** MCP-клиент запрашивает `mcp_help(tool_name="workspace_open")`
- **WHEN** сервер строит on-demand help
- **THEN** help явно говорит, что `configuration_path` добавляет configuration metadata
- **AND** help не создаёт впечатление, что `configuration_path` заменяет platform docs

### Requirement: `mcp_help` предоставляет recipe-oriented workflows для common MCP сценариев
Система SHALL расширять `mcp_help` компактными recipe-oriented сценариями поверх per-tool examples.

Минимальный набор recipes SHALL включать:
- diagnostics по файлу;
- hot diagnostics с overlay;
- type at position;
- definition + references;
- resume после рестарта.

Для async workflows help MUST явно фиксировать:
- `job_wait` возвращает status/progress only;
- `job_result` возвращает payload после `succeeded`.

#### Scenario: Async recipe явно разделяет `job_wait` и `job_result`
- **GIVEN** MCP-клиент запрашивает recipe для async workflow
- **WHEN** сервер возвращает `mcp_help`
- **THEN** recipe явно показывает, что `job_wait` не возвращает payload
- **AND** следующий шаг recipe использует `job_result`

### Requirement: `build_info` возвращает operator-visible runtime context
Система SHALL расширять `build_info` additive полями runtime context, полезными оператору после startup/runtime failures.

Минимум ответ SHALL поддерживать:
- `log_file_path: string | null`
- `ui_url: string | null`

Если file logging включён и bootstrap успешно завершён, `log_file_path` SHALL указывать на effective persistent log file.

Если HTTP UI не включён или не стартовал, `ui_url` MAY быть `null`.

#### Scenario: `build_info` показывает effective log path
- **GIVEN** `bsl-agent` запущен как MCP stdio process и file logging bootstrap успешен
- **WHEN** клиент вызывает `build_info`
- **THEN** ответ содержит `log_file_path`, совпадающий с effective log path инстанса

#### Scenario: `build_info` показывает UI context без отдельного discovery шага
- **GIVEN** `bsl-agent` запущен с включённым HTTP UI
- **WHEN** клиент вызывает `build_info`
- **THEN** ответ содержит `ui_url`, согласованный с read-only tool `ui_url`

### Requirement: `bsl_diagnostics_file_start` предоставляет convenience entry point для single-file diagnostics
Система SHALL предоставлять tool `bsl_diagnostics_file_start(...)` как thin convenience wrapper поверх существующего file-scope diagnostics path.

Tool SHALL:
- принимать `session_id` и путь к одному документу внутри `roots`;
- использовать тот же underlying diagnostics execution path, что и `bsl_diagnostics_start` с `scope={kind:file,...}`;
- возвращать `job_id` и следовать той же async job model;
- использовать тот же diagnostics result contract, что и file-scope `bsl_diagnostics_start`.

#### Scenario: Convenience tool эквивалентен tagged file scope
- **GIVEN** ready workspace-сессия и путь к документу внутри `roots`
- **WHEN** клиент вызывает `bsl_diagnostics_file_start(...)`
- **THEN** сервер возвращает `job_id`
- **AND** итоговый `job_result` эквивалентен вызову `bsl_diagnostics_start(scope={kind:file, document:{path:...}})`

### Requirement: Common operator-facing lifecycle errors имеют canonical wording
Система SHALL возвращать предсказуемые operator-facing сообщения как минимум для следующих частых случаев:
- workspace not ready;
- path is outside roots;
- `job_result` вызван до terminal `succeeded`.

Canonical wording MUST оставаться достаточно стабильным, чтобы на него можно было опираться в automation/LLM workflows.

#### Scenario: `job_result` до `succeeded` даёт операционную ошибку
- **GIVEN** job существует, но находится в состоянии `queued` или `running`
- **WHEN** клиент вызывает `job_result`
- **THEN** сервер возвращает `INVALID_PARAMS`
- **AND** сообщение явно указывает, что `job_result` требует `succeeded` state

#### Scenario: Путь вне sandbox roots даёт каноническую ошибку
- **GIVEN** клиент передаёт путь вне `roots`
- **WHEN** сервер пытается разрешить документ
- **THEN** сервер возвращает `INVALID_PARAMS` с канонической фразой про `outside roots`
