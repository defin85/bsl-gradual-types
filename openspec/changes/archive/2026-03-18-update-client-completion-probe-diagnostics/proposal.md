# Change: update-client-completion-probe-diagnostics

## Why
Текущий dual-view completion observability уже показывает `Server Timeline` и `Client Probe Feed`, но client-side probes всё ещё недостаточно объясняют проблемные кейсы:
- auto-cancelled completion выглядит как `cancelled`, но не объясняет, был ли запрос superseded более новым completion;
- длинный `ok_empty` probe не раскладывается на client pre-send delay, in-flight/LSP wait и post-response overhead;
- probe не показывает форму результата (`empty_array` vs `empty_list` vs `nullish`) и overlap/version-drift контекст во время жизни запроса.

Из-за этого диагностика остаётся полезной, но не даёт достаточно локальной причинности для разборов long-empty и auto-cancel paths в VS Code extension.

## What Changes
- Расширить bounded/redacted client-side completion probe schema новыми локальными диагностическими полями:
  - `cancel_reason_hint`, `superseded_by_probe_id`, `superseded_after_ms`;
  - transport-phase timestamps/deltas для `client -> LSP -> client`;
  - result-shape summary (`result_kind`, `item_count_bucket`, `is_incomplete` when available);
  - version-drift и overlap summary (`document_version_at_terminal`, `did_change_count_during_probe`, `cursor_moved_during_probe`, `active_completion_count_at_start`, `same_uri_probe_overlap_count`, `newer_probe_started_before_terminal`).
- Обновить `Client Probe Feed` UI/clipboard/model так, чтобы новые поля отображались как local-only debug diagnostics и не подменяли server trace.
- Явно сохранить текущую архитектурную границу:
  - без trace-level correlation с `Server Timeline`;
  - без изменения `bsl.getCompletionTimeline` contract;
  - без protocol-level `client_probe_id` в этом change.

## Impact
- Affected specs:
  - `bsl-intellisense`
  - `bsl-intellisense-v2`
- Affected code:
  - `vscode-extension/src/providers/completionProbe*.ts`
  - `vscode-extension/src/lsp/client/client-options.ts`
  - `vscode-extension/src/providers/completionTimeline*.ts`
  - `vscode-extension/src/test/suite/*completion*`
