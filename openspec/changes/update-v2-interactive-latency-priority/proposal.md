# Change: update-v2-interactive-latency-priority

## Why
Текущая v2 orchestration модель в LSP использует строгую последовательность `wait_for_file_version -> snapshot -> queries` для всех операций, включая latency-sensitive интерактивные запросы.

По наблюдаемым метрикам это приводит к заметным tail-latency:
- `intellisense_v2_syntax_diagnostics_query`: p50 ~2318ms, p95 ~2640ms;
- `completion_duration_ms`: p50 ~2612ms, p95 ~3053ms;
- `intellisense_v2_wait_for_file_version_completion_ms`: p95 ~2805ms.

Очередь runtime почти не является bottleneck (в основном 0-3ms), поэтому проблема концентрируется в политике свежести данных и конкуренции CPU между background diagnostics и интерактивными запросами.

## What Changes
- Зафиксировать в `bsl-intellisense-v2` latency-priority policy для интерактивных LSP операций (`completion`, `hover`, `signatureHelp`):
  - предпочтение актуальной версии,
  - bounded wait с runtime knobs (`intellisense_v2_interactive_wait_budget_ms`, default `120ms`),
  - controlled stale fallback на последний доступный snapshot с лимитами `version_gap`/`stale_age_ms`.
- Зафиксировать strict consistency для `diagnostics`: публикация только для актуальной requested version, без stale overwrite.
- Зафиксировать singleflight-дедупликацию дорогих v2 query для одинаковой ревизии (`parse_result`, `syntax_diagnostics`, `ir`) и lifecycle очистки in-flight.
- Зафиксировать priority-aware CPU scheduling с раздельными permits для `interactive` и `background`.
- Зафиксировать observability контракт для stale/singleflight/priority поведения с фиксированными ключами метрик.

## Impact
- Affected specs: `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/system/system_coordinator/coordinator.rs`

## Non-Goals
- Переход на LSP pull diagnostics (`textDocument/diagnostic`) в этом change.
- Изменение публичных LSP/HTTP/MCP контрактов payload.
- Полная переработка semantic pipeline вне задач latency/freshness/scheduling.
