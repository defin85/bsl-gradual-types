# Change: phase 1 довести completion output egress split до truthful `v23`

## Why

После `investigate-completion-transport-gap` blind spot сузился до server-only окна между `response_sent_at_ms` и `response_flush_completed_at_ms`.

Bundle `2026-04-03T22:08:07Z` показывает:

- `request=53`: `server_handler_exec_ms=2`, но `response_ready_to_flush_wait_ms=3125`;
- `request=41`: `server_handler_exec_ms=155`, но `response_ready_to_flush_wait_ms=2843`;
- `request=34`: кроме `query_bundle_ir_query=3491`, ещё `response_ready_to_flush_wait_ms=2229`;
- post-flush transport и client-after-receive хвост уже почти нулевые.

Первый shipped шаг (`v22`/`v19`) закрыл bounded clocks и surfaces, но acceptance-review показал semantic mismatch:

- `response_output_write_started_at_ms` сейчас фиксируется до `serde_json::to_vec(...)`, а не перед первым фактическим `write`;
- из-за этого `response_output_queue_wait_ms` фактически включает encode time, хотя spec обещает literal wait до write start;
- retroactive reinterpretation `v22` недопустима, потому что downstream уже может полагаться на shipped field names.

Текущий `v22` payload по-прежнему полезен как compatibility surface, но для truthful split должен появиться новый additive контракт, который честно отделяет:

- wait до постановки completion response в outbound path;
- queue wait до старта output encode/write phase;
- encode/serialize exec;
- first actual write и write+flush exec.

Этот шаг должен довести change до 100% без transport refactor и без guessed backlog attribution: сохранить shipped `v22` как legacy compatibility layer и добавить truthful `v23`.

## What Changes

- authoritative completion timeline поднимается `22 -> 23`, а contiguous contract baseline поднимается `contracts/lsp-completion-timeline/v19 -> v20`;
- `v23` сохраняет semantics `response_sent_at_ms` и `response_flush_completed_at_ms`, а `v22` остаётся shipped compatibility surface без retroactive reinterpretation;
- `v23` добавляет новый intermediate milestone `response_output_encode_started_at_ms` и фиксирует truthful output-egress boundaries:
  - `response_output_enqueue_completed_at_ms`;
  - `response_output_encode_started_at_ms`;
  - `response_output_write_started_at_ms`;
  - `response_output_encode_completed_at_ms`;
  - `response_ready_to_output_enqueue_wait_ms`;
  - `response_output_queue_wait_ms`;
  - `response_output_encode_exec_ms`;
  - `response_output_write_and_flush_exec_ms`;
- backend публикует `v23` egress milestones атомарно для completion trace, без partial patches;
- Completion Timeline panel, clipboard export и incident bundle summary переносят новый `v23` split в человекочитаемом виде;
- на `v22` surfaces деградируют явно, отмечая, что literal encode-start/write-start split unavailable by design.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Affected code:
  - `backend/src/bin/lsp_server/server/transport_adapter.rs`
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `contracts/lsp-completion-timeline/v20/*`
  - `scripts/check-versioned-contracts.py`
  - `scripts/test-versioned-contracts.py`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle*.ts`
  - focused backend/extension contract tests

## Non-Goals

- не публиковать backlog snapshot (`output_messages_ahead_count`, `output_bytes_ahead_*`, `output_head_blocker_class`) в этой фазе;
- не менять fairness/ordering output path;
- не снижать сам output backlog latency в рамках этого change;
- не переопределять shipped `v22` semantics задним числом.
