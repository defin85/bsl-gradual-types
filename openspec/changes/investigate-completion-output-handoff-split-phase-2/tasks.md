# Tasks

## 1. Спека и versioning

- [x] Зафиксировать additive `v24` contract для truthful post-handler handoff split поверх shipped `v23` egress split.
- [x] Явно зафиксировать legacy semantics `response_output_enqueue_completed_at_ms` как compatibility writer-selection seam, а не truthful send-side enqueue acceptance.
- [x] Зафиксировать human-readable projection и explicit degradation rules для `v23`.

## 2. Server handoff instrumentation

- [x] Добавить bounded `response_output_handoff_started_at_ms` и `response_output_handoff_enqueued_at_ms` для completion response до legacy writer-selection boundary.
- [x] Экспортировать truthful `v24` derived waits `response_ready_to_output_handoff_wait_ms`, `response_output_handoff_send_wait_ms` и `response_output_handoff_to_writer_wait_ms`, сохранив `response_ready_to_output_enqueue_wait_ms` как compatibility umbrella.
- [x] Вынести post-response derivation в shared helper, используемый и trace-store patch path, и completion capture path.
- [x] Расширить atomic completion egress patch carrier и contiguous contract baseline `contracts/lsp-completion-timeline/v21`.

## 3. Human-readable surfaces и incident bundle

- [x] Обновить Completion Timeline panel, clipboard export и incident bundle summary на новый `v24` handoff split.
- [x] Явно помечать legacy seam `response_output_enqueue_completed_at_ms` как compatibility boundary и не называть её truthful enqueue completion.
- [x] Явно деградировать на `v23`, не выдумывая pre-enqueue handoff boundaries или culprit attribution.

## 4. Verification

- [x] Добавить focused backend tests на `v24` milestone ordering и derived consistency для `handoff_started -> handoff_enqueued -> writer_selected` split.
- [x] Добавить focused backend test, который гарантированно разводит send-side handoff acceptance и output-loop writer-selection seam.
- [x] Добавить extension tests на `v24` rendering paths и `v23` degradation для clipboard, webview и incident bundle.
- [x] Прогнать минимальный релевантный verify set для backend + extension + contract scripts + OpenSpec validation.
