# Change: add-observability-incident-bundle-export

## Why
Сейчас данные для разбора IntelliSense/observability инцидентов уже существуют, но они разнесены по нескольким user-facing поверхностям:
- `Completion Timeline` webview с server timeline и local client probes;
- Observability sidebar с compact metrics view;
- Output dump `Dump Raw Metrics to Output`, который удобен для локальной отладки, но шумный, cumulative по природе и легко обрезается `... (truncated)`.

Для человека это уже перегруженный набор панелей. Для внешнего AI-анализа ситуация ещё хуже:
- приходится копировать данные из двух разных UI мест;
- correlation между server trace, local probes и metrics snapshot восстанавливается вручную;
- raw metrics dump не отделён от derived summary;
- отсутствует единый машиночитаемый export, который можно сохранить в файл и передать на разбор.

Нужен отдельный export/report слой поверх уже существующих observability поверхностей: один компактный AI-friendly bundle, который сохраняет raw данные отдельно, но даёт короткий incident summary для анализа.

## What Changes
- Добавить в VS Code extension явный export observability incident bundle для AI/внешнего анализа.
- Зафиксировать bundle как derived/export surface поверх уже существующих источников:
  - authoritative server timeline из `bsl.getCompletionTimeline`;
  - local-only client probes из extension runtime;
  - observability metrics snapshot из `bsl.getObservabilityMetrics`.
- Добавить bundle structure с:
  - кратким human-readable `summary.md`;
  - machine-readable `incident.json`;
  - отдельными raw attachments без truncation.
- Явно разделить в exported report:
  - authoritative server trace;
  - local-only client probes;
  - cumulative metrics snapshot.
- Сохранить текущие панели, copy flow и raw dump как существующие debug surfaces; новый export должен быть additive и не заменять их.
- Явно оставить вне scope этого change:
  - новый server-side custom request специально для AI export;
  - always-on file logging / continuous NDJSON session logging;
  - изменение существующего meaning/shape server timeline contract.

## Impact
- Affected specs:
  - `bsl-intellisense`
- Affected code (planned):
  - `vscode-extension/src/providers/observabilityProvider.ts`
  - `vscode-extension/src/providers/completionTimelineWebview.ts`
  - `vscode-extension/src/providers/completionTimelineClipboard.ts`
  - `vscode-extension/src/providers/completionTimelineModel.ts`
  - `vscode-extension/src/commands/observability.ts`
  - `vscode-extension/src/lsp/customRequests.ts`
  - `vscode-extension/src/test/suite/*observability*`
  - `vscode-extension/src/test/suite/*completionTimeline*`
- User-facing impact:
  - вместо ручного копирования из нескольких панелей пользователь получает один export bundle для AI/incident анализа;
  - raw debug данные не исчезают, но становятся приложением к summary, а не единственным форматом handoff.

## Relation To Existing Changes
- `rewrite-v2-observability-perf-pipeline` остаётся отдельным архитектурным rewrite change. Этот change не меняет observability pipeline boundary и не требует нового LSP контракта; он строит derived/export слой поверх уже существующих surface-ов.
- `add-bsl-agent-compact-diagnostics-mode` решает похожую задачу уменьшения шума, но в другой системе (`bsl-agent` MCP diagnostics). Этот change не навязывает общий schema с `bsl-agent` и ограничен VS Code observability UX/export.
