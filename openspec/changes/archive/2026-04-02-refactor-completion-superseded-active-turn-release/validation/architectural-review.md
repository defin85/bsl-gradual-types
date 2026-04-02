# Архитектурная проверка

## Проверенные зависимости change

- `refactor-completion-prepare-lightweight-exact-split`
- `refactor-document-symbol-interactive-isolation`

## Вывод

Реализация осталась на existing completion path и не превратилась в admission workaround или общий executor redesign.

## Что именно изменено

- В `backend/src/bin/lsp_server/server/language_server/impl_completion.rs` completion request теперь оборачивает `response_build` в abort guard и снимает active turn сразу при наблюдаемом `cancelled/superseded` outcome, а не только в общем epilogue.
- В `bsl-runtime/src/application/type_system/services/completion_service.rs` добавлены cooperative yield boundaries между `collect`, `rank` и `format`, чтобы stale response-build future мог быть реально прерван на ближайшей interruptible boundary.
- В `backend/src/bin/lsp_server/server/core/tests.rs` добавлен live overlap regression для same-file completion и representative real-module overlap gate.

## Что сознательно не делалось

- Не добавлялась новая admission lane.
- Не менялась transport concurrency policy как workaround.
- Не переделывался общий scheduler/executor.
- Не расширялся scope на `hover`, `signatureHelp`, `definition` или `documentSymbol`.

## Риски проверки

- Early release active turn теперь происходит раньше общего completion epilogue. Это уменьшает stale retention, но не меняет published contract для актуального request.
- Cooperative yields добавляют минимальную scheduling overhead, но локализованы внутри тяжёлого completion response-build tail и компенсируются устранением multi-second stale hold.
