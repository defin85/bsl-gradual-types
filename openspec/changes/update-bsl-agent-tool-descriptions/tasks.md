## 1. Inventory
- [ ] Собрать текущий список tool-ов и их `description` (как видит клиент через `tools/list`)
- [ ] Отметить точки неоднозначности (минимум: пути/roots, scope, позиция, lifecycle jobs)

## 2. Rules (style guide)
- [ ] Зафиксировать “гайд” для описаний tool-ов:
  - [ ] 1 строка, без примеров JSON
  - [ ] максимум ~120–160 символов
  - [ ] ключевые форматы писать явно (например: `path: absolute or {root_id,path}`)
  - [ ] если есть multi-root правило — упомянуть детерминизм (longest-prefix) или требование `root_id`
  - [ ] если требуется async flow — упомянуть `*_start → job_wait/job_result`

## 3. Tool descriptions
- [ ] Обновить `description` у tool-ов, где ошибка наиболее вероятна:
  - [ ] `workspace_open` (single-session, config/platform requirements, mode)
  - [ ] `workspace_documents_set` / `workspace_documents_clear` (absolute path формы, overlay/version)
  - [ ] `bsl_diagnostics_start` (scope варианты)
  - [ ] `bsl_type_at_position_start` / `bsl_members_start` / `bsl_definition_start` (позиция, file refs)
  - [ ] `job_*` (percent=100 только terminal, wait semantics)

## 4. On-demand help tool
- [ ] Добавить read-only tool `mcp_help` (или `tool_examples`) который возвращает:
  - [ ] канонический quickstart flow
  - [ ] 2–3 типичных примера payload’ов по имени tool-а
  - [ ] правила путей (multi-root) и scope
  - [ ] типичные ошибки (`INVALID_PARAMS`) и причины
- [ ] В `description` спорных tool-ов добавить короткую фразу “See `mcp_help` for examples”

## 5. Validation
- [ ] Интеграционный тест: `tools/list` содержит обновлённые короткие формулировки (без больших примеров)
- [ ] Прогон: `cargo test -p bsl-agent`

