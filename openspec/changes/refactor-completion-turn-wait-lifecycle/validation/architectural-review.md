# Архитектурная проверка

## Проверенные зависимости change

- `refactor-completion-superseded-active-turn-release`
- `refactor-document-symbol-interactive-isolation`

## Вывод

Реализация осталась completion-scoped follow-up на existing dispatcher path и не превратилась ни в transport-priority workaround, ни в общий scheduler redesign.

## Что именно изменено

- В `backend/src/bin/lsp_server/server/completion_dispatcher.rs` появился явный pre-active lifecycle state для same-file completion между queue exit и `mark_completion_active(...)`, чтобы stale request оставался discoverable для latest-wins/cancel.
- В `backend/src/bin/lsp_server/server/language_server/impl_completion.rs` completion request boundedly сворачивается ещё до active registration, если его уже superseded/cancelled в `turn_wait` lifecycle, и не публикует поздний user-facing completion ответ.
- В `backend/src/bin/lsp_server/server/core.rs` explicit `$/cancelRequest` теперь покрывает и pre-active `turn_wait` window, а не только queued/active states.
- В `backend/src/bin/lsp_server/server/core/tests.rs` и `backend/src/bin/lsp_server/server/completion_dispatcher/tests.rs` добавлены regression и representative gate для stranded pre-active contender, плюс truthful absolute `turn_wait` lifecycle checks.

## Что сознательно не делалось

- Не поднимался приоритет completion относительно других LSP methods как symptom workaround.
- Не увеличивалась transport concurrency как замена root-cause fix.
- Не менялся scope `documentSymbol` isolation change; `documentSymbol` не стал частью нового runtime contract.
- Не переделывался общий ingress scheduler для всех request classes.

## Риски проверки

- Dispatcher теперь хранит дополнительный pre-active registry. Это локально усложняет per-file state machine, но остаётся ограничено только completion path.
- Truthful `turn_wait` lifecycle опирается на absolute timestamps и regression tests; если дальше появится cross-method stall вне completion lifecycle, это должен быть отдельный change, а не расширение текущего scope.
