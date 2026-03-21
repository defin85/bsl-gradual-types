# Change: bounded pre-service-scope attribution для completion timeline

## Почему
Последние incident bundle с `Completion Timeline v8` уже показывают trustworthy pre-method facts, но крупный bottleneck всё чаще концентрируется целиком внутри `transport_to_service_scope_wait_ms`, тогда как `service_scope_to_method_wait_ms` остаётся нулевым или почти нулевым.

В incident bundle, экспортированном `2026-03-21T12:24:04.933Z`, authoritative source всё ещё сообщает `contract=v8`, а request summary для traces `790`, `815` и `819` показывает именно этот паттерн:
- `transport_to_service_scope_wait_ms=11875/2957/5930`;
- `service_scope_to_method_wait_ms=0`;
- verdict остаётся `server_before_method_entry_dominant`.

Это означает, что текущий authoritative payload уже доказал: задержка живёт до `service_scope_entered`. Но следующего bounded разреза всё ещё нет: оператор не может отличить, ушло ли время
- до возврата `inner.call(request)` и создания service future;
- или уже после создания future, но до первого poll внутри request scope.

Без этого следующий цикл отладки снова требует чтения кода `RequestContextService` и косвенных выводов по bundle, хотя blind spot уже сузился до одного короткого сегмента pipeline.

## Что меняется
- Поднять authoritative `bsl.getCompletionTimeline` contract до `v9`.
- Добавить bounded pre-service-scope split внутри уже существующего `transport_received -> service_scope_entered`:
  - optional `service_future_created_at_ms`;
  - optional `transport_to_service_future_wait_ms`;
  - optional `service_future_to_scope_wait_ms`.
- Сохранить существующие `v8` pre-method fields и provenance semantics без ослабления integrity.
- Довести новые `v9` facts до существующих completion surfaces:
  - Completion Timeline panel;
  - clipboard export;
  - request-centric incident bundle summary / findings / gaps.
- Явно зафиксировать graceful degradation на `v8`: отсутствие `v9` split не реконструируется эвристикой, не маскируется под authoritative signal и не теряется за нейтральным `No gaps were recorded`.

## Не входит в scope
- Исправление самого latency bottleneck до `service_scope_entered`.
- Новый лог-канал, free-text event stream или unbounded debug payload.
- Изменение probe schema или probe-to-trace correlation.
- Новый custom request.

## Влияние
- Затронутые спеки:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Затронутый код:
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
