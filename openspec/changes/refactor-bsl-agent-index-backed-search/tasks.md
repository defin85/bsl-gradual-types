## 1. Spec And Contracts
- [ ] 1.1 Уточнить и зафиксировать shared runtime search contract (index-backed primary path) для MCP adapters.
- [ ] 1.2 Зафиксировать правила overlay/revision-consistency и supersede для search batch jobs.
- [ ] 1.3 Зафиксировать observability ключи/поля для `search_path` и fallback reason (low-cardinality).

## 2. Runtime Search Layer
- [ ] 2.1 Реализовать runtime-level query API поверх `IndexSnapshot` для type/symbol/references candidates.
- [ ] 2.2 Добавить candidate-first references flow с ограниченной семантической верификацией.
- [ ] 2.3 Обеспечить детерминированную сортировку и стабильность выдачи в рамках `analysis_revision`.

## 3. MCP Adapter Migration
- [ ] 3.1 Перевести `bsl_types_search_start` и parity `GET /api/mcp/search` на shared runtime search API.
- [ ] 3.2 Перевести `bsl_symbol_search_start` с full workspace scan на index-backed candidates.
- [ ] 3.3 Перевести `bsl_references_start` на index-backed candidate path с explicit fallback semantics.

## 4. Validation
- [ ] 4.1 Добавить unit tests для runtime search contract (type/symbol/references candidate retrieval).
- [ ] 4.2 Добавить integration tests для MCP: overlay-aware consistency, deterministic ordering, supersede behavior.
- [ ] 4.3 Добавить parity/perf regression checks: MCP vs LSP contract и mixed-load interactive progress.
- [ ] 4.4 Прогнать минимальный verification набор (`cargo test` таргетно по затронутым crates) и задокументировать результаты.
