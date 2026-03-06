## 1. Spec And Contracts
- [ ] 1.1 Уточнить и зафиксировать shared runtime search contract (index-backed primary path) для MCP adapters.
- [ ] 1.2 Зафиксировать правила overlay/revision-consistency и supersede для search batch jobs.
- [ ] 1.3 Зафиксировать observability ключи/поля для `search_path` (`index|fallback|legacy_forced`) и fallback reason (low-cardinality, включая `rollout_override`).
- [ ] 1.4 Зафиксировать boundary: search contract остаётся discovery-first и служит substrate для follow-on `context_pack`, но не тянет в scope redesign `context_pack` и full semantic enrichment.
- [ ] 1.5 Зафиксировать temporary rollback switch через existing runtime env overrides: default index-backed on, explicit operator override forces legacy path.

## 2. Runtime Search Layer
- [ ] 2.1 Реализовать runtime-level query API поверх `IndexSnapshot` для type/symbol/references candidates.
- [ ] 2.2 Добавить candidate-first references flow с ограниченной семантической верификацией.
- [ ] 2.3 Обеспечить детерминированную сортировку и стабильность выдачи в рамках `analysis_revision`.
- [ ] 2.4 Добавить semantic-friendly candidate metadata (stable identity, symbol kind, qualified/enclosing owner where available, file/range), достаточный для adapter-level compact context planning.
- [ ] 2.5 Удержать primary search path discovery-oriented: без обязательной snippet materialization и без unconditional full semantic execution для каждого candidate.

## 3. MCP Adapter Migration
- [ ] 3.1 Перевести `bsl_types_search_start` и parity `GET /api/mcp/search` на shared runtime search API.
- [ ] 3.2 Перевести `bsl_symbol_search_start` с full workspace scan на index-backed candidates.
- [ ] 3.3 Перевести `bsl_references_start` на index-backed candidate path с explicit fallback semantics.
- [ ] 3.4 Сохранить текущий публичный `context_pack` contract без payload/planner redesign; новый search layer готовить как follow-on dependency, а не как breaking MCP refactor.
- [ ] 3.5 Подключить temporary rollback switch так, чтобы forced legacy path не менял публичный result contract и был виден только через observability.

## 4. Validation
- [ ] 4.1 Добавить unit tests для runtime search contract (type/symbol/references candidate retrieval).
- [ ] 4.2 Добавить integration tests для MCP: overlay-aware consistency, deterministic ordering, supersede behavior, forced-legacy rollback visibility.
- [ ] 4.3 Добавить parity/perf regression checks: MCP vs LSP contract и mixed-load interactive progress.
- [ ] 4.4 Прогнать минимальный verification набор (`cargo test` таргетно по затронутым crates) и задокументировать результаты.
