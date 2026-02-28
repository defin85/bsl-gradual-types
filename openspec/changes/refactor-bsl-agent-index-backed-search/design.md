## Context
`bsl-agent` уже использует shared v2 runtime для semantic операций, но search-path остаётся неоднородным:
- type search/list строятся из `repository.get_all_types()` с линейной фильтрацией;
- symbol/references выполняют per-request workspace scan и ephemeral IR per file.

В проекте уже есть `IndexSnapshot` и индексные структуры (`type_index`, `symbol_index`, `module_index`, `metadata_index`), однако текущий MCP search path использует их неполно.

## Goals / Non-Goals
- Goals:
  - Сделать index-backed path основным для MCP search tools.
  - Обеспечить консистентность с overlays и `analysis_revision`.
  - Свести drift между MCP/LSP, используя общий runtime contract.
  - Снизить worst-case latency и объём дискового чтения на запрос.
- Non-Goals:
  - Не вводить отдельную внешнюю search БД (SQLite/FTS) как primary source.
  - Не менять публичную job-модель MCP tools.
  - Не переписывать completion/hover pipeline.

## Decisions
- Decision: использовать `IndexSnapshot` как primary read-model для search.
  - Rationale: индекс уже строится и обновляется в runtime, что минимизирует дублирование pipeline.

- Decision: вынести search API в shared runtime facade, а `bsl-agent` оставить адаптером.
  - Rationale: это снижает риск расхождения поведения между MCP и LSP.

- Decision: для references применить двухфазную схему.
  - Phase A: индексный отбор кандидатов.
  - Phase B: ограниченная семантическая верификация только по кандидатам.
  - Rationale: сохраняем корректность без полного сканирования workspace на каждый запрос.

- Decision: fallback допускается только как явный, наблюдаемый режим.
  - Rationale: без silent fallback проще triage и rollback; соответствует fail-closed практике для correctness-path.

## Alternatives Considered
- Оставить текущий file-scan path и только повысить лимиты.
  - Rejected: не решает фундаментальную проблему O(all files) на запрос.

- Встроить отдельный AST/FTS движок как второй источник истины.
  - Rejected for now: дублирует инвалидацию, парсинг и повышает операционную сложность.

- Гибрид с внешней БД как primary.
  - Deferred: возможен позже для offline/CLI сценариев, но не нужен в этом change.

## Risks / Trade-offs
- Риск некорректных/неполных candidates в index path для references.
  - Mitigation: двухфазная верификация + контракт на explicit fallback reason.

- Риск рассинхрона с unsaved overlays.
  - Mitigation: revision-bound merge policy и supersede для устаревших jobs.

- Риск регрессий в determinism/ordering.
  - Mitigation: фиксированный сортировочный контракт и интеграционные parity тесты.

## Migration Plan
1. Добавить shared runtime search contract и observability keys.
2. Перевести `bsl_types_search_start` и parity `/api/mcp/search`.
3. Перевести `bsl_symbol_search_start`.
4. Перевести `bsl_references_start` на candidate-first path.
5. Добавить regression тесты (overlay consistency, parity, mixed-load).

## Open Questions
- Нужен ли отдельный feature-flag rollout для index-backed search path в MCP (по умолчанию on/off)?
- Нужна ли явная выдача `search_path` в payload `job_result`, или достаточно observability snapshot?
