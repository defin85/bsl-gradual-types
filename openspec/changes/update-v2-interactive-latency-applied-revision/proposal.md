# Change: Уточнить latency-priority v2 через applied revision и приоритет control-path

## Why
Новые замеры warm-path для LSP показывают, что пользовательское поведение не улучшилось:
- `completion_result_total_cancelled` остаётся около 50% от `completion_total`;
- `p95(intellisense_v2_wait_for_file_version_completion_ms)` держится около 2.6-2.8s при wait budget 120ms;
- наблюдается повторяемый цикл "пустой completion -> повторный completion даёт результат" после правок в новой строке.

По коду видно, что:
- singleflight ключ включает `file_version`, поэтому каждый `didChange` создаёт новый cold key;
- completion ждёт latest received version, но не различает `received` и реально `applied` версию runtime;
- CPU-borrow правила допускают конкуренцию background и interactive за зарезервированные permits, что ухудшает tail latency.

## What Changes
- Зафиксировать обязательную модель двух ревизий файла для интерактивного пути:
  - `received_version` (из LSP didChange),
  - `applied_version` (подтверждённая runtime writer thread после `SetFile`).
- Уточнить latency-priority policy:
  - интерактивный wait ориентируется на `applied_version`;
  - completion при bounded-timeout/cancel использует stale-first fallback и возвращает частичный результат (`isIncomplete=true`) вместо "жёстко пустого" ответа, когда stale snapshot допустим.
- Уточнить CPU scheduling контракт:
  - control-path runtime операции получают приоритет над тяжёлыми query-path задачами;
  - background не должен отбирать интерактивную гарантию при наличии interactive waiters.
- Расширить observability обязательными метриками lag/fallback и добавить quality-gate по cancel-rate completion.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (follow-up implementation):
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `backend/src/bin/lsp_server/server/core.rs` (perf/contract tests)

## Non-Goals
- Переписывание семантического движка `analysis-v2` или изменение алгоритма вывода типов.
- Изменение протокола LSP за пределами допустимого completion поведения (`isIncomplete`, partial/stale response).
- Введение новых внешних сервисов/хранилищ для completion-кэша.
