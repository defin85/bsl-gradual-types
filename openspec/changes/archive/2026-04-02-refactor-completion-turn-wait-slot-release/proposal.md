# Изменение: освободить LSP transport slot до длительного completion `turn_wait`

## Почему
Последний incident bundle `2026-03-26T19:02:29Z` показывает, что previous follow-up `refactor-completion-turn-wait-lifecycle` закрыл только часть проблемы.

Наблюдаемое поведение на live same-file overlap path:
- request `32` копит `16378ms` в `service_future_created -> first_poll`, а среди authoritative contenders виден same-file `textDocument/completion[phase=turn_wait]` того же возраста;
- request `44` получает первый `poll()` почти сразу (`2ms`), но затем проводит `3457ms` внутри handler с dominant stage `turn_wait`;
- оба trace идут по default event-driven completion path и укладываются в pattern `server_before_method_entry_dominant`, хотя current-request heavy completion work почти отсутствует.

Практический смысл: completion request, который уже admitted в LSP service future, но ещё только пассивно ждёт dispatcher turn, продолжает удерживать один из `tower-lsp` transport slots. Даже после исправления inflight-cleanup telemetry stale same-file `turn_wait` completion по-прежнему может создавать seconds-scale ingress backlog для более нового completion.

Это уже не выглядит как проблема type inference, `documentSymbol` isolation или stale telemetry. Нужен completion-scoped fix на service/admission boundary.

Архитектурный follow-up показывает ещё одно ограничение: в текущем `tower-lsp` transport slot живёт столько же, сколько живёт `Service<Request>::Future`, поэтому локальный refactor только внутри completion handler не гарантирует release transport slot. Change должен явно зафиксировать вариант `B`: локальный project-owned handoff на transport/service boundary для default event-driven completion path.

## Что меняется
- Зафиксировать runtime contract, что default event-driven completion MUST NOT удерживать LSP transport admission slot только потому, что request пассивно ждёт dispatcher turn или older same-file holder.
- Ввести completion-scoped handoff boundary до длительного `turn_wait`, сохранив existing same-file latest-wins/cancel semantics и normal LSP response contract.
- Зафиксировать, что этот handoff реализуется через локальный transport/service adaptation для default event-driven completion path в явном entry point: `backend/src/bin/lsp_server/main.rs` перестаёт напрямую звать `tower_lsp::Server::serve(...)`, а project-owned scheduling boundary живёт в новом модуле `backend/src/bin/lsp_server/server/transport_adapter.rs` и экспортируется через `server::serve_with_completion_handoff(...)`.
- Зафиксировать ownership contract после handoff: request id, cancellation token, terminal outcome и право отправить ровно один terminal response MUST иметь один completion-owned lifecycle owner.
- Уточнить authoritative observability contract, чтобы operator видел разницу между:
  - реальным ingress backlog до first poll;
  - completion-owned wait после handoff;
  - stale contender в `phase=turn_wait`.
- Добавить отдельный regression/gate слой для сценария, где current completion first-poll bounded, но older same-file turn owner заставляет request ждать секунды вне transport slot retention, а также для race windows `handoff -> cancel/supersede -> terminal cleanup`.

## Влияние
- Затронутые спецификации:
  - `bsl-intellisense-v2`
- Затронутый код (implementation follow-up):
  - `backend/src/bin/lsp_server/main.rs`
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - `backend/src/bin/lsp_server/server/mod.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `contracts/lsp-completion-timeline/*`
  - `vscode-extension/src/providers/completionTimeline*`
  - `vscode-extension/src/providers/observabilityIncidentBundle*`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - validation/readiness wrappers, checked-in evidence и runbook docs

## Не-цели
- Не делать общий scheduler redesign для всех LSP methods.
- Не лечить проблему приоритизацией, простым увеличением `concurrency_level` или другим pressure-relief workaround.
- Не делать full custom/forked LSP runtime для всех request classes, если scoped project-local adaptation default completion path достаточна.
- Не переоткрывать `documentSymbol` isolation и auxiliary request traffic как отдельную причину.
- Не менять shipped non-default completion modes шире, чем это необходимо для сохранения default-path contract.
