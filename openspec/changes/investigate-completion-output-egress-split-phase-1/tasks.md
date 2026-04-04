# Tasks

## 1. Спека и versioning

- [x] Зафиксировать additive `v22` contract для finer output-egress split без backlog snapshot.
- [x] Зафиксировать human-readable projection и explicit degradation rules для `v21`.

## 2. Server output instrumentation

- [x] Добавить bounded output-egress milestones для completion response: enqueue completed, write started, encode completed, flush completed.
- [x] Ввести atomic completion egress patch carrier и синхронное применение в authoritative trace store.
- [x] Экспортировать derived `response_ready_to_output_enqueue_wait_ms`, `response_output_queue_wait_ms`, `response_output_encode_exec_ms` и `response_output_write_and_flush_exec_ms`.
- [x] Обновить contiguous contract baseline `contracts/lsp-completion-timeline/v19`.

## 3. Human-readable surfaces и incident bundle

- [x] Обновить Completion Timeline panel, clipboard export и incident bundle summary на новый `v22` output-egress split.
- [x] Явно деградировать на `v21`, не выдумывая backlog attribution.

## 4. Verification

- [x] Добавить focused backend contract tests на immediate `v22` egress split visibility и timestamp/derived consistency.
- [x] Добавить extension tests на `v22` rendering/degradation paths для clipboard, webview и incident bundle.
- [x] Прогнать минимальный релевантный verify set для backend + extension + contract scripts.
