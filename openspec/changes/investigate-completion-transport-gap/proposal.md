# Change: изолировать completion transport/client gap через flush-aware timeline и probe receive split

## Почему

Incident bundle `2026-04-03T13:48:00Z` показывает, что perceived UX tail не сводится к одному server hot path:

- `request=41`: server timeline `3249ms`, но correlated client probe видит `9918ms`;
- `request=36`: server timeline `2605ms`, а client probe видит `10759ms`;
- derived summary уже различает `client_to_transport_wait_ms` и большой post-response хвост, но этот хвост всё ещё слишком coarse.

Текущие bounded факты недостаточны, чтобы ответить, где именно живут лишние `6-8s`:

- `response_sent_at_ms` сейчас означает handler-ready boundary, а не реальный transport flush completion;
- client probe `lspResponseReceivedAtMs` фактически совпадает с promise resolution и не отделяет raw receive от downstream extension-host work;
- из-за этого `server_to_client_post_response_ms` остаётся непрозрачным смешанным bucket'ом.

Нужен отдельный observability/investigation change, чтобы разложить этот gap на server egress, transport-after-flush и client-after-receive части без guessed reconstruction.

## Что меняется

- authoritative completion timeline поднимается `20 -> 21`, а contiguous contract baseline поднимается `contracts/lsp-completion-timeline/v17 -> v18`;
- `response_sent_at_ms` сохраняет текущую handler-ready semantics, а server payload получает additive flush-aware boundary (`response_flush_completed_at_ms`) и self-contained wait field для post-handler server egress;
- client-side completion probes получают явный split между LSP dispatch, raw transport response receive, promise resolve и client terminal;
- Completion Timeline panel, clipboard export и incident bundle summary перестают опираться на один opaque post-response bucket и начинают переносить новый `v21` gap split в человекочитаемом виде;
- на `v20` и на старых probe paths surfaces деградируют явно, отмечая, что post-response gap unresolved by design, а не "server виноват" или "No gaps were recorded".

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `contracts/lsp-completion-timeline/v18/*`
  - `vscode-extension/src/lsp/client/client-options.ts`
  - `vscode-extension/src/lsp/client/completionProbeRuntime.ts`
  - `vscode-extension/src/providers/completionProbeRecorder.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle*.ts`
  - focused backend/extension contract tests

## Non-Goals

- не снижать server exact IR latency самим этим change;
- не обещать заранее, что источник gap обязательно в extension host, transport или stdio flush;
- не переосмыслять старые поля задним числом и не ломать legacy payload.
