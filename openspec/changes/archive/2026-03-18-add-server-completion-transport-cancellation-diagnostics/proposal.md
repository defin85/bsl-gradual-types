# Change: add-server-completion-transport-cancellation-diagnostics

## Why
После расширения `Client Probe Feed` стало видно, что длинные completion-запросы почти всегда проводят время после отправки из extension, но текущий authoritative `Server Timeline` всё ещё не объясняет, где именно проходит этот хвост:
- server timeline стартует около входа в completion handler и не показывает задержку между transport receive и handler entry;
- timeline не фиксирует, когда сервер впервые увидел cancellation для уже устаревшего запроса;
- в long `ok_empty` и late-cancel кейсах невозможно отделить queue-before-handler от реального долгого handler execution.

Из-за этого текущая диагностика уже полезна, но всё ещё не даёт server-side причинности для разборов `lsp_roundtrip_ms=10s+` и поздно замеченной cancellation.

## What Changes
- Расширить authoritative `bsl.getCompletionTimeline` bounded server-side diagnostics для completion transport/cancellation path:
  - optional `server_edge_details` с server-edge timestamps и derived deltas;
  - bounded transport/cancellation diagnostics для `transport_to_handler_wait` и `cancel observed`.
- Эволюционировать server-driven timeline payload до `response.version=3` и оформить новый versioned surface `contracts/lsp-completion-timeline/v5` с migration note для consumers.
- Обновить VS Code `Server Timeline` model/webview/clipboard так, чтобы server-edge diagnostics были видны пользователю, когда сервер их предоставляет.
- Явно сохранить границы предыдущих changes:
  - без trace-level correlation между `Server Timeline` и `Client Probe Feed`;
  - без `client_probe_id`;
  - без изменения ranking/completion semantics;
  - без расширения общей observability surface вне completion path.

## Impact
- Affected specs:
  - `bsl-intellisense`
  - `bsl-intellisense-v2`
- Affected code:
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/server/command_handlers.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `backend/src/bin/lsp_server/server/core/tests.rs`
  - `contracts/lsp-completion-timeline/v5/*`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimeline*.ts`
  - `vscode-extension/src/test/suite/*completion*`
