# Change: Встраивание flow-sensitive анализа в v2 по всем интерфейсам (IDE/Web API/MCP)

> Статус: закрыто как superseded новым change `fix-flow-sensitive-v2-wiring`.

## Why
Сейчас в репозитории уже есть базовая инфраструктура flow-sensitive анализа:
- CFG как часть IR (`SemanticProgram.cfg`) строится в v2 pipeline;
- есть доменная логика type narrowing / null-safety и тесты на неё.

Но результаты flow-sensitive анализа не доходят до пользовательских интерфейсов:
- LSP hover/completion/diagnostics используют v2 snapshot, но не используют flow-sensitive результаты;
- Web API и MCP не возвращают flow-sensitive данные, несмотря на существующие флаги/параметры;
- текущая модель CFG в v2 не имеет стабильной привязки к IR (позициям/узлам), что затрудняет корректное `type-at-position` с учётом control flow.

Это создаёт разрыв между “инфраструктура есть” и “пользователь видит пользу”, а также повышает риск архитектурного дрейфа (разные места будут пытаться “додумать” flow-sensitive поведение локально).

## What Changes
- Добавить v2-only flow-sensitive подсистему (type narrowing + null-safety) как набор v2 queries, вычисляемых по требованию.
- Встроить flow-sensitive результаты во все интерфейсы:
  - IDE (LSP): hover, completion, diagnostics, signatureHelp, definition;
  - Web API: endpoints, возвращающие типовую информацию;
  - MCP (bsl-agent): `bsl_type_at_position_start`, `bsl_members_start`, `bsl_diagnostics_start` и связанные инструменты.
- Сделать поведение строго управляемым: flow-sensitive вычисления выполняются только при явном включении флага/настройки (default: OFF).
- Зафиксировать архитектурный контракт: как CFG и flow-sensitive результаты привязаны к позициям (byte offsets) и как они используются интерфейсами (без legacy inference путей).
- Расширить repo policy: проверка ссылок на пути в документации запускается в CI (как repo-policy job), чтобы ловить дрейф до мержа.

## Impact
- Affected specs:
  - `bsl-intellisense-v2` (flow-sensitive поведение и gating в IDE/Web API)
  - `mcp-bsl-agent` (flow-sensitive поведение и gating в MCP tools)
  - `dev-workflow` (CI gate для проверки doc-paths)
- Affected code (после утверждения и реализации):
  - `analysis-v2/` (v2 queries для flow-sensitive анализа)
  - `shared/` (контракты CFG/привязка к позициям, DTO)
  - `backend/` (LSP + Web API wiring и настройки)
  - `bsl-agent/` (MCP tools wiring и параметры)
  - `.github/workflows/` (repo policy job)
  - `scripts/` (использование существующего `check-doc-paths.py`)

## Non-Goals
- Делать flow-sensitive анализ включённым по умолчанию во всех режимах (default остаётся OFF).
- Реализовывать “идеальную” точность анализа для всех конструкций языка; цель — корректная интеграция и стабильный контракт/включение.
- Вводить новый отдельный inference pipeline вне `bsl-analysis-v2`.

## Dependencies / Assumptions
- В репозитории уже существует канонический CFG в IR и построение CFG в v2 pipeline (не требуется новый альтернативный путь).
- Текущие flow-sensitive анализаторы (narrowing/null-safety) могут быть переиспользованы или адаптированы, но результаты MUST вычисляться на основе v2 snapshot/queries.
