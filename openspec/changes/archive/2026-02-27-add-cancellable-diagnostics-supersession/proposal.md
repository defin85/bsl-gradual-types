# Change: Cancellable diagnostics supersession для didChange burst

## Why
При активном редактировании deferred/heavy diagnostics задачи могут становиться устаревшими раньше завершения, но продолжать потреблять CPU до естественного конца. Это увеличивает конкуренцию за ресурсы с интерактивным completion и удлиняет tail latency.

## What Changes
- **ADDED**: контракт supersession-cancel для diagnostics задач на уровне `(file_id, profile, generation/version)`.
- **ADDED**: кооперативная отмена in-flight diagnostics стадий (parse/syntax/semantic) при приходе более новой ревизии.
- **ADDED**: единая причина/disposition модель для superseded cancellation в observability.
- **ADDED**: strict гарантия, что superseded задача не публикует diagnostics и не продолжает тяжелые стадии после cancel checkpoint.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade.rs`
  - `bsl-runtime/src/application/intellisense_v2/policy.rs`
  - `bsl-runtime/src/system/basic_observability.rs`

## Dependencies
- Should build on top of `add-incremental-parse-snapshot-for-analysis-v2` for maximum effect.

## Out of Scope
- Изменение пользовательской severity/rules diagnostics.
- Изменение LSP publish формата.
