# Change: isolate documentSymbol auxiliary traffic from interactive completion

## Why
После `refactor-current-revision-readiness-fast-lane` и `refactor-completion-prepare-lightweight-exact-split` основной multi-second tail сместился за пределы completion prepare path.

Incident bundle `2026-03-24T10-10-36Z` показывает:
- completion request `40` ждал `18009ms` до первого `poll()`, хотя сам handler занял только `156ms`;
- completion request `54` вошёл в handler за `1ms` и пошёл по `head_hit`, что подтверждает: главный стоппер уже не в canonical completion fast path;
- cumulative metrics в том же процессе показывают `documentSymbol` `runtime_wait_for_file_version` `p95=14951ms` при `count=9`.

Это означает, что companion IDE traffic, прежде всего `textDocument/documentSymbol` для Outline/Breadcrumbs, всё ещё способен занимать admission/transport path и starving interactive completion.

## What Changes
- Зафиксировать `textDocument/documentSymbol` как auxiliary IDE companion path, а не как interactive-critical semantic gate.
- Ввести для `documentSymbol` bounded serving contract с outcome-классами `current_ready`, `latest_ready` и `unavailable`.
- Потребовать admission/execution isolation, чтобы `documentSymbol` не мог задерживать первый `poll()` для `completion`, `hover`, `signatureHelp` и `definition`.
- Формализовать supersession/coalescing policy для устаревших outline-refresh запросов под `didChange`/`didSave` churn.
- Добавить mixed-load observability и representative gate, который ловит outline-induced starvation completion path.

## Impact
- Affected specs:
  - `bsl-intellisense-ide-grade`
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/language_server/impl_features_a.rs`
  - LSP admission/orchestration around `tower-lsp` transport and request dispatch
  - observability/timeline instrumentation для mixed auxiliary + interactive load
  - representative perf/live gate scripts and checked-in evidence

## Non-Goals
- Не перепроектировать все non-interactive LSP methods в одном change.
- Не решать secondary `turn_wait` completion dispatcher, если он остаётся после снятия `documentSymbol` starvation.
- Не подменять completion stale/degraded semantic fallback.
- Не смешивать этот change с detached head snapshot архитектурой из `refactor-current-revision-head-detached-snapshot`.
