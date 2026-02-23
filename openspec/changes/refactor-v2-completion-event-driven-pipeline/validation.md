# Validation: refactor-v2-completion-event-driven-pipeline

Дата: 2026-02-23 (updated; baseline reference remains from 2026-02-20)

## 1. Baseline reference (cold/warm/start)

Источник baseline-артефактов:
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-baseline-start.json`
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-baseline-cold.json`
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-baseline-warm.json`

| profile | iterations | warmup | n (requests) | p95 ms | p99 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| start | 1 | 0 | 5 | 230.827 | 230.827 |
| cold | 20 | 0 | 100 | 96.519 | 142.700 |
| warm | 200 | 20 | 1000 | 78.522 | 83.468 |

## 2. Validation suite runs (7.5)

Выполненные команды:
- `cargo test -p bsl-backend --bin bsl-lsp-server p26_interactive_warm_path_completion_slo_smoke_conf_big -- --nocapture`
- `BSL_INTELLISENSE_V2_COMPLETION_MODE=off cargo test -p bsl-backend --bin bsl-lsp-server p27_interactive_completion_acceptance_gates_emit_artifact -- --nocapture`
- `BSL_INTELLISENSE_V2_COMPLETION_MODE=shadow cargo test -p bsl-backend --bin bsl-lsp-server p27_interactive_completion_acceptance_gates_emit_artifact -- --nocapture`
- `BSL_INTELLISENSE_V2_COMPLETION_MODE=canary BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT=100 cargo test -p bsl-backend --bin bsl-lsp-server p27_interactive_completion_acceptance_gates_emit_artifact -- --nocapture`
- `BSL_INTELLISENSE_V2_COMPLETION_MODE=on cargo test -p bsl-backend --bin bsl-lsp-server p27_interactive_completion_acceptance_gates_emit_artifact -- --nocapture`

Результат:
- `p26_interactive_warm_path_completion_slo_smoke_conf_big`: PASS
- `p27_interactive_completion_acceptance_gates_emit_artifact`: PASS во всех режимах (`off`, `shadow`, `canary(100)`, `on`)

## 3. Rollout gates pass/fail

Источники gate-артефактов:
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-gate-off.json`
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-gate-shadow.json`
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-gate-canary-100.json`
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-gate-on.json`

Gate thresholds (из p27):
- `completion_p95_ms <= 300`
- `completion_p99_ms <= 800`
- `first_trigger_success_rate >= 0.99`
- `terminal_empty_missing_ir_rate <= 0.005`
- `parity_mismatch_rate <= 0.01`

| mode | canary_percent | pass | completion p95 | completion p99 | first_trigger_success_rate | terminal_empty_missing_ir_rate | parity_mismatch_rate |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| off | 0 | PASS | 2.0 | 3.0 | 1.000 | 0.0000 | 0.0000 |
| shadow | 0 | PASS | 2.0 | 3.0 | 1.000 | 0.0000 | 0.0000 |
| canary | 100 | PASS | 2.0 | 2.0 | 1.000 | 0.0000 | 0.0000 |
| on | 0 | PASS | 1.0 | 2.0 | 1.000 | 0.0000 | 0.0000 |

## 4. Mode-split stage metrics (completion contour)

Обязательные стадии: `runtime_wait_for_file_version`, `runtime_snapshot_with_deps`, `ir_query`, `parse_result_query`.

Проверка выполнена по `results.mode_split_stage_metrics` в gate-артефактах:
- `off`: stage totals присутствуют в `mode=legacy` (по 240 на стадию), `event_driven/shadow = 0`.
- `shadow`: stage totals присутствуют в `mode=legacy` и `mode=shadow` (по 240 на стадию), `event_driven = 0`.
- `canary(100)`: stage totals присутствуют в `mode=event_driven` (по 240 на стадию), `legacy/shadow = 0`.
- `on`: stage totals присутствуют в `mode=event_driven` (по 240 на стадию), `legacy/shadow = 0`.

Примечание:
- В p27-артефактах `stage latency p95` по mode-split равен `0.0ms` на данном микрофикстурном прогоне (очень короткий контур + ms-квантование), поэтому для mode-split проверки использовался основной сигнал присутствия/объёма stage totals.

## 5. OpenSpec validation (7.6)

Команда:
- `openspec validate refactor-v2-completion-event-driven-pipeline --strict --no-interactive`

Результат:
- PASS (`Change 'refactor-v2-completion-event-driven-pipeline' is valid`)

## 6. Additional e2e contract runs (2026-02-23)

Выполненные команды:
- `cargo test -p bsl-backend --bin bsl-lsp-server p28_cancel_request_stops_completion_and_prevents_late_publish -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p29_completion_mode_matrix_parity_on_fixed_revision -- --nocapture`
- `cargo test -p bsl-backend --bin bsl-lsp-server p30_backpressure_fairness_interactive_vs_background_no_starvation -- --nocapture`

Результат:
- `p28_cancel_request_stops_completion_and_prevents_late_publish`: PASS
- `p29_completion_mode_matrix_parity_on_fixed_revision`: PASS
- `p30_backpressure_fairness_interactive_vs_background_no_starvation`: PASS

Артефакты:
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-mode-parity-matrix.json`
- `backend/tests/perf/reports/refactor-v2-completion-event-driven-pipeline-fairness-interactive-vs-background.json`

Ключевые значения (p29 matrix):
- user-facing drift vs `off`: `shadow=0.0`, `canary=0.0`, `on=0.0` (threshold `<= 0.01`)
- shadow parity drift: `0.0` (threshold `<= 0.01`)
- stage routing:
  - `off`: `legacy=320`, `shadow=0`, `event_driven=0`
  - `shadow`: `legacy=320`, `shadow=316`, `event_driven=0`
  - `canary(100)`: `legacy=0`, `shadow=0`, `event_driven=320`
  - `on`: `legacy=0`, `shadow=0`, `event_driven=320`

Ключевые значения (p30 fairness):
- Round A (`background burst` vs `interactive probe`): interactive `24/24`, background `24/24`
- Round B (`interactive burst` vs `background probe`): interactive `32/32`, background `16/16`
- bounded latency: `max_request_latency_ms` observed `23.92ms` (threshold `10000ms`)
- saturation counters:
  - `intellisense_v2_runtime_queue_wait_interactive_total=58`
  - `intellisense_v2_runtime_queue_wait_background_total=360`
  - `intellisense_v2_runtime_exec_interactive_total=58`
  - `intellisense_v2_runtime_exec_background_total=360`

## 7. Architecture Lock Conformance Statement

Подтверждение conformance к Architecture Lock:
- Completion execution выполняется **строго через per-file actor queue** с deterministic ordering (`file_seq`) и monotonic `request_epoch`.
- Turn-gating enforced до heavy-path (`CompletionTurnOutcome::Ready|SupersededBeforeStart|QueueRejected`), что исключает late publish для superseded requests.
- Cancellation проходит по контракту `$/cancelRequest -> Cancel(request_id)` с checkpoint stop/no-late-publish (подтверждено `p28`).
- Rollout routing соблюдает lock-контракт `off|shadow|canary|on` с mode-aware stage split (подтверждено `p29`).
- Bounded backpressure/fairness подтверждены двунаправленным e2e (`p30`): interactive и background выполняются без starvation под встречной burst-нагрузкой.
