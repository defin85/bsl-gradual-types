# Change: phase 2 добавить truthful completion output handoff split

## Why

После `investigate-completion-output-egress-split-phase-1` completion timeline уже честно показывает queue/encode/write+flush egress buckets, но свежий production-like dump `2026-04-04` показал новый dominant blind spot до текущего writer-selection boundary:

- `request=77`: `server_handler_exec_ms=5`, но `response_ready_to_output_enqueue_wait_ms=6144`, при этом `response_output_queue_wait_ms=0`, `response_output_encode_exec_ms=0`, `response_output_write_and_flush_exec_ms=1`;
- `request=70`: `server_handler_exec_ms=3748`, но `response_ready_to_output_enqueue_wait_ms=2480`, при этом queue/encode/write buckets нулевые;
- `request=60`: `server_handler_exec_ms=684`, но `response_ready_to_output_enqueue_wait_ms=2442`, при этом queue/encode/write buckets снова близки к нулю.

Это означает, что главный server-side tail сейчас находится не внутри writer queue, а в окне между `response_sent_at_ms` и уже shipped `response_output_enqueue_completed_at_ms`.

Runtime audit при этом показал важную архитектурную деталь: несмотря на историческое имя, `response_output_enqueue_completed_at_ms` сейчас фактически фиксируется в output loop, когда completion response уже выбран writer'ом из merged outbound stream, а не в момент успешного send-side enqueue в `responses_tx`.

Текущий change `investigate-completion-output-backlog-attribution-phase-2` целится в объяснение `response_output_queue_wait_ms`, но по наблюдаемым traces эта метрика сейчас не является dominant blind spot. Сначала нужно truthful разложить post-handler handoff gap до legacy writer-selection seam, а уже потом, при необходимости, расследовать writer backlog attribution.

## What Changes

- authoritative completion timeline поднимается `23 -> 24`, а contiguous contract baseline поднимается `contracts/lsp-completion-timeline/v20 -> v21`;
- `v24` сохраняет shipped `v23` semantics для `response_sent_at_ms`, legacy compatibility seam `response_output_enqueue_completed_at_ms`, `response_output_encode_started_at_ms`, `response_output_write_started_at_ms`, `response_output_encode_completed_at_ms` и `response_flush_completed_at_ms`;
- `v24` явно фиксирует, что `response_output_enqueue_completed_at_ms` остаётся legacy output-writer-selection boundary и не переосмысляется как truthful send-side enqueue acceptance;
- `v24` добавляет truthful send-side handoff milestones:
  - `response_output_handoff_started_at_ms`;
  - `response_output_handoff_enqueued_at_ms`;
- `v24` добавляет truthful derived waits:
  - `response_ready_to_output_handoff_wait_ms`;
  - `response_output_handoff_send_wait_ms`;
  - `response_output_handoff_to_writer_wait_ms`;
- existing `response_ready_to_output_enqueue_wait_ms` сохраняется как compatibility umbrella для legacy интервала `response_sent_at_ms -> response_output_enqueue_completed_at_ms` и не переосмысляется задним числом;
- backend публикует новый `v24` handoff split вместе с existing egress milestones одним atomic patch в authoritative trace store, без partial trace state;
- post-response derivation выносится в shared helper, чтобы trace-store patch path и helper-built completion traces считали одинаковые buckets;
- Completion Timeline panel, clipboard export и incident bundle summary переносят новый `v24` handoff split в человекочитаемом виде и явно помечают legacy seam;
- writer-backlog attribution остаётся отдельным будущим change и не смешивается с этим шагом.

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
  - `contracts/lsp-completion-timeline/v21/*`
  - `scripts/check-versioned-contracts.py`
  - `scripts/test-versioned-contracts.py`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle*.ts`
  - focused backend/extension contract tests

## Non-Goals

- не менять ordering/fairness policy output path;
- не снижать latency самим этим change;
- не публиковать writer-backlog snapshot или culprit-class fields в этой фазе;
- не добавлять live in-flight visibility для незавершённого handoff stall;
- не переопределять shipped `v23` semantics задним числом.
