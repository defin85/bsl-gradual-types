# Change: bound conf_big mixed-load degradation across parse and current-revision apply

## Why
Incident bundle `bsl-observability-incident-2026-04-09T10-54-27Z` показал, что для `conf_big`
доминирующая деградация больше не сидит в VS Code UI или completion transport/output path.
Primary latency собирается в backend runtime:
- `didSave` heavy follow-up тратит секунды и десятки секунд в `runtime_queue_wait` и `apply_lag`,
  хотя bounded first publish остается быстрым;
- large-module parse path периодически выпадает в `mode_full`, и один cold/full sample уже делает
  `parse_snapshot_build_ms` огромным;
- auxiliary parse producers и document-sync parse snapshot builders конкурируют в одном bounded
  CPU domain и сериализуются на одном parser mutex.

Кодовое расследование подтвердило три remediation-worthy источника:
- current-revision `ApplyChanges(SetFile)` visibility может отставать под same-file auxiliary churn;
- same-version auxiliary consumers (`build_parse_snapshot_v2`, `bsl.getCurrentContext`, save-triggered
  refresh paths) слишком легко платят повторный cold/full parse за идентичный текст;
- глобальная parser serialization усиливает хвосты на больших модулях.

Уже существующие changes про auxiliary runtime isolation и didSave follow-up lane isolation закрывают
runtime-loop starvation и generic background gating, но не фиксируют отдельный contract для
same-version parse reuse/coalescing и не делают current-revision apply visibility first-class
protected resource под `conf_big` mixed load.

## What Changes
- Добавить в `bsl-intellisense-v2` contract, что после same-file handoff latest `applied_version`
  visibility не должна по умолчанию зависеть от same-file auxiliary parse churn.
- Добавить contract, что large-module same-version auxiliary parse consumers reuse/coalesce canonical
  parse truth вместо repeated independent cold/full parse по идентичному shadow text.
- Зафиксировать representative `conf_big` mixed-load acceptance, который различает parse-cold-start
  regressions и writer/apply backlog, а не схлопывает их в generic runtime slowdown.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/server/language_server/impl_document_sync.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/server/core/diagnostics_runtime.rs`
  - `bsl-runtime/src/system/parser_coordinator.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - representative runtime/perf tests and live reports for `conf_big`

## Non-Goals
- Не переоткрывать UI/extension investigation без нового прямого контрдоказательства.
- Не делать full observability rewrite вместо точечного remediation.
- Не заменять current-revision path detached immutable snapshot architecture из
  `refactor-current-revision-head-detached-snapshot`.
- Не добавлять net-new unlimited process-wide parallelism вместо bounded reuse/coalescing.
