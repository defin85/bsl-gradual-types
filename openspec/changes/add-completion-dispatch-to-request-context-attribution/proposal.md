# Change: bounded dispatch-to-request-context attribution для completion timeline

## Почему
Последний incident bundle, экспортированный `2026-03-21T15:03:18Z`, уже показывает, что `Completion Timeline v9` сузил blind spot до сегмента `service_future_created -> service_scope_entered`.

Для server-dominant traces `249`, `253` и `272` summary и raw payload показывают один и тот же паттерн:
- `transport_to_service_future_wait_ms=0`;
- `service_future_to_scope_wait_ms=12264/28726/9464`;
- `service_scope_to_method_wait_ms=0`;
- `dispatcher_resolution_latency_ms=0`, `turn_wait_outcome=ready`.

Это означает, что оператор уже может исключить completion dispatcher и pre-method prelude, но всё ещё не может отличить два разных ingress-сценария:
- request завис до входа в `RequestContextService::call`, внутри jsonrpc/tower dispatch path;
- request вошёл в `RequestContextService::call` вовремя, но застрял после возврата `inner.call(request)` и до первого poll service future.

Текущий authoritative payload этого не показывает, потому что публичный `transport_received_at_ms` до сих пор ставится на входе в `RequestContextService::call`, а не в outer jsonrpc/tower ingress.

Без следующего bounded разреза оператору снова приходится читать runtime code, чтобы понять, где именно сидит скрытый backlog: до middleware entry или уже после него.

## Что меняется
- Поднять authoritative `bsl.getCompletionTimeline` contract до `v10`.
- Добавить bounded dispatch-to-request-context split в `server_edge_details`:
  - `transport_received_at_ms_provenance`;
  - optional `jsonrpc_dispatch_received_at_ms`;
  - optional `dispatch_to_request_context_wait_ms`.
- Сохранить `v9` pre-service-scope split и existing pre-method attribution fields без ослабления integrity semantics.
- Явно зафиксировать relationship между legacy `transport_received_at_ms` и новым outer dispatch cut, чтобы operator-facing surfaces не гадали, что именно означает ingress anchor.
- Довести новый `v10` split до существующих completion surfaces:
  - Completion Timeline panel;
  - clipboard export;
  - request-centric incident bundle summary / findings / gaps.
- Явно зафиксировать graceful degradation на `v9`: отсутствие dispatch split не реконструируется эвристикой и не прячется за нейтральным отсутствием gaps.

## Не входит в scope
- Исправление scheduler / executor starvation, если оно действительно подтвердится.
- Новый event log, unbounded trace stream или free-text runtime narrative.
- Изменение probe schema, request correlation model или completion verdict taxonomy.
- Новый custom request помимо уже существующего `bsl.getCompletionTimeline`.

## Влияние
- Затронутые спеки:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Затронутый код:
  - `backend/src/bin/lsp_server/main.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `contracts/lsp-completion-timeline/v7/*`
  - `scripts/check-versioned-contracts.py`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
