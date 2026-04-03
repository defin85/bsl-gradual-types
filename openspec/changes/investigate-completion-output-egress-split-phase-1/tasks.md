# Tasks

## 1. Спека и versioning

- [ ] Зафиксировать additive `v22` contract для finer output-egress split без backlog snapshot.
- [ ] Зафиксировать human-readable projection и explicit degradation rules для `v21`.

## 2. Server output instrumentation

- [ ] Добавить bounded output-egress milestones для completion response: enqueue completed, write started, encode completed, flush completed.
- [ ] Ввести atomic completion egress patch carrier и синхронное применение в authoritative trace store.
- [ ] Экспортировать derived `response_ready_to_output_enqueue_wait_ms`, `response_output_queue_wait_ms`, `response_output_encode_exec_ms` и `response_output_write_and_flush_exec_ms`.
- [ ] Обновить contiguous contract baseline `contracts/lsp-completion-timeline/v19`.

## 3. Human-readable surfaces и incident bundle

- [ ] Обновить Completion Timeline panel, clipboard export и incident bundle summary на новый `v22` output-egress split.
- [ ] Явно деградировать на `v21`, не выдумывая backlog attribution.

## 4. Verification

- [ ] Добавить focused backend contract tests на immediate `v22` egress split visibility и timestamp/derived consistency.
- [ ] Добавить extension tests на `v22` rendering/degradation paths для clipboard, webview и incident bundle.
- [ ] Прогнать минимальный релевантный verify set для backend + extension + contract scripts.
