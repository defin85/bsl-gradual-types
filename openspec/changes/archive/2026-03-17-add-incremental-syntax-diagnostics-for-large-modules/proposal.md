# Change: Mode-aware observability для syntax diagnostics на больших модулях

## Why
Инкрементальный `ParseSnapshot` и fail-safe full fallback уже доставлены как общий parse contract для completion/diagnostics.

Остаточный пробел только один: текущая observability позволяет видеть `parse_snapshot` mode и общий `syntax_diagnostics_query_ms`, но не позволяет формально сравнить latency syntax-stage по parse mode (`incremental`, `reused`, `full`, `other`) в одном и том же diagnostics контуре.

Из-за этого root-cause анализ на больших модулях остается неполным: видно, каким был parse path, и видно общую latency syntax-stage, но не видно их связку на одном low-cardinality observability contract.

## What Changes
- **ADDED**: residual observability delta для `syntax_diagnostics`.
  - Канонический observability contract MUST различать latency syntax-stage по parse mode.
  - Используется low-cardinality mode taxonomy, согласованная с уже существующим `ParseSnapshot` observability: `incremental`, `reused`, `full`, `other`.
  - Поле `mode` в каноническом event model остаётся общим измерением, но его семантика MUST быть stage-aware:
    - для `syntax_diagnostics` `mode` означает parse mode текущей revision-bound diagnostics операции;
    - для completion-related stages сохраняется уже существующая completion-routing semantics.
  - Для `non-LSP` origins или любого diagnostics path без version-bound `ParseSnapshot` система MUST публиковать `mode=other`.
- **ADDED**: deterministic projection rule.
  - Legacy fixed-key метрика `intellisense_v2_syntax_diagnostics_query_ms` сохраняется как aggregate compatibility projection.
  - Mode-aware разрез публикуется через канонический/drilldown contract и тестируется отдельно.

## Explicitly Out of Scope
- Любые изменения incremental parse path, edit mapping, fallback logic или lifecycle `ParseSnapshot`.
- Любые изменения parse/diagnostics semantics.
- Любые новые parser/runtime algorithms.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `analysis-v2/src/lib/analysis_api.rs`
  - `bsl-runtime/src/system/basic_observability/query_metrics.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/system/basic_observability/labels.rs`
  - `bsl-runtime/src/system/basic_observability/core_metrics.rs`
  - `bsl-runtime/src/system/system_coordinator/coordinator/observability.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`

## Out of Scope
- Изменение LSP wire-контракта для diagnostics/completion.
- Замена или повторная реализация `add-incremental-parse-snapshot-for-analysis-v2`.
