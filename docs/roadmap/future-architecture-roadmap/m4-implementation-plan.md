# План реализации M4: `bsl-agent` (MCP/CLI/IDE)

**Статус:** 🔴 ПЛАН  
**Цель:** единый локальный агент (stdio MCP) для Codex CLI/Claude CLI/VSCode/Cursor, который читает workspace и общается с Semantic Server.

---

## Область работ

- MCP server (stdio) с read-only tools:
  - `diagnostics`
  - `typeAtPosition`
  - `members`
  - `definition`
  - `references`
  - `context.pack`
- Конфигурация endpoint (SaaS vs localhost).
- Политики include/exclude и “hot set” по открытым файлам.
- Реализация `syncMode=progressive` как дефолта для IDE:
  - построение `config.skeleton` на клиенте,
  - отправка `hot_set` (открытые модули),
  - автоматическая догрузка `missingInputs[]` (full-file blobs через `workspace.applyChanges`).

---

## Критерии завершения (DoD)

- Агент не имеет write-функций (никаких applyPatch/edit).
- Агент может отдать диагностику/типы по файлу через MCP вызов.
- `context.pack` возвращает “готовый для LLM” текстовый пакет и (при необходимости) сам выполняет 1–2 итерации догрузки `missingInputs` в рамках бюджета.
