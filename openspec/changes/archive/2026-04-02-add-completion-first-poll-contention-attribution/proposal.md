# Change: bounded first-poll contention attribution для completion timeline

## Почему
Incident bundle, экспортированный `2026-03-25T09:40:08Z`, уже дал сильный server-side сигнал, что проблема находится до входа в completion handler:
- у trace `request=46` зафиксированы `service_future_to_first_poll_wait_ms=13707` и `server_handler_exec_ms=104`;
- у trace `request=60` зафиксированы `service_future_to_first_poll_wait_ms=3393` и `server_handler_exec_ms=6`.

Текущий `v11` contract уже умеет честно разделять:
- `service_future_created -> first_poll`;
- `first_poll(Pending) -> first_wake`;
- дальнейший handler/runtime tail.

Но этого всё ещё недостаточно для incident handoff. Operator видит, что future долго не poll'илась, однако не видит, какой класс server-side нагрузки был наблюдаем рядом с этим gap. В результате следующий шаг всё ещё требует чтения `RequestContextService`, `tower-lsp` wiring и косвенных выводов по raw evidence.

## Что меняется
- **MODIFIED**: authoritative `bsl.getCompletionTimeline` поднимается до `contract=v12`.
- **ADDED**: bounded `first_poll_contention_attribution` внутри `server_edge_details`, который даёт server-side contender facts для сегмента `service_future_created -> first_poll` без free-text и high-cardinality payload.
- **ADDED**: existing completion surfaces переносят новый `v12` fact в Completion Timeline panel, clipboard export и request-centric incident bundle summary без guessed blocker claims.
- **ADDED**: graceful degradation на `v11` фиксируется явно; отсутствие `v12` attribution не маскируется как "gaps отсутствуют".

## Не входит в scope
- Исправление самого latency/starvation инцидента.
- Полный rewrite observability/perf pipeline.
- Новый custom request вне `bsl.getCompletionTimeline`.
- Ослабление deterministic probe-to-trace correlation или inventing blocker из client-side probes.

## Влияние
- Затронутые спеки:
  - `bsl-intellisense-v2`
  - `bsl-intellisense`
- Затронутый код:
  - `backend/src/bin/lsp_server/server/request_context.rs`
  - `backend/src/bin/lsp_server/server/language_server/helpers.rs`
  - `backend/src/bin/lsp_server/server/language_server/impl_completion.rs`
  - `backend/src/bin/lsp_server/types.rs`
  - `contracts/lsp-completion-timeline/v9/*`
  - `scripts/check-versioned-contracts.py`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/completionTimelineDrilldown.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundleRequests.ts`
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - `vscode-extension/manual-lsp-test.md`

## Связь с существующими change
- `rewrite-v2-observability-perf-pipeline` остаётся отдельным большим rewrite-треком и не заменяется этим change.
- Этот change продолжает узкую линию `v8 -> v9 -> v10 -> v11` completion timeline attribution и добавляет следующий bounded diagnostic cut без смены transport/API surface.
