## 1. Contract
- [ ] 1.1 Зафиксировать user-facing смысл входов `workspace_open`:
  - [ ] `platform_docs_archive` загружает platform types и method signatures
  - [ ] без `platform_docs_archive` full platform type lookup может быть недоступен
  - [ ] `configuration_path` добавляет configuration metadata types и не заменяет platform docs
- [ ] 1.2 Зафиксировать recipe-oriented `mcp_help` для common сценариев:
  - [ ] diagnostics по файлу
  - [ ] hot diagnostics с overlay
  - [ ] type at position
  - [ ] definition + references
  - [ ] resume после рестарта
  - [ ] явное правило: `job_wait` возвращает status only, `job_result` возвращает payload
- [ ] 1.3 Зафиксировать расширение `build_info`:
  - [ ] optional `log_file_path`
  - [ ] optional `ui_url` или эквивалентный UI context
- [ ] 1.4 Зафиксировать convenience tool `bsl_diagnostics_file_start(...)` как thin wrapper над file-scope diagnostics
- [ ] 1.5 Зафиксировать canonical operator-facing error wording для:
  - [ ] workspace not ready
  - [ ] path is outside roots
  - [ ] job_result before succeeded
- [ ] 1.6 Явно зафиксировать совместимость с `add-bsl-agent-compact-diagnostics-mode`:
  - [ ] не дублировать compact payload contract
  - [ ] convenience tool использует тот же diagnostics result path, что и `bsl_diagnostics_start`

## 2. Implementation
- [ ] 2.1 Обновить `#[tool(description=...)]`, `mcp_help(workspace_open)` и README с явным разделением platform docs vs configuration metadata.
- [ ] 2.2 Добавить recipe-oriented `mcp_help` examples/notes для common workflows.
- [ ] 2.3 Расширить `BuildInfoResponse` runtime context полями `log_file_path` и `ui_url` (или эквивалентным UI indicator) без breaking changes.
- [ ] 2.4 Реализовать `bsl_diagnostics_file_start(session_id, path, limit?, include_flow_sensitive?, ...)` как thin wrapper над tagged file scope.
- [ ] 2.5 Нормализовать common operator-facing error wording в job/workspace/filesystem paths.
- [ ] 2.6 Обновить README примерами:
  - [ ] только platform types
  - [ ] platform types + configuration metadata
  - [ ] single-file diagnostics convenience path

## 3. Tests
- [ ] 3.1 Добавить stdio regression: `mcp_help(tool_name=\"workspace_open\")` явно объясняет роли `platform_docs_archive` и `configuration_path`.
- [ ] 3.2 Добавить stdio regression: `mcp_help` содержит recipe-oriented async guidance (`job_wait` vs `job_result`).
- [ ] 3.3 Добавить stdio regression: `build_info` возвращает `log_file_path` и согласованный UI context.
- [ ] 3.4 Добавить stdio regression: `bsl_diagnostics_file_start` эквивалентен `bsl_diagnostics_start(scope={kind:file,...})`.
- [ ] 3.5 Добавить regressions на canonical error wording для common lifecycle mistakes.

## 4. Validation
- [ ] 4.1 Прогнать `openspec validate update-bsl-agent-mcp-ergonomics --strict --no-interactive`.
- [ ] 4.2 Подготовить traceability `Requirement -> Code -> Test`.
