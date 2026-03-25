# Изменение: быстро освобождать superseded active completion turn

## Почему
После `refactor-document-symbol-interactive-isolation` прироста в проблемном completion-сценарии не произошло: incident bundle `2026-03-24T18:21:45Z` показывает, что bottleneck сместился не в `documentSymbol`, а в уже активный completion request, который продолжает держать interactive turn после потери актуальности.

Подтверждающие данные из bundle:
- request `42` ждёт `5903ms` до первого `poll()`, хотя после входа в handler сам request отрабатывает только `120ms`;
- для того же request `wait_for_file_version_runtime.resolution = immediate` и `wait_elapsed_ms = 0`, то есть stall живёт не в current-revision wait path;
- предыдущий request `32` получает first poll за `1ms`, но остаётся в handler `8961ms`;
- клиентский probe стартует при `active_completion_count_at_start=1`, то есть новый completion заходит, пока старый same-file completion всё ещё считается активным.

Это означает, что текущий latest-wins/cancellation contract для completion недостаточно жёстко фиксирует поведение уже активного request после first poll: старый request может быть superseded, но всё равно слишком долго удерживать active turn и задерживать first poll нового запроса.

## Что меняется
- Зафиксировать в `bsl-intellisense-v2` новый контракт: superseded same-file completion MUST promptly release active interactive ownership, а не дожидаться полного stale `response_build`.
- Потребовать cooperative cancellation checkpoints внутри длинного completion `response_build` tail (`collect` / `rank` / `format` или эквивалентной interruptible boundary), чтобы superseded request не держал multi-second stale tail после потери latest-wins.
- Расширить representative completion gate отдельным overlap profile, который проверяет live same-file completion supersession: старый request завершается boundedly, а новый request достигает first poll в пределах interactive budgets.
- Явно зафиксировать архитектурное ограничение: fix MUST выполняться на existing completion path и MUST NOT подменяться новой admission lane, дополнительным transport workaround или общим executor redesign.

## Влияние
- Затронутые спецификации:
  - `bsl-intellisense-v2`
- Затронутый код (implementation follow-up):
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
  - `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - representative validation / readiness scripts и checked-in evidence для overlap gate

## Не-цели
- Не пересматривать заново `documentSymbol` isolation и другие auxiliary LSP methods.
- Не менять current-revision head/exact split из `refactor-completion-prepare-lightweight-exact-split`.
- Не добавлять stale fallback, degraded semantic substitute или silent workaround для completion.
- Не распространять этот change на `hover`, `signatureHelp` и `definition`, пока для них нет отдельной authoritative evidence о таком же root cause.
- Не считать допустимой реализацией новый admission workaround, повышение concurrency само по себе или общий redesign scheduler/executor вместо prompt release stale active completion на существующем completion path.
