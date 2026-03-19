## Context
Сейчас extension уже умеет:
- читать authoritative completion timeline через `bsl.getCompletionTimeline`;
- показывать local-only client probe feed рядом с server timeline;
- показывать compact sidebar metrics и отдельный raw metrics dump в Output.

Проблема не в отсутствии данных, а в отсутствии одного handoff surface для анализа инцидента:
- UI разбит на несколько панелей;
- raw metrics dump не структурирован под внешнего читателя и может быть truncated;
- derived explanation приходится собирать вручную из разных источников.

Пользовательский запрос здесь не про “ещё одну метрику”, а про новый способ упаковки уже существующих observability данных в формат, удобный для передачи AI/внешнему анализатору.

## Goals / Non-Goals
- Goals:
  - Дать один export bundle для incident/AI анализа.
  - Сохранить raw данные отдельно от derived summary.
  - Не менять existing raw requests и не требовать нового server API в первой итерации.
  - Явно различать authoritative server trace, local-only probes и cumulative metrics snapshot.
  - Сделать handoff пригодным как для человека, так и для автоматического анализа.
- Non-Goals:
  - Полный rewrite observability pipeline.
  - Always-on logging в файл или continuous NDJSON capture.
  - Изменение server timeline contract или meaning existing metrics.
  - Замена текущих UI panels новым export surface.

## Alternatives Considered
### 1. Clipboard-only unified text report
- Плюсы:
  - минимальная реализация;
  - не требует file I/O.
- Минусы:
  - снова один большой текст;
  - плох для raw attachments;
  - хуже для машинного анализа и повторного воспроизведения.
- Verdict:
  - rejected как единственный surface; может остаться вторичным convenience flow позже.

### 2. Export bundle в папку с summary + JSON attachments
- Плюсы:
  - хорошо разделяет derived summary и raw evidence;
  - удобен для передачи AI и для последующего сравнения;
  - не требует нового server API;
  - не зависит от Output panel formatting/truncation.
- Минусы:
  - требует явного file export UX;
  - вводит новый versioned export schema.
- Verdict:
  - chosen as primary design.

### 3. Continuous NDJSON/session logging
- Плюсы:
  - лучший формат для длинных прогонов и flaky инцидентов.
- Минусы:
  - слишком тяжёлый scope для первой итерации;
  - требует retention/rotation/privacy decisions;
  - overkill для обычного ручного handoff.
- Verdict:
  - explicitly out of scope for this change.

## Decisions
### 1. Extension-side derived export, а не новый server request
Первая итерация строится полностью в extension:
- raw completion timeline запрашивается через существующий `bsl.getCompletionTimeline`;
- raw observability metrics snapshot запрашивается через существующий `bsl.getObservabilityMetrics`;
- local client probes берутся из уже существующего session-local probe buffer.

Это keeps scope tight и не меняет LSP/server contract только ради export convenience.

### 2. Bundle format: summary + incident JSON + raw attachments
Export bundle должен содержать:
- `summary.md` — короткий human-readable incident summary;
- `incident.json` — machine-readable derived report;
- `raw/completion_timeline.json`;
- `raw/client_probes.json`;
- `raw/observability_metrics.json`.

`summary.md` и `incident.json` являются derived/export layer.
Файлы в `raw/` являются evidence attachments и не должны подменяться форматированным Output dump text.

### 3. Явное разделение типов истины
В `incident.json` и `summary.md` данные должны быть явно разделены по типу доверия:
- `authoritative server timeline`;
- `local-only client probes`;
- `cumulative metrics snapshot`.

Derived report не должен смешивать эти уровни так, будто они эквивалентны. Например:
- metrics snapshot не должен подаваться как per-request truth;
- local probe не должен подменять server stage/outcome;
- missing section должна маркироваться как unavailable, а не реконструироваться.

### 4. Partial export разрешён, fabrication запрещён
Export должен уметь завершаться частично:
- при unsupported `bsl.getCompletionTimeline`;
- при временно недоступных metrics;
- при отсутствии client probes.

Но в bundle должны оставаться:
- capture metadata;
- явные capability flags / unavailable sections;
- derived gaps вместо выдуманных данных.

### 5. Bounded summary, raw detail отдельно
`summary.md` и `incident.json` должны быть bounded и ориентированы на handoff:
- короткие findings;
- focus window;
- ключевые traces/probes;
- список observed gaps.

Полный объём данных остаётся в `raw/*`.
Такой split уменьшает шум без потери forensic evidence.

## Proposed Incident JSON Shape
Минимальный shape первой итерации:

```json
{
  "schema_version": 1,
  "captured_at": "2026-03-19T10:00:00Z",
  "workspace": {
    "name": "bsl-gradual-types"
  },
  "capabilities": {
    "completion_timeline": "available",
    "client_probes": "available",
    "observability_metrics": "available"
  },
  "focus": {
    "uri": "file:///...",
    "server_trace_count": 5,
    "client_probe_count": 5
  },
  "findings": [
    {
      "kind": "transport_to_handler_wait_dominant",
      "evidence": ["completion-trace-4"]
    }
  ],
  "sections": {
    "server_timeline": { "source": "authoritative" },
    "client_probes": { "source": "local_only" },
    "observability_metrics": { "source": "cumulative_snapshot" }
  },
  "gaps": []
}
```

Это versioned export schema extension-side bundle, а не изменение LSP wire contract.

## UX Entry Points
Минимально нужны два user-facing entry point:
- action в Observability tree;
- action в Completion Timeline webview.

Оба entry point должны вызывать один и тот же export pipeline и генерировать идентичный bundle format.

## Risks / Trade-offs
- Риск: summary logic станет слишком “умной” и начнёт переинтерпретировать raw data.
  - Mitigation: bounded findings vocabulary + raw attachments как source of truth.
- Риск: пользователи начнут воспринимать export schema как server contract.
  - Mitigation: в design/spec явно отделить extension-side export schema от LSP timeline contract.
- Риск: metrics snapshot остаётся cumulative, а не window delta.
  - Mitigation: явно маркировать metrics как cumulative snapshot; delta/logging оставить вне scope первой итерации.

## Relation To Other Changes
- Этот change совместим с `rewrite-v2-observability-perf-pipeline`, потому что использует существующие raw surfaces и не задаёт новый canonical ingestion path.
- Этот change идейно похож на `add-bsl-agent-compact-diagnostics-mode`, но не должен насильно переиспользовать его schema: здесь приоритет у incident handoff bundle, а не у сокращения одного RPC payload.

## Open Questions
- Нужен ли после первой итерации отдельный clipboard shortcut, который копирует только `summary.md` без файлового export.
- Нужен ли во второй итерации session logging / delta counters для длинных прогонов.
