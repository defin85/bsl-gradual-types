## Context
`bsl-agent` уже использует shared v2 runtime для semantic операций, но search-path остаётся неоднородным:
- type search/list строятся из `repository.get_all_types()` с линейной фильтрацией;
- symbol/references выполняют per-request workspace scan и ephemeral IR per file.

Это ограничивает и `context_pack`: текущий budgeted context path получает мало пользы от search-layer, потому что discovery часто сводится к snippet/materialization path вместо компактного candidate-first отбора.

В проекте уже есть `IndexSnapshot` и индексные структуры (`type_index`, `symbol_index`, `module_index`, `metadata_index`), однако текущий MCP search path использует их неполно.

## Goals / Non-Goals
- Goals:
  - Сделать index-backed path основным для MCP search tools.
  - Обеспечить консистентность с overlays и `analysis_revision`.
  - Свести drift между MCP/LSP, используя общий runtime contract.
  - Снизить worst-case latency и объём дискового чтения на запрос.
  - Подготовить discovery-first runtime contract, который пригоден как substrate для follow-on token-aware `context_pack`/budgeted context orchestration.
- Non-Goals:
  - Не вводить отдельную внешнюю search БД (SQLite/FTS) как primary source.
  - Не менять публичную job-модель MCP tools.
  - Не переписывать completion/hover pipeline.
  - Не redesign'ить `context_pack` payload/planner в рамках этого change.
  - Не вносить `type_at_position`, flow-sensitive diagnostics, members/completion semantics как обязательную часть primary search path.

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

- Decision: `search_path` и `fallback_reason` в рамках этого change живут в observability, а не в публичном `job_result`.
  - Rationale: search result contract должен оставаться compact candidate payload без operator/debug полей; triage и rollout state должны читаться из канонического observability path.

- Decision: shared search contract остаётся discovery-first, а не semantic-summary engine.
  - Rationale: это удерживает scope change управляемым и не смешивает candidate retrieval с тяжёлой semantic интерпретацией.

- Decision: shared runtime search contract должен возвращать semantic-friendly candidate metadata.
  - Rationale: потребители уровня adapters/LLM tooling должны уметь ранжировать и выбирать follow-up reads по компактному candidate set без обязательного full semantic execution на primary path.

- Decision: rollout выполняется с временным operator-only kill-switch через existing runtime env overrides.
  - Contract: `BSL_AGENT_INDEX_SEARCH=0` принудительно переводит MCP search tools на legacy path; отсутствие переменной или значение `1` оставляет index-backed path primary по умолчанию.
  - Observability: при принудительном legacy режиме `search_path=legacy_forced`, а причина фиксируется как `fallback_reason=rollout_override`.
  - Rationale: change заметно меняет latency/correctness envelope на больших workspace, поэтому нужен быстрый rollback без нового API surface.

- Decision: отдельный shared planner facade для `context_pack` в этот change не входит.
  - Rationale: новый search contract должен быть достаточным discovery substrate; orchestration/planner остаётся adapter-level concern до отдельного follow-on change.

## Alternatives Considered
- Оставить текущий file-scan path и только повысить лимиты.
  - Rejected: не решает фундаментальную проблему O(all files) на запрос.

- Встроить отдельный AST/FTS движок как второй источник истины.
  - Rejected for now: дублирует инвалидацию, парсинг и повышает операционную сложность.

- Гибрид с внешней БД как primary.
  - Deferred: возможен позже для offline/CLI сценариев, но не нужен в этом change.

## Related Prior Art
- `Claude-ast-index-search`:
  - полезный паттерн: разделять indexed discovery и exact code extraction/reading;
  - полезный паттерн: держать candidate payload compact и token-aware;
  - что не переносим в этот change: отдельную SQLite/FTS search БД как primary source.

## Risks / Trade-offs
- Риск некорректных/неполных candidates в index path для references.
  - Mitigation: двухфазная верификация + контракт на explicit fallback reason.

- Риск рассинхрона с unsaved overlays.
  - Mitigation: revision-bound merge policy и supersede для устаревших jobs.

- Риск регрессий в determinism/ordering.
  - Mitigation: фиксированный сортировочный контракт и интеграционные parity тесты.

## Migration Plan
1. Добавить shared runtime search contract, semantic-friendly candidate metadata, observability keys и temporary rollback switch semantics.
2. Перевести `bsl_types_search_start` и parity `/api/mcp/search`.
3. Перевести `bsl_symbol_search_start`.
4. Перевести `bsl_references_start` на candidate-first path.
5. Добавить regression тесты (overlay consistency, parity, mixed-load, forced-legacy rollback visibility).
6. `context_pack` оставить на текущем contract в рамках этого change; follow-on change сможет переиспользовать новый discovery-first search layer для planner/enrichment без отдельного planner facade в рамках текущего change.
