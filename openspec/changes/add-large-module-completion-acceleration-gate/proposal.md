# Change: Объективный gate ускорения completion v2 на больших модулях

## Why
Текущий контракт latency для completion v2 покрывает warm-path, но не фиксирует отдельный объективный критерий ускорения именно на больших модулях по сценарию `start/cold/warm` из LSP observability.

По фактическому прогону (event-driven mode) видно выраженное расхождение между профилями:
- большой модуль: `completion_duration_ms p95 ≈ 3910ms`, `wait_for_file_version_completion_ms p95 ≈ 3024ms`;
- маленький модуль: `completion_duration_ms p95 ≈ 286ms`, `wait_for_file_version_completion_ms p95 ≈ 0ms`.

Без scale-aware baseline/gate нельзя ответить объективно, ускоряемся ли мы на больших модулях или только не регрессируем на маленьких.

## What Changes
- **MODIFIED**: `bsl-intellisense-v2` requirement про interactive latency quality gate.
  - Добавляется scale-aware режим проверки: отдельные профили `large` и `small` в одном прогоне.
  - Для `large` вводятся objective ratio-targets к зафиксированному baseline.
  - Для `small` вводится отдельный non-regression guard.
- **ADDED**: требование про versioned baseline artifact и формат отчета `start/cold/warm` для LSP observability.
  - В отчёт обязательно входят stage-метрики completion контура (`wait_for_file_version`, `snapshot`, `ir_query`) и итоговые percentiles.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/tests/intellisense_v2_conf_big_perf_regression_test.rs`
  - `backend/tests/perf/scenarios/intellisense_large.json`
  - `backend/tests/perf/scenarios/intellisense_small.json`
  - `backend/tests/perf/reports/*` (новый baseline/gate artifacts)
  - `bsl-runtime/src/system/basic_observability.rs`

## Out of Scope
- Расширение семантики completion candidates.
- Изменение UX-контракта LSP completion response.
- Оптимизации, не влияющие на latency профили `large/small`.
