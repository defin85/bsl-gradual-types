## 1. Spec and contract
- [x] 1.1 Обновить `bsl-intellisense-v2` contract так, чтобы authoritative completion timeline включал bounded `server_edge_details`, response `version=3` и migration note для нового versioned surface.
- [x] 1.2 Обновить `bsl-intellisense` requirement для Observability UI так, чтобы `Server Timeline` отображал server-edge transport/cancellation diagnostics и extension оставался совместим с legacy `version=2` payload без этих полей.

## 2. Backend instrumentation
- [x] 2.1 Добавить server-edge capture points для completion request: transport receive, handler entry, response send и optional cancel observed, не меняя completion semantics.
- [x] 2.2 Добавить bounded derived deltas и completion-specific observability metrics для `transport_to_handler_wait`, `server_handler_exec` и `cancel observed`, без high-cardinality labels.
- [x] 2.3 Эмитить `server_edge_details` в `bsl.getCompletionTimeline` traces и обновить versioned contract artifacts (`contracts/lsp-completion-timeline/v5`).

## 3. VS Code extension
- [x] 3.1 Обновить `customRequests`, timeline model, webview и clipboard так, чтобы server-edge diagnostics отображались в `Server Timeline`, когда они доступны.
- [x] 3.2 Сохранить backward-compatible поведение extension для payload `version=2`: older server timeline продолжает рендериться без server-edge diagnostics и без деградации `Client Probe Feed`.

## 4. Validation
- [x] 4.1 Добавить/обновить backend tests для `response.version=3`, `server_edge_details`, late-cancel capture и bounded transport/cancellation metrics.
- [x] 4.2 Добавить/обновить extension tests для rendering/export/backward-compatibility server-edge diagnostics.
- [x] 4.3 Прогнать targeted backend/extension verification и `openspec validate add-server-completion-transport-cancellation-diagnostics --strict --no-interactive`.
