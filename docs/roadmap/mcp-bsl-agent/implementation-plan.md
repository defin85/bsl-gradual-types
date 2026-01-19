# План реализации: `bsl-agent` (MCP, локальная семантика)

**Статус:** 🔴 ПЛАН  
**Цель:** за 5–10 дней получить working MCP (stdio) с project-level семантикой и `context_pack`.

---

## M0: Skeleton (0.5–1 день)

- Новый workspace crate: `bsl-agent` (binary).
- `rmcp` server + stdio transport (как `mcp-debug-server/src/main.rs`).
- Структурный logging в stderr.
- Tool: `workspace_open/status/close` (пустые заглушки, но контракт зафиксирован).

**DoD:** MCP поднимается, `tools/list` работает, базовый lifecycle сессии есть.

---

## M1: SemanticFacade extraction (2–3 дня)

- Выделить общий `SemanticFacade` (in-proc API) для:
  - `diagnostics`
  - `typeAtPosition`
  - `definition`
  - `references`
  - `members`
- Сделать DTO слой (без tower-lsp типов).
- Гарантировать детерминизм: stable ids + sorting.

**DoD:** фасад покрыт unit тестами на детерминизм и базовые сценарии.

---

## M2: MCP tools = thin adapter (1–2 дня)

- Реализовать tools поверх `SemanticFacade`:
  - `bsl_diagnostics`
  - `bsl_type_at_position`
  - `bsl_members`
  - `bsl_definition`
  - `bsl_references`
  - `bsl_symbol_search` (минимальный индекс)
- Политики безопасности: roots sandbox + лимит чтения файлов.
- Поддержка unsaved буферов:
  - ad-hoc: `FileRef.text` (snapshot на один вызов);
  - session overlay: `workspace_documents_set` / `workspace_documents_clear` (для `scope=hot` и `context_pack`).

**DoD:** “точечные” tools работают на реальном workspace и возвращают DTO без паник.

---

## M3: `context_pack` (2–3 дня)

- `ContextPackBuilder`:
  - бюджетирование (`budget_chars` как hard limit; `budget_tokens` как подсказка/alias)
  - ранжирование items по фокусу
  - `context_expand`
- Добавить `missing_inputs[]`/`completeness` (например: нет platform docs/config path).

**DoD:** один вызов `context_pack` даёт LLM достаточно данных, чтобы локализовать и исправлять ошибку без ручного “обхода” проекта.

---

## M4: Integration tests (1–2 дня)

- Интеграционные тесты MCP по stdio (поднять процесс, сделать `initialize`, `tools/list`, `tools/call`).
- Golden tests для `context_pack` (стабильность текста/структуры).
- Тесты на корректность `analysis_revision` и поведение при stale IDs.

**DoD:** 10+ integration тестов, воспроизводимые результаты.
