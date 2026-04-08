# Change: phase 2 добавить truthful completion output backlog attribution

## Why

После phase 1 (`investigate-completion-output-egress-split-phase-1`) timeline уже сможет отделять enqueue/queue/encode/write+flush интервалы.

Следующий blind spot останется внутри `response_output_queue_wait_ms`: текущая архитектура writer path не хранит truthful metadata о том, какие outbound message уже стоят впереди completion response и какой coarse blocker class удерживает head.

Архитектурное ревью показало, что без unified outbound envelope/queue backlog snapshot будет guessed, потому что output loop сейчас merge-ит разные источники сообщений и не несёт достаточной metadata для authoritative culprit attribution.

## What Changes

- change строится поверх phase 1 и поднимает authoritative completion timeline `22 -> 23`, а contiguous contract baseline `contracts/lsp-completion-timeline/v19 -> v20`;
- backend вводит unified bounded outbound envelope path для writer instrumentation и truthful completion correlation;
- `v23` сохраняет все `v22` clocks/derived waits и добавляет bounded backlog snapshot:
  - `output_messages_ahead_count`;
  - `output_bytes_ahead_estimate`;
  - `output_head_blocker_class`;
- payload и surfaces объясняют `response_output_queue_wait_ms` через authoritative ahead snapshot, не выдавая estimate за exact flushed byte count;
- на `v22` surfaces деградируют явно, отмечая, что backlog attribution unavailable by design.

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

- не менять admission/fairness policy output path;
- не снижать output backlog latency самим этим change;
- не публиковать raw payload fragments, request labels или unbounded queue dumps.
