# Изменение: устранить orphaned completion `turn_wait` lifecycle

## Почему
После `refactor-document-symbol-interactive-isolation` и `refactor-completion-superseded-active-turn-release` остался отдельный completion-scoped blind spot.

Incident bundle `2026-03-26T11:13:29Z` показывает:
- request `35` накапливает `14070ms` до first poll, хотя его собственный `turn_wait` резолвится мгновенно и handler затем выполняется только `132ms`;
- authoritative contenders для того же trace содержат older same-file `textDocument/completion` в `phase=turn_wait` с `age_ms=24037`;
- client probe `probe-1` становится superseded уже через `803ms`, но терминальный результат приходит только через `24561ms`;
- у trace `49` stage `turn_wait=3510ms`, но абсолютные `turn_wait_entered/resolved/wake` timestamps схлопнуты в одну точку времени, то есть текущая observability ещё не даёт полностью truthful reconstruction этого lifecycle.

Это означает, что older completion может выйти из per-file queue и войти в `turn_wait`, но оказаться между уже вытесненными queue entries и ещё не зарегистрированным active completion. В таком состоянии request остаётся inflight слишком долго и создаёт multi-second ingress stall для нового same-file completion.

## Что меняется
- Зафиксировать отдельный runtime contract для same-file completion request, который уже вошёл в dispatcher `turn_wait`, но ещё не стал active: такой request MUST участвовать в latest-wins/cancel lifecycle так же жёстко, как queued и active states.
- Потребовать, чтобы superseded или explicitly cancelled `turn_wait` request boundedly резолвился до active registration и не превращался в orphaned inflight waiter.
- Ужесточить observability contract для `turn_wait`, чтобы authoritative timeline не схлопывал multi-second `turn_wait` stage в нулевую absolute lifecycle и отдельно различал current-request wait от stale contender в `phase=turn_wait`.
- Добавить отдельный overlap regression и representative real-module gate для сценария stale pre-active `turn_wait` request.

## Влияние
- Затронутые спецификации:
  - `bsl-intellisense-v2`
- Затронутый код (implementation follow-up):
  - `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `contracts/lsp-completion-timeline/*`
  - `vscode-extension/src/providers/completionTimeline*`
  - `vscode-extension/src/providers/observabilityIncidentBundle*`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - representative validation/readiness scripts и checked-in overlap evidence

## Не-цели
- Не переоткрывать `documentSymbol` isolation и другие auxiliary LSP methods.
- Не подменять root-cause fix приоритизацией, увеличением concurrency или новым transport/admission workaround.
- Не вводить stale/degraded fallback для completion.
- Не распространять этот change на `hover`, `signatureHelp` или `definition`, пока для них нет отдельной authoritative evidence.
