# Tasks: add-mcp-bsl-agent-type-tools

## 1. Спецификация
- [x] Добавить delta к `openspec/changes/add-mcp-bsl-agent-type-tools/specs/mcp-bsl-agent/spec.md` (новые MCP tools и сценарии).
- [x] `openspec validate add-mcp-bsl-agent-type-tools --strict --no-interactive`

## 2. MCP tools (stdio) для типов
- [x] Добавить params-структуры в `bsl-agent/src/server/types.rs`:
  - `BslTypesListParams`, `BslTypesSearchParams`, `BslTypeGetParams` (включая `source` и `view`).
- [x] Реализовать tools в `bsl-agent/src/server/mod.rs`:
  - `bsl_types_list_start`,
  - `bsl_types_search_start`,
  - `bsl_type_get_start`.
- [x] Провести вызовы через `SessionManager` (с проверкой `ready=true`) и вернуть DTO в json.
- [x] Обновить `mcp_help` (примеры вызовов новых tools).

## 3. Тесты
- [x] Добавить тесты `bsl-agent` на контракты:
  - invalid params (невалидные лимиты/режимы),
  - неготовая сессия (ожидаемая ошибка),
  - детерминированность порядка (на фиксированном snapshot/fixture).
- [x] Добавить минимальный e2e/интеграционный тест на “детали типа содержат реквизиты/табличные части” на fixture конфигурации (если уже есть).

## 4. Документация
- [x] Обновить `bsl-agent/README.md` (раздел MCP: примеры получения списка типов и реквизитов документа).

## 5. Quality gates (apply-стадия)
- [x] `cargo fmt`
- [x] `cargo clippy -- -D warnings`
- [x] `cargo test`
