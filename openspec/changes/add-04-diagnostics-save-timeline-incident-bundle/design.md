## Context

Сейчас bundle имеет три секции:

- authoritative completion timeline;
- local-only client probes;
- cumulative observability metrics snapshot.

Для диагностик этого недостаточно. `observability_metrics` показывают aggregate stage latencies (`wait_for_file_version`,
`syntax_diagnostics_query`, `semantic_diagnostics_query`) и low-cardinality counters по `trigger/profile`, но они не
связывают эти факты с конкретным `didSave` refresh.

После `refactor-03` именно `didSave` refresh стал user-facing seam:

- нужен first publish;
- нужен optional final publish;
- нужен clear answer, применялся ли `save_fastlane`;
- нужен truthful root cause по одному refresh, а не по process-wide histogram.

## Goals

- Дать authoritative request-centric trace для `didSave` diagnostics refresh.
- Отдельно показать `save_fastlane` first publish и `idle_heavy` follow-up.
- Сохранить bundle AI-friendly и bounded, без raw text и high-cardinality payload.
- Не ломать existing completion-centric incident workflow.

## Non-Goals

- Не строить full per-keystroke diagnostics timeline для каждого `didChange`.
- Не заменять existing cumulative metrics snapshot.
- Не добавлять client-side guessed correlation для diagnostics trace в первой итерации.

## Proposed Shape

### 1. New server request

Добавить новый custom request, например `bsl/getDiagnosticsSaveTimeline`, который возвращает bounded recent traces
для `didSave`-triggered diagnostics refresh.

Payload должен быть server-authored и request-centric, а не derived из metrics snapshot.

### 2. Trace model

Один trace соответствует одному save refresh cycle и содержит:

- `trace_id`
- `uri`
- `requested_version`
- `diagnostics_generation`
- `trigger=did_save`
- `started_at_ms`
- `first_publish_profile` (`save_fastlane|idle_heavy`)
- `first_publish_elapsed_ms`
- `first_publish_kind` (`syntax_only|full`)
- `first_publish_outcome` (`published|cancelled|no_publish`)
- bounded stage timings для first publish:
  - `wait_for_file_version_ms`
  - `snapshot_with_deps_ms`
  - `syntax_diagnostics_query_ms`
  - `semantic_diagnostics_query_ms`
  - `publish_wait_ms`
- optional follow-up section:
  - `followup_profile`
  - `followup_publish_elapsed_ms`
  - `followup_publish_kind`
  - `followup_outcome`
- terminal outcome / cancellation reason.

### 3. Grouping semantics

`save_fastlane` и `idle_heavy` не должны экспортироваться как два несвязанных traces. Bundle должен видеть один
save refresh cycle, внутри которого:

- first publish является canonical user-visible freshness boundary;
- `idle_heavy` остаётся optional follow-up для richer/final result.

Это означает, что серверу нужен save-cycle grouping key, который связывает оба профиля одного `didSave`.

### 4. Bundle integration

Incident bundle получает четвёртый источник:

- `diagnostics_save_timeline` с raw attachment `raw/diagnostics_save_timeline.json`.

`summary.md` и `incident.json` должны:

- показывать source status для diagnostics save timeline;
- включать request-centric summary для captured save refresh traces;
- не пытаться выводить save trace из `observability_metrics`, если новый источник недоступен.

## Tradeoffs

### Why a new request instead of reusing metrics

Metrics snapshot cumulative по процессу. Он годится для трендов и p95, но не отвечает на вопрос:
`какой именно didSave завис и где именно`.

### Why separate request instead of extending completion timeline

Completion timeline уже имеет другой lifecycle, другой UI vocabulary и другой source of truth.
Смешивание completion и diagnostics save traces в одном контракте усложнит и server, и extension projection.

### Why only didSave in v1

`didChange`-path diagnostics живёт в high-frequency churn зоне. Request-centric timeline для него сильно повышает
volume и cardinality. `didSave` даёт наибольший операторский выигрыш при контролируемом размере trace buffer.

## Risks

- Неправильное grouping key может снова сделать `save_fastlane` и `idle_heavy` несвязанными для одного save-cycle.
- Если trace будет строиться из post-factum heuristics, а не из explicit runtime milestones, bundle снова получит
  guessed reconstruction.
- Слишком большой raw payload сломает bounded incident handoff; нужны retention limit и low-cardinality поля.
