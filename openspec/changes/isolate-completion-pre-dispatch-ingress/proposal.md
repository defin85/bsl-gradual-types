# Change: Изолировать completion pre-dispatch ingress от общего LSP backlog

## Почему

`stabilize-completion-front-edge` сделал completion incident bundles детерминированными и убрал ambiguity между client probe и server trace, но следующий bundle `2026-03-27T18:17:21Z` показал новый dominant gap:

- у части запросов human-readable verdict уже говорит `client_before_transport_dominant`;
- при этом server-side `dispatch_to_request_context_wait_ms`, `transport_to_service_future_wait_ms`, `service_future_to_first_poll_wait_ms` и `turn_wait` остаются около `0-1ms`;
- `transport_adapter.rs` ждёт `service.poll_ready()` до того, как completion request вообще классифицируется и попадает в existing handoff seam;
- `jsonrpc_dispatch_received_at_ms` фиксируется только внутри `DispatchContextService::call()`, то есть уже после этого общего readiness wait.

Следовательно, текущий observability split не умеет отделять настоящий client-side ingress от server-side backlog в окне `adapter read -> dispatch`, а default transport path всё ещё может задерживать интерактивный completion до входа в существующий post-dispatch pipeline.

## Что меняется

- authoritative completion timeline получает раннюю server-side ingress timestamp на transport adapter boundary и bounded split для окна `adapter read -> dispatch`;
- response version поднимается `18 -> 19`, а contiguous contract baseline поднимается `contracts/lsp-completion-timeline/v15 -> v16`;
- derived extension verdicts перестают приписывать server pre-dispatch backlog клиенту и различают новый случай `adapter_before_dispatch_dominant`;
- transport adapter ОБЯЗАТЕЛЬНО переходит на архитектуру `reader -> single-owner scheduler` со strict priority lanes `control -> completion -> general`:
  - request classification и enqueue для completion/control происходят до shared `poll_ready()` blocking;
  - completion ingress изолируется от общего request backlog до dispatch;
  - queued cancellation сохраняет exactly-once terminal semantics;
  - weighted/fair scheduler и `instrumentation-only` вариант не входят в scope этого change;
- representative mixed-load gate и traceability docs расширяются так, чтобы ловить именно pre-dispatch starvation, а не только post-dispatch first-poll delay.
- saturated completion admission больше не имеет права останавливать single reader: после bounded spillover overflow older queued completion fail-closed завершается как pre-dispatch `queue_rejected`, что сохраняет control reserved progress и late `$/cancelRequest` classification на default path.
- change-specific readiness wrapper остаётся blocking не только для mixed-load transport path, но и для authoritative representative-matrix perf gate; поэтому delivery обязан не оставлять drift между transport-side fix и уже shipped shared runtime latency policy для соседних user-facing semantic queries (`members`, `type_at_position`).

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/test/suite/*completion*`
  - `contracts/lsp-completion-timeline/v16/*`
  - `docs/agent/verification.md`
  - `docs/guides/development-workflow.md`
  - `scripts/validate-*completion*.sh`
- External references:
  - Tower `Service`: https://docs.rs/tower/latest/tower/trait.Service.html
  - Tower `buffer`: https://docs.rs/tower/latest/tower/buffer/index.html
  - LSP completion: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
  - LSP cancellation: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#cancellation-support
  - VS Code `CompletionItemProvider`: https://code.visualstudio.com/api/references/vscode-api#CompletionItemProvider
