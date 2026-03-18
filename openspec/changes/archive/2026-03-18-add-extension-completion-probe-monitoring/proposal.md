# Change: add-extension-completion-probe-monitoring

## Why
Текущий server-driven completion timeline уже показывает server-side симптомы (`prepare_timeout`, `exact_deadline`, `head_hit`), но не отвечает на вопрос, что происходило на клиентской границе в цепочке `local edit -> didChange -> completion request -> cancellation/result`.

Это особенно заметно на representative real-module workflow (`examples/conf_big/.../Module.bsl`):
- fast path иногда срабатывает и даёт `head_hit` за единицы миллисекунд;
- но значимая часть запросов уходит в `prepare_timeout` или `exact_deadline`;
- при этом VS Code extension в основном activation path не ведёт per-request completion probe.

Из-за этого Observability panel показывает только server truth, но не даёт быстро отличить:
- клиент отменил запрос до usable ответа;
- completion пришёл слишком близко к свежему `didChange`;
- локальный документ/version уже ушёл вперёд;
- transport/runtime lag накопился ещё до server-side completion pipeline.

## What Changes
- Добавить в VS Code extension bounded in-memory client-side completion probe buffer на основном `LanguageClient` path.
- Расширить Observability completion UI так, чтобы она показывала две независимые поверхности:
  - authoritative `Server Timeline`;
  - local-only `Client Probe Feed`.
- Сохранить server-driven `bsl.getCompletionTimeline` единственным источником истины для server trace, routes, outcomes и stage taxonomy.
- Явно запретить trace-level correlation между local probes и server traces в рамках этого MVP.
- Ограничить MVP локальным debug stream в extension:
  - без нового backend/LSP protocol surface;
  - без persistent telemetry pipeline;
  - без хранения raw document text или unbounded labels.
- Явно отложить exact correlation через `client_probe_id` в отдельный follow-up change.

## Impact
- Affected specs:
  - `bsl-intellisense`
  - `bsl-intellisense-v2`
- Affected code:
  - `vscode-extension/src/lsp/client/client-options.ts`
  - `vscode-extension/src/extension.ts`
  - `vscode-extension/src/providers/completionTimelineModel.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/utils/performance-monitor.ts` or a new dedicated probe store module
  - `vscode-extension/src/test/suite/*completionTimeline*.test.ts`
