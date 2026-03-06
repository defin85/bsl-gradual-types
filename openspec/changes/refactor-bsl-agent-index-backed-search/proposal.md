# Change: refactor-bsl-agent-index-backed-search

## Why
Текущий поиск в `bsl-agent` смешивает два разных подхода: типы ищутся линейной фильтрацией полного набора, а `bsl_symbol_search`/`bsl_references` делают полный обход файлов и построение ephemeral анализа на каждый запрос. На больших workspace это создаёт нестабильную latency, truncation и рассинхрон с unsaved overlays.

Из-за этого search-layer плохо работает как discovery substrate для budgeted MCP tooling: вместо дешёвого отбора компактных candidates с достаточным metadata потребители вроде `context_pack` быстро упираются в snippet-first shaping и теряют token efficiency.

## What Changes
- Ввести единый index-backed search контракт в shared runtime (поверх `IndexSnapshot`) для adapters.
- Перевести `bsl-agent` search-инструменты (`bsl_types_search_start`, `bsl_symbol_search_start`, `bsl_references_start`) на index-backed query path как основной.
- Зафиксировать, что shared search contract остаётся discovery-first и пригоден как substrate для follow-up budgeted context tooling: компактные candidate sets, semantic-friendly metadata, без обязательного full semantic execution на primary path.
- Требовать overlay-aware консистентность поиска: результаты должны соответствовать effective revision (overlay + disk), а устаревшие batch-запуски должны supersede.
- Зафиксировать observability-контракт для поиска: явный путь выполнения (`index`/`fallback`) и low-cardinality причины fallback.
- Явно вывести за рамки change redesign `context_pack` payload/planner и перенос полной семантики (`type_at_position`, flow-sensitive diagnostics, completion-like member resolution) в shared search layer.
- Сохранить текущую модель read-only и single-session; не вводить внешнюю search БД как источник истины в рамках этого change.

## Impact
- Affected specs:
  - `mcp-bsl-agent`
  - `bsl-intellisense-v2`
- Affected code (expected):
  - `bsl-runtime/src/system/intellisense_index*.rs`
  - `bsl-runtime/src/application/intellisense_v2/*` (shared search facade/contracts)
  - `bsl-agent/src/session/mod.rs` (types/symbol/references paths)
  - `bsl-agent/src/server/mod.rs` (tool orchestration wiring)
  - observability export paths для MCP/LSP parity
