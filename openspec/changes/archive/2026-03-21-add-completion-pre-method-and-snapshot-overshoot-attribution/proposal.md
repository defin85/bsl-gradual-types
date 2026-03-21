# Change: pre-method ingress и snapshot overshoot attribution для completion observability

## Почему
Последние incident bundle показывают два повторяющихся blind spot'а в authoritative completion timeline:
- крупный `server_before_method_entry` bottleneck уже виден по `transport_to_method_wait_ms`, но payload всё ещё не отделяет задержку до первого poll service future от задержки между первым poll и входом в `lsp_completion`;
- `prepare_timeout@prepare_guard` на фазе `snapshot_with_deps` уже локализован, но timeout path не даёт bounded runtime split, достаточный для различения queue wait, writer exec и wake wait overshoot.

Из-за этого следующий цикл отладки всё ещё требует чтения сырого кода и косвенных выводов, хотя проблема уже сузилась до двух конкретных подслоёв.

## Что меняется
- Поднять authoritative `bsl.getCompletionTimeline` contract до `v7`.
- Добавить bounded pre-method ingress split внутри server edge:
  - `transport_received -> service_scope_entered`;
  - `service_scope_entered -> method_entered`;
  - без free-text логов и без high-cardinality payload.
- Добавить timeout-safe bounded attribution для `snapshot_with_deps`, которая остаётся доступной даже на timeout path и помогает отличить как минимум:
  - queue wait;
  - writer exec;
  - wake wait / post-reply wakeup;
  - unavailable.
- Довести новые `v7` fact lines до существующих completion surfaces:
  - Completion Timeline panel;
  - clipboard export;
  - request-centric incident bundle summary.
- Явно зафиксировать graceful degradation на `v6`: отсутствие новых полей не реконструируется и не подменяется эвристикой.

## Не входит в scope
- Исправление самих latency bottlenecks в runtime, tower-lsp или executor scheduling.
- Новая логика probe-to-trace correlation.
- Новый custom request или отдельный лог-файл.
- Изменение exact-wait contract сверх уже существующего `artifact_poll`.

## Влияние
- Затронутые спеки:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Затронутый код:
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/operations.rs`
  - `bsl-runtime/src/application/intellisense_v2/facade/runtime.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
