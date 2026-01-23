## 1. Implementation
- [ ] Принять `mode="default"` как алиас режима по умолчанию (без warning)
- [ ] Исправить семантику `missing_inputs` для `workspace_open` (не требовать `configuration_path` всегда)
- [ ] Добавить fail-fast валидацию/инференс `platform_version` при `configuration_path`:
  - [ ] Попытка определить версию из дампа конфигурации
  - [ ] Если определить нельзя — `INVALID_PARAMS` с понятным сообщением
- [ ] Добавить LLM-friendly формы для ссылок на файлы/документы и scope на базе абсолютных путей (multi-root):
  - [ ] Нормализация/разрешение абсолютного пути в `(root_id, relative_path)` (deterministic, sandbox)
  - [ ] Поддержка в `workspace_documents_set|clear`, `*_start` (где есть `FileRef`/`DocumentRef`/`WorkspaceScope`)
- [ ] Уточнить семантику прогресса: запрет `percent=100` до terminal-state
- [ ] Обновить документацию MCP API и README `bsl-agent` (примеры “LLM-friendly” payload’ов)
- [ ] Добавить тесты:
  - [ ] Multi-root: абсолютные пути однозначно резолвятся в правильный root
  - [ ] `configuration_path` без `platform_version`: инференс работает или возвращает `INVALID_PARAMS`
  - [ ] Прогресс: `percent=100` только при terminal

