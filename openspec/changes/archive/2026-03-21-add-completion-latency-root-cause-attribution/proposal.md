# Change: add-completion-latency-root-cause-attribution

## Почему
`v5` completion timeline уже умеет локализовать общие bottleneck-классы вроде `transport_to_handler_wait`, `prepare_timeout` и `exact_deadline`, но для реального root-cause анализа всё ещё остаются три blind spot:

- `transport_to_handler_wait_ms` смешивает задержку до входа в метод и async prelude внутри `lsp_completion`, потому что `handler_entered_at_ms` ставится уже после нескольких await-точек;
- `prepare_timeout` может срабатывать через секунды при бюджете `120ms`, но payload не фиксирует, какой именно timeout-layer сработал поздно и насколько он overshoot'нул budget;
- `exact_deadline` может происходить ещё на artifact-readiness polling path, до waiter/task-state path, но текущий `exact_wait` не показывает bounded polling evidence.

Из-за этого root-cause по-прежнему приходится восстанавливать по коду и raw JSON вручную, хотя authoritative timeline уже является основным диагностическим surface.

## Что меняется
- Расширить authoritative контракт `bsl.getCompletionTimeline` до `v6` bounded root-cause attribution-полями для:
  - split между `transport_received -> method_entered` и `method_entered -> handler_entered`;
  - `prepare_timeout` source/budget/elapsed/overshoot;
  - exact artifact polling до перехода в type-index waiter path.
- Сохранить additive, low-cardinality и fail-open инварианты: без нового server API, без отдельного лог-файла и без свободного текста в payload.
- Обновить existing completion-oriented consumer surfaces в extension так, чтобы они принимали `v6` payload, переносили ключевые authoritative fact lines в уже существующие panel/clipboard/incident handoff flows и явно деградировали на `v5`.

## Влияние
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/core/deps_and_precompute.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimeline*`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
