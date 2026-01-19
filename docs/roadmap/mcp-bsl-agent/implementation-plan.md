# План реализации: `bsl-agent` (MCP, локальная семантика)

**Статус:** 🔴 ПЛАН  
**Цель:** за 5–10 дней получить working MCP (stdio) с project-level семантикой и `context_pack`.

**Связанные документы:**
- Архитектура: `docs/roadmap/mcp-bsl-agent/architecture.md`
- MCP API: `docs/roadmap/mcp-bsl-agent/api.md`
- Референс по реализации MCP: `mcp-debug-server/`

---

## Область работ (MVP)

MVP покрывает:
- Lifecycle сессии: `workspace_open`, `workspace_status`, `workspace_close`
- Unsaved buffers:
  - ad-hoc snapshot через `FileRef.text` (в рамках одного tool-call)
  - session overlay через `workspace_documents_set` / `workspace_documents_clear` (для `scope=hot`)
- Семантические tools (read-only): diagnostics/type/definition/references/members/search
- Агрегатор для LLM: `context_pack` + дозапрос через `context_expand`

Вне объёма MVP (осознанно):
- Remote/“тонкий агент” к серверу семантики (только local-first)
- HTTP/SSE transport для MCP (только stdio)
- Любые write/patch операции над workspace (строго read-only)

## M0: Skeleton (0.5–1 день)

Цель этапа: поднять минимальный MCP-сервер и зафиксировать контракт lifecycle без семантики.

Задачи:
- Добавить новый workspace crate: `bsl-agent` (bin).
- Подключить `rmcp` server + stdio transport (как `mcp-debug-server/src/main.rs`).
- Настроить structured logging в stderr (важно: не шуметь в stdout, т.к. это MCP transport).
- Реализовать tools-заглушки:
  - `workspace_open`
  - `workspace_status`
  - `workspace_close`
- Ошибки tool-calls: единый формат + корректные коды `INVALID_PARAMS`/`INTERNAL` (по паттерну `mcp-debug-server/src/types/error.rs`).

**DoD:** MCP поднимается, `tools/list` работает, базовый lifecycle сессии есть.

Проверка:
- `cargo build -p bsl-agent`
- Минимальный smoke: `tools/list` и `tools/call workspace_open` / `workspace_status` / `workspace_close`

---

## M1: SemanticFacade extraction (2–3 дня)

Цель этапа: получить единый in-proc API для семантики, пригодный и для LSP, и для MCP (без протаскивания LSP-типов).

Задачи:
- Инвентаризировать текущие entrypoints семантики в `backend/` (diagnostics/type/definition/references/members).
- Выделить общий `SemanticFacade` (или `SemanticProvider`) со стабильными DTO (без `tower-lsp` типов).
- Определить и реализовать:
  - `analysis_revision` (монотонный `u64` внутри сессии)
  - stable ids (hash `blake3` → hex) и фиксированную сортировку (см. `docs/roadmap/mcp-bsl-agent/architecture.md`)
- Добавить unit-тесты на детерминизм:
  - одинаковый snapshot → одинаковые результаты и порядок
  - IDs стабильны внутри одного `analysis_revision`

**DoD:** фасад покрыт unit тестами на детерминизм и базовые сценарии.

Проверка:
- `cargo test -p bsl-backend` (или будущий crate, где живёт `SemanticFacade`)

---

## M2: MCP tools = thin adapter (1–2 дня)

Цель этапа: сделать MCP “тонким” адаптером над `SemanticFacade` и запустить семантические queries на реальном workspace.

Задачи:
- Реализовать `WorkspaceSessionManager` и `DocumentStore`:
  - roots sandbox (canonicalize + запрет path traversal)
  - лимиты: max file size / max total read / max results per query
  - `workspace_documents_set` / `workspace_documents_clear` (overlay + hot_set) → увеличивает `analysis_revision`
- Реализовать tools поверх `SemanticFacade` (как в `docs/roadmap/mcp-bsl-agent/api.md`):
  - `bsl_diagnostics`
  - `bsl_type_at_position`
  - `bsl_members`
  - `bsl_definition`
  - `bsl_references`
  - `bsl_symbol_search` (минимальный индекс)
- Обработать деградации и “честные ответы”:
  - `completeness=partial` + `missing_inputs[]` при отсутствии platform docs/config
  - явная ошибка на stale ids / stale revision
- Добавить базовую observability: тайминги стадий + счётчики (load/parse/resolve/pack).

**DoD:** “точечные” tools работают на реальном workspace и возвращают DTO без паник.

Проверка:
- `cargo test -p bsl-agent` (unit)
- Минимальный e2e: поднять процесс `bsl-agent` и сделать `tools/call` на sample workspace

---

## M3: `context_pack` (2–3 дня)

Цель этапа: дать LLM “IDE‑grade” контекст одним вызовом, с бюджетом и детерминизмом.

Задачи:
- Реализовать `ContextPackBuilder`:
  - бюджетирование: `budget_chars` как hard limit, `budget_tokens` как детерминированный alias
  - ранжирование/приоритизация items по фокусу (`diagnostic`/`symbol`/`position`/`query`)
  - формирование “LLM-ready” текста + структурированных `items[]`
- Реализовать `context_expand` для дозапроса конкретного item.
- Добавить `missing_inputs[]`/`completeness` и `truncated=true` при любой обрезке.
- Добавить golden tests на стабильность `context_pack.text` и состава `items[]` (snapshot через `insta`).

**DoD:** один вызов `context_pack` даёт LLM достаточно данных, чтобы локализовать и исправлять ошибку без ручного “обхода” проекта.

Проверка:
- `cargo test -p bsl-agent` (golden/snapshot)

---

## M4: Integration tests (1–2 дня)

Цель этапа: зафиксировать контракт MCP (stdio) воспроизводимыми интеграционными тестами.

Задачи:
- Интеграционные тесты MCP по stdio:
  - поднять процесс `bsl-agent`
  - `initialize`, `tools/list`, `tools/call` базовых tools
- Golden tests для `context_pack` (стабильность текста/структуры).
- Тесты на `analysis_revision` и stale ids:
  - после `workspace_documents_set` старые `pack_id/item_id/symbol_id` считаются stale
  - сервер отвечает явно (ошибка или `completeness=partial` + причина)

**DoD:** 10+ integration тестов, воспроизводимые результаты.

Проверка:
- `cargo test -p bsl-agent --test '*'`
