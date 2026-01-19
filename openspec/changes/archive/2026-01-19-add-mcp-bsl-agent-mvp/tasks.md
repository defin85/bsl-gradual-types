# Tasks: `add-mcp-bsl-agent-mvp`

Ниже — перенесённый и атомизированный чеклист M0–M4 из `docs/roadmap/mcp-bsl-agent/implementation-plan.md`.

## 1. Реализация (MVP)

### M0: Skeleton (0.5–1 день)

- [x] Добавить новый workspace crate: `bsl-agent` (bin).
- [x] Подключить `rmcp` server + stdio transport (по паттерну `mcp-debug-server/src/main.rs`).
- [x] Настроить structured logging в stderr (stdout — только MCP transport).
- [x] Реализовать tools-заглушки:
  - [x] `workspace_open`
  - [x] `workspace_status`
  - [x] `workspace_close`
- [x] Единый формат ошибок tool-calls + корректные коды `INVALID_PARAMS`/`INTERNAL` (по паттерну `mcp-debug-server/src/types/error.rs`).
- [x] Зафиксировать поведение по локальному кэшу (env-based) в help/README для `bsl-agent`:
  - [x] `BSL_CACHE_DIR`, `BSL_CACHE_DISABLE`, `BSL_CACHE_STRICT_FINGERPRINT`
  - [x] `BSL_AST_CACHE_CAPACITY`

**DoD:** MCP поднимается, `tools/list` работает, базовый lifecycle сессии есть.

**Проверка:**
- `cargo build -p bsl-agent`
- минимальный smoke: `tools/list` и `tools/call workspace_open` / `workspace_status` / `workspace_close`

---

### M1: SemanticFacade extraction (2–3 дня)

- [x] Инвентаризировать текущие entrypoints семантики в `backend/` (diagnostics/type/definition/references/members).
- [x] Выделить общий `SemanticFacade` (или `SemanticProvider`) со стабильными DTO (без `tower-lsp` типов).
- [x] Зафиксировать и реализовать:
  - [x] `analysis_revision` (монотонный `u64` внутри сессии)
  - [x] stable ids (hash `blake3` → hex)
  - [x] фиксированную сортировку результатов (см. `docs/roadmap/mcp-bsl-agent/architecture.md`)
- [x] Unit‑тесты на детерминизм:
  - [x] одинаковый snapshot → одинаковые результаты и порядок
  - [x] IDs стабильны внутри одного `analysis_revision`

**DoD:** фасад покрыт unit тестами на детерминизм и базовые сценарии.

**Проверка:**
- `cargo test -p bsl-backend` (или будущий crate, где живёт `SemanticFacade`)

---

### M2: MCP tools = thin adapter (1–2 дня)

- [x] Реализовать `WorkspaceSessionManager` и `DocumentStore`:
  - [x] roots sandbox (canonicalize + запрет path traversal)
  - [x] лимиты: max file size / max total read / max results per query
  - [x] `workspace_documents_set` / `workspace_documents_clear` (overlay + hot_set) → увеличивает `analysis_revision`
- [x] Интегрировать локальный кэш:
  - [x] создать/сконфигурировать `DiskCache` (schema_version = 1, как в backend) с учётом env (`BSL_CACHE_*`)
  - [x] переиспользовать DiskCache при загрузке platform docs / config metadata (как в `backend/src/system/system_coordinator/*`)
  - [x] привязать `DiskCache` к парсингу (AST disk cache) и выставлять `cache_scope` (`project_id`/`config_id`) когда известны
- [x] Гарантировать совместное использование кэша LSP + MCP без “драк”:
  - [x] подтвердить, что все записи идут под per‑key `.lock` и атомарной записью (уже есть в `DiskCache`)
  - [x] сделать cleanup/eviction безопасным для multi‑process: перед удалением entry попытаться захватить `.lock`, иначе пропустить
- [x] Реализовать tools поверх `SemanticFacade` (как в `docs/roadmap/mcp-bsl-agent/api.md`):
  - [x] `bsl_diagnostics`
  - [x] `bsl_type_at_position`
  - [x] `bsl_members`
  - [x] `bsl_definition`
  - [x] `bsl_references`
  - [x] `bsl_symbol_search` (минимальный индекс)
- [x] Обработать деградации и “честные ответы”:
  - [x] `completeness=partial` + `missing_inputs[]` при отсутствии platform docs/config
  - [x] явная ошибка на stale ids / stale revision
- [ ] Добавить базовую observability: тайминги стадий + счётчики (load/parse/resolve/pack).

**DoD:** “точечные” tools работают на реальном workspace и возвращают DTO без паник.

**Проверка:**
- `cargo test -p bsl-agent` (unit)
- минимальный e2e: поднять процесс `bsl-agent` и сделать `tools/call` на sample workspace

---

### M3: `context_pack` (2–3 дня)

- [x] Реализовать `ContextPackBuilder`:
  - [x] бюджетирование: `budget_chars` как hard limit, `budget_tokens` как детерминированный alias
  - [x] ранжирование/приоритизация items по фокусу (`diagnostic`/`symbol`/`position`/`query`)
  - [x] формирование “LLM-ready” текста + структурированных `items[]`
- [x] Реализовать `context_expand` для дозапроса конкретного item.
- [x] Добавить `missing_inputs[]`/`completeness` и `truncated=true` при любой обрезке.
- [x] Golden/snapshot‑тесты на стабильность `context_pack.text` и состава `items[]` (через `insta`).

**DoD:** один вызов `context_pack` даёт LLM достаточно данных, чтобы локализовать и исправлять ошибку без ручного “обхода” проекта.

**Проверка:**
- `cargo test -p bsl-agent` (golden/snapshot)

---

### M4: Integration tests (1–2 дня)

- [x] Интеграционные тесты MCP по stdio:
  - [x] поднять процесс `bsl-agent`
  - [x] `initialize`, `tools/list`, `tools/call` базовых tools
- [x] Golden tests для `context_pack` (стабильность текста/структуры).
- [x] Тесты на `analysis_revision` и stale ids:
  - [x] после `workspace_documents_set` старые `pack_id/item_id/symbol_id` считаются stale
  - [x] сервер отвечает явно (ошибка или `completeness=partial` + причина)
- [x] Тесты на локальный кэш (через temp `BSL_CACHE_DIR`):
  - [x] cache enabled: первый прогон создаёт артефакты, второй прогон переиспользует их
  - [x] `BSL_CACHE_DISABLE`: артефакты не создаются и не читаются
- [x] Тест на конкурентный доступ к одному entry (LSP + MCP модель):
  - [x] два процесса/потока одновременно вызывают `get_or_build` на один и тот же ключ в одном `BSL_CACHE_DIR`
  - [x] результат валиден, без повреждений, и сборка выполняется ровно один раз

**DoD:** 10+ integration тестов, воспроизводимые результаты.

**Проверка:**
- `cargo test -p bsl-agent --test '*'`

## 2. Quality gates

- [x] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --workspace`
