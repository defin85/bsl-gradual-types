## 1. Inventory
- [x] Собрать текущий список tool-ов и их `description` (как видит клиент через `tools/list`)
- [x] Отметить точки неоднозначности (минимум: пути/roots, scope, позиция, lifecycle jobs)

## 2. Rules (style guide)
- [x] Зафиксировать “гайд” для описаний tool-ов:
  - [x] 1 строка, без примеров JSON
  - [x] максимум ~120–160 символов
  - [x] ключевые форматы писать явно (например: `path: absolute or {root_id,path}`)
  - [x] если есть multi-root правило — упомянуть детерминизм (longest-prefix) или требование `root_id`
  - [x] если требуется async flow — упомянуть `*_start → job_wait/job_result`

## 3. Tool descriptions
- [x] Обновить `description` у tool-ов, где ошибка наиболее вероятна:
  - [x] `workspace_open` (single-session, config/platform requirements, mode)
  - [x] `workspace_documents_set` / `workspace_documents_clear` (absolute path формы, overlay/version)
  - [x] `bsl_diagnostics_start` (scope варианты)
  - [x] `bsl_type_at_position_start` / `bsl_members_start` / `bsl_definition_start` (позиция, file refs)
  - [x] `job_*` (percent=100 только terminal, wait semantics)

## 4. On-demand help tool
- [x] Добавить read-only tool `mcp_help` (или `tool_examples`) который возвращает:
  - [x] канонический quickstart flow
  - [x] 2–3 типичных примера payload’ов по имени tool-а
  - [x] правила путей (multi-root) и scope
  - [x] типичные ошибки (`INVALID_PARAMS`) и причины
- [x] В `description` спорных tool-ов добавить короткую фразу “See `mcp_help` for examples”

## 5. Validation
- [x] Интеграционный тест: `tools/list` содержит обновлённые короткие формулировки (без больших примеров)
- [x] Прогон: `cargo test -p bsl-agent`
