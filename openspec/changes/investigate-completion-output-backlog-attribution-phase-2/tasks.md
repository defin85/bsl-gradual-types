# Tasks

## 1. Спека и versioning

- [ ] Зафиксировать additive `v23` contract для truthful output backlog attribution поверх `v22` egress split.
- [ ] Зафиксировать human-readable projection и explicit degradation rules для `v22`.

## 2. Unified outbound instrumentation

- [ ] Ввести unified bounded outbound envelope path для writer instrumentation и completion correlation.
- [ ] Добавить authoritative backlog snapshot `output_messages_ahead_count`, `output_bytes_ahead_estimate` и `output_head_blocker_class`.
- [ ] Зафиксировать snapshot semantics: enqueue-time snapshot включает active writer head и queued ahead envelopes.
- [ ] Обновить contiguous contract baseline `contracts/lsp-completion-timeline/v20`.

## 3. Human-readable surfaces и incident bundle

- [ ] Обновить Completion Timeline panel, clipboard export и incident bundle summary на новый `v23` backlog attribution.
- [ ] Явно деградировать на `v22`, не выдумывая backlog culprit.

## 4. Verification

- [ ] Добавить focused backend tests на truthful ahead snapshot для active head и queued blockers.
- [ ] Добавить focused backend tests на `output_head_blocker_class` vocabulary и `output_bytes_ahead_estimate` semantics.
- [ ] Добавить extension tests на `v23` rendering/degradation paths для clipboard, webview и incident bundle.
- [ ] Прогнать минимальный релевантный verify set для backend + extension + contract scripts.
