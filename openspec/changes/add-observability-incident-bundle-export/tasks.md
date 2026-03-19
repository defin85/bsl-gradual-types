## 1. Спецификация и контракт export bundle
- [x] 1.1 Зафиксировать user-facing flow export bundle из observability surfaces VS Code extension.
- [x] 1.2 Зафиксировать bundle structure и file naming:
  - [x] `summary.md`
  - [x] `incident.json`
  - [x] `raw/completion_timeline.json`
  - [x] `raw/client_probes.json`
  - [x] `raw/observability_metrics.json`
- [x] 1.3 Зафиксировать, какие секции считаются authoritative, какие local-only, а какие cumulative snapshot.
- [x] 1.4 Зафиксировать partial-export semantics для legacy/unsupported/unavailable data без выдумывания отсутствующих данных.

## 2. Дизайн extension-side export pipeline
- [x] 2.1 Спроектировать command/UI entry points для export из Observability и Completion Timeline.
- [x] 2.2 Спроектировать extension-side capture pipeline поверх существующих `bsl.getCompletionTimeline` и `bsl.getObservabilityMetrics`, без нового server API.
- [x] 2.3 Спроектировать bounded derived report:
  - [x] capture metadata
  - [x] focus window / summary scope
  - [x] concise findings
  - [x] явные gaps/unavailable sections
- [x] 2.4 Зафиксировать, что raw attachments сохраняются отдельно и не используют truncated Output dump text как источник.

## 3. Валидация и delivery planning
- [x] 3.1 Определить regression coverage:
  - [x] happy-path export bundle
  - [x] partial bundle при unsupported `bsl.getCompletionTimeline`
  - [x] partial bundle при недоступных metrics
  - [x] deterministic file names / schema shape
- [x] 3.2 Подготовить traceability `Requirement -> Code -> Test` для export bundle capability.
- [x] 3.3 Прогнать `openspec validate add-observability-incident-bundle-export --strict --no-interactive`.
