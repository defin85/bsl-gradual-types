# Change: phase 1 разложить completion output egress на clocks и derived waits

## Why

После `investigate-completion-transport-gap` blind spot сузился до server-only окна между `response_sent_at_ms` и `response_flush_completed_at_ms`.

Bundle `2026-04-03T22:08:07Z` показывает:

- `request=53`: `server_handler_exec_ms=2`, но `response_ready_to_flush_wait_ms=3125`;
- `request=41`: `server_handler_exec_ms=155`, но `response_ready_to_flush_wait_ms=2843`;
- `request=34`: кроме `query_bundle_ir_query=3491`, ещё `response_ready_to_flush_wait_ms=2229`;
- post-flush transport и client-after-receive хвост уже почти нулевые.

Текущий `v21` payload честно показывает coarse server egress tail, но не отделяет:

- wait до постановки completion response в outbound path;
- queue wait до фактического write start;
- encode/serialize exec;
- write+flush exec.

Этот шаг должен дать bounded authoritative clocks без transport refactor и без guessed backlog attribution.

## What Changes

- authoritative completion timeline поднимается `21 -> 22`, а contiguous contract baseline поднимается `contracts/lsp-completion-timeline/v18 -> v19`;
- `v22` сохраняет semantics `response_sent_at_ms` и `response_flush_completed_at_ms`, но добавляет intermediate output-egress milestones и derived waits:
  - `response_output_enqueue_completed_at_ms`;
  - `response_output_write_started_at_ms`;
  - `response_output_encode_completed_at_ms`;
  - `response_ready_to_output_enqueue_wait_ms`;
  - `response_output_queue_wait_ms`;
  - `response_output_encode_exec_ms`;
  - `response_output_write_and_flush_exec_ms`;
- backend публикует эти поля атомарно для completion trace, без partial `v22` patches;
- Completion Timeline panel, clipboard export и incident bundle summary переносят новый `v22` split в человекочитаемом виде;
- на `v21` surfaces деградируют явно, отмечая, что finer egress split unavailable by design.

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
  - `contracts/lsp-completion-timeline/v19/*`
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
- не снижать сам output backlog latency в рамках этого change.
