# Change: Добавить `bsl-agent` (MCP) — MVP локального семантического агента

## Почему

Сейчас LLM‑агенты (IDE/CLI) не имеют стабильного, “IDE‑grade” семантического контекста по BSL‑проекту: типы, диагностики, переходы по символам и компактный контекст под конкретную задачу приходится собирать вручную и несогласованно.

Цель этого change — получить локальный MCP‑сервер (stdio), который читает workspace и отдаёт семантику проекта детерминированно и в рамках бюджета, не модифицируя файлы (read‑only).

## Что меняется

- Добавляется новый бинарник `bsl-agent` (workspace crate) — MCP server (stdio).
- Семантика предоставляется **in-proc** (через общий `SemanticFacade`/`SemanticProvider`), без проксирования через LSP.
- Поддерживаются unsaved буферы:
  - ad-hoc snapshot через `FileRef.text` в конкретном tool-call;
  - session overlay через `workspace_documents_set` / `workspace_documents_clear` для `scope=hot`.
- Реализуются MCP tools (MVP) для семантики и агрегатор `context_pack` (+ `context_expand`).
- Вводятся стабильные ID + `analysis_revision` и фиксированная сортировка для детерминизма выдачи.
- Добавляются интеграционные тесты MCP по stdio и golden/snapshot‑тесты для `context_pack`.

## Что НЕ входит (явно)

- Remote режим / “тонкий агент” к серверу семантики (только local-first).
- MCP transport кроме stdio (нет HTTP/SSE).
- Любые write/patch операции над workspace (строго read‑only).

## Влияние

- Новые артефакты: `bsl-agent` crate + тесты.
- Ожидаемые затронутые области кода:
  - вынесение/переиспользование семантического фасада/DTO из существующих слоёв (чтобы не зависеть от `tower-lsp` в MCP);
  - возможные небольшие изменения в `backend/` для выделения общего API.
- Публичные интерфейсы существующих компонентов (LSP/CLI/Web) не должны ломаться.

## Референсы

- Roadmap: `docs/roadmap/mcp-bsl-agent/README.md`
- Архитектура: `docs/roadmap/mcp-bsl-agent/architecture.md`
- MCP API: `docs/roadmap/mcp-bsl-agent/api.md`
- Паттерн реализации MCP: `mcp-debug-server/`

