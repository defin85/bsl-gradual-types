# Change: Event-driven precompute `type_index` и serve-only cache для completion v2

## Why
В текущем completion hot path на больших модулях при churn периодически выполняется повторный тяжелый parse/query путь до старта `type_index`, что дает секундные хвосты latency и нестабильный p99.

Система уже имеет version-bound `ParseSnapshot` из `didOpen/didChange`, но интерактивный type lookup все еще может уходить в on-demand пересчет через query pipeline. Это архитектурный drift между ingest-контуром и serving-контуром.

Нужен единый event-driven контракт: вычисление `type_index` происходит при событии изменения документа, а интерактивный запрос только читает cache-артефакт (serve-only), без синхронного запуска тяжелых parse/index стадий.

## What Changes
- **ADDED**: event-driven precompute pipeline `ParseSnapshot -> TypeIndexArtifact` на `didOpen/didChange`.
- **ADDED**: serve-only контракт для интерактивного type lookup (`completion/hover/signatureHelp`) из version-bound cache.
- **ADDED**: детерминированный ключ cache-артефакта `(file_id, file_version, deps_id, settings_id)` и правила invalidation/supersede.
- **MODIFIED**: churn-aware completion стратегия уточняется: latest-path использует только precomputed artifacts; on-demand parse/index в интерактивном запросе запрещен.
- **ADDED**: observability и perf-evidence для precompute/serve-only пути (cache hit/miss, precompute queue wait/exec, bounded fallback outcomes).
- **ADDED**: staged rollout (`shadow -> canary -> on`) с parity-валидацией относительно legacy path.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `analysis-v2/src/lib.rs` (derived artifacts boundary, cache keys, serve-only read path)
  - `analysis-v2/src/type_inference_v2.rs` (precompute integration contract)
  - `bsl-runtime/src/application/intellisense_v2/facade.rs` (event-driven orchestration and scheduling)
  - `backend/src/bin/lsp_server/server/language_server.rs` (didOpen/didChange ingestion and rollout wiring)
  - `bsl-runtime/src/system/basic_observability.rs` (low-cardinality metrics for precompute/serve)
  - `backend/src/bin/lsp_server/server/core.rs` (quality gate integration/reporting)

## Relation To Guardrails
Изменение классифицируется как `perf_critical` и должно проходить с ADR/doc-first/perf evidence в соответствии с `add-performance-first-ai-engineering-guardrails`.

## Non-Goals
- Полный lock-free rewrite runtime очередей.
- Изменение пользовательского LSP API completion/hover/signatureHelp.
- Массовая перестройка всех diagnostics стадий в рамках этого change.
