# Change: bound runtime readiness waiter contention before apply backlog

## Why
После того как truthful transport seams для completion уже не выглядят проблемой, свежий bundle `2026-04-09T19-25-20Z` продолжает показывать seconds-scale readiness/apply contention внутри runtime:

- `intellisense_v2_runtime_wait_for_file_version_queue_wait_ms p95 = 20876ms`;
- `intellisense_v2_runtime_apply_changes_queue_wait_ms p95 = 20876ms`;
- `intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms p95 = 17171ms`;
- didSave follow-up trace всё ещё публикует `runtime_queue_wait_ms = 8195ms`, `apply_lag_ms = 2349ms`.

Read-only разбор по коду показывает, почему это возможно даже при уже существующих readiness/follow-up gates:

- `WaitForFileVersion` и смежные readiness commands сейчас сначала попадают в generic background writer FIFO в [runtime.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/intellisense_v2/facade/runtime.rs);
- request может spend-ить seconds-scale queue wait ещё до того, как вообще станет passive waiter в writer-owned state;
- пока waiter не зарегистрирован, completion prepare и didSave follow-up не наблюдают current revision readiness как passive condition, а просто висят за тем же backlog, что и apply work;
- operator-facing traces уже различают `wait_for_file_version_runtime_queue_wait_ms` и `runtime_apply_changes_queue_wait_ms`, но architectural path по-прежнему допускает, что passive wait registration сама становится primary latency tail.

## What Changes
- Зафиксировать в `bsl-intellisense-v2`, что readiness observation для current-revision paths (`wait_for_file_version` и semantically equivalent waits) MUST регистрироваться через low-latency passive waiter path, а не через raw generic background FIFO residency до момента регистрации wait.
- Потребовать, чтобы passive readiness wait:
  - не требовал seconds-scale residence в generic writer/runtime queue до becoming observable;
  - не занимал additional blocking CPU permits;
  - оставался distinct failure class от actual apply execution backlog.
- Распространить этот contract как минимум на:
  - interactive completion readiness fast lane;
  - didSave heavy follow-up, когда richer publish всё ещё ждёт applied revision или equivalent ready state.
- Сохранить truthful observability: traces и metrics MUST отделять latency регистрации passive waiter от actual apply execution/apply lag и от downstream semantic work.

## Implementation Order
Это второй change в серии. Он должен идти после `refactor-11-current-context-parse-broker-bounding`, чтобы сначала убрать лишний auxiliary parse storm, а затем уже мерить и лечить оставшийся runtime/apply backlog без искажения от current-context fan-out.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `bsl-runtime/src/system/basic_observability/runtime_metrics.rs`
  - representative perf/live evidence under `backend/tests/perf/reports/`

## Non-Goals
- Не делать полный rewrite writer thread transport в этом change.
- Не увеличивать total runtime/CPU parallelism.
- Не возвращать stale applied state вместо requested revision.
- Не оптимизировать cold semantic query cost.

## Resolved Assumptions
- Основной owner readiness observation остаётся runtime/writer layer, потому что именно там уже живут `applied_file_revisions` и waiter wake-up logic.
- Change должен быть fail-closed: если requested revision так и не стала ready, request может truthfully завершиться bounded timeout/empty outcome, но passive waiter registration не должна сама становиться seconds-scale bottleneck.
