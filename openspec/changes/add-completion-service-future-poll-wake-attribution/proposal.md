# Change: bounded service-future first-poll / first-wake attribution для completion timeline

## Почему
Incident bundle, экспортированный `2026-03-21T16:29:20.495Z`, показывает, что `Completion Timeline v10` уже сузил blind spot до сегмента `service_future_created -> service_scope_entered`.

Для server-dominant traces `50`, `55`, `70` и `74` summary и raw payload повторяют один и тот же паттерн:
- `dispatch_to_request_context_wait_ms=0`;
- `transport_to_service_future_wait_ms=0`;
- `service_future_to_scope_wait_ms=11945/14715/5896/8881`;
- `service_scope_to_method_wait_ms=0`;
- `dispatcher_resolution_latency_ms=0`, `turn_wait_outcome=ready`.

Это означает, что оператор уже может исключить:
- lag до входа в `RequestContextService::call`;
- sync `inner.call(request)` path до возврата service future;
- pre-method prelude после входа в request scope.

Но authoritative payload всё ещё не различает два разных post-dispatch сценария:
- returned service future долго не poll'илась вообще;
- service future poll'илась быстро, вернула `Pending`, а затем долго не получала первый wake.

Без следующего bounded разреза оператору снова приходится читать tower/runtime code, чтобы понять, где именно сидит backlog: до первого poll future или уже после него на pending/wake path.

## Что меняется
- Поднять authoritative `bsl.getCompletionTimeline` contract до `v11`.
- Добавить bounded service-future poll/wake split в `server_edge_details`:
  - optional `service_future_first_poll_entered_at_ms`;
  - optional `service_future_to_first_poll_wait_ms`;
  - optional `service_future_first_poll_outcome`;
  - optional `service_future_first_wake_scheduled_at_ms`;
  - optional `first_poll_to_first_wake_wait_ms`.
- Сохранить existing `v10` dispatch split, `v9` pre-service-scope split и `v8` pre-method provenance semantics без ослабления integrity rules.
- Явно зафиксировать relationship между `service_future_created_at_ms` и новым first-poll / first-wake cut, чтобы operator-facing surfaces не гадали, где именно future провисла.
- Довести новый `v11` split до существующих completion surfaces:
  - Completion Timeline panel;
  - clipboard export;
  - request-centric incident bundle summary / findings / gaps.
- Явно зафиксировать graceful degradation на `v10`: отсутствие first-poll / first-wake split не реконструируется эвристикой и не прячется за нейтральным отсутствием gaps.

## Не входит в scope
- Исправление tower/runtime scheduler starvation как такового.
- Новый event log, unbounded trace stream или free-text runtime narrative.
- Изменение probe schema, request correlation model или completion verdict taxonomy.
- Новый custom request помимо уже существующего `bsl.getCompletionTimeline`.

## Влияние
- Затронутые спеки:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Затронутый код:
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `contracts/lsp-completion-timeline/v8/*`
  - `scripts/check-versioned-contracts.py`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
