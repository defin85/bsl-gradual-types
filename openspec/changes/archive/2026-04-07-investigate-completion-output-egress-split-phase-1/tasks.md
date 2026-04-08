# Tasks

## 1. Спека и versioning

- [x] Перевести approved change на truthful `v23` redesign с новым `response_output_encode_started_at_ms`.
- [x] Зафиксировать contiguous baseline `contracts/lsp-completion-timeline/v20` и explicit degradation rules для shipped `v22`.

## 2. Server output instrumentation

- [x] Добавить bounded `v23` milestones для completion response: enqueue completed, encode started, encode completed, first actual write started, flush completed.
- [x] Расширить atomic completion egress patch carrier и синхронное применение в authoritative trace store для `v23`.
- [x] Экспортировать truthful `v23` derived waits: `response_ready_to_output_enqueue_wait_ms`, `response_output_queue_wait_ms`, `response_output_encode_exec_ms` и `response_output_write_and_flush_exec_ms` без retroactive reinterpretation `v22`.

## 3. Human-readable surfaces и incident bundle

- [x] Обновить Completion Timeline panel, clipboard export и incident bundle summary на новый `v23` split.
- [x] Явно деградировать на `v22`, не выдумывая literal encode-start/write-start split и backlog attribution.

## 4. Verification

- [x] Добавить focused backend tests на `v23` milestone ordering и derived consistency.
- [x] Добавить extension tests на `v23` rendering paths и `v22` degradation для clipboard, webview и incident bundle.
- [x] Прогнать минимальный релевантный verify set для backend + extension + contract scripts + OpenSpec validation.
