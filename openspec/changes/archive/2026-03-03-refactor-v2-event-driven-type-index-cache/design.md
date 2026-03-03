## Context
Интерактивный completion v2 уже работает в event-driven режиме по документам, но часть type lookup нагрузки может попадать в on-demand query путь с повторным parse/index во время пользовательского запроса.

Для больших модулей при churn это создает нестабильный хвост latency (особенно p99), даже когда ingestion уже построил version-bound `ParseSnapshot`.

Целевая архитектура: разделить контуры на ingest-time precompute и request-time serve-only.

## Goals / Non-Goals
- Goals:
  - Сделать `didOpen/didChange` единственной точкой запуска precompute `type_index`.
  - Исключить синхронный parse/type-index compute из интерактивного request path.
  - Сохранить latest-wins и bounded поведение под burst `didChange`.
  - Сохранить correctness через version-bound ключи cache и parity rollout.
- Non-Goals:
  - Полный отказ от текущего legacy path на первом шаге.
  - Рефакторинг всех diagnostics алгоритмов в рамках одного change.

## Architecture
### 1) Artifact Model
Вводится явный артефакт:
- `TypeIndexArtifactKey`:
  - `file_id`
  - `file_version`
  - `deps_id`
  - `settings_id`
- `TypeIndexArtifact`:
  - `type_index`
  - `build_profile`
  - `produced_at_millis`
  - `parse_snapshot_meta` (`incremental`, `fallback_reason`, `changed_ranges_count`)

Нормативный ключ кэша:
- `TypeIndexArtifactKey(file_id, file_version, deps_id, settings_id)` MUST быть единственным индексом exact serving.
- Для всех interactive serving операций (`completion|hover|signatureHelp`) MUST использоваться только exact-key или policy fallback.

### 2) Ingest-Time Precompute
- На `didOpen/didChange` после получения `ParseSnapshot(version=V)` orchestrator планирует precompute job.
- Job вычисляет `TypeIndexArtifact(key(file_id, V, deps_id, settings_id))`.
- Для одного `file_id` действует latest-wins: precompute для старых версий superseded/cancelled.

### 3) Serve-Only Request Path
Для `completion/hover/signatureHelp` type lookup:
- Путь читает только cache-артефакт по текущему `(file_id, version, deps_id, settings_id)`.
- Если exact-hit отсутствует:
  - разрешен bounded fallback (`stale/degraded_incomplete/fallback_unavailable`) по policy;
  - тяжелый parse/index sync compute в request path запрещен.

### 4) Invalidation Rules
- Любое изменение `deps_id` или `settings_id` инвалидирует artifacts текущего файла для старых ключей.
- Любая новая версия файла инвалидирует exact serving для старых `file_version`.
- Внутри `file_id` публикация user-facing ответа возможна только для актуального epoch/version.

### 4.1) Retention / Eviction Policy
- Per-file window retention: хранить не более `N=2` последних `file_version` артефактов на `(file_id, deps_id, settings_id)`.
- Global guard: суммарное число artifacts в процессе ограничивается `MAX_ARTIFACTS=10_000` с LRU-eviction по `produced_at_millis`.
- MUST first-evict superseded artifacts старых версий файла, затем stale artifacts с неактуальным `deps_id/settings_id`.
- Eviction MUST NOT удалять актуальный exact artifact для latest `(file_id, version, deps_id, settings_id)`.
- При исчерпании global guard eviction MUST быть детерминированным и observability-visible (reason code + counters).

### 4.2) Reason-Code Taxonomy (v1)
- `type_index_exact_hit`
- `type_index_stale_served`
- `type_index_degraded_incomplete`
- `type_index_fallback_unavailable`
- `type_index_precompute_superseded`
- `type_index_precompute_cancelled`
- `type_index_precompute_queue_saturated`
- `type_index_artifact_invalidated_deps`
- `type_index_artifact_invalidated_settings`
- `type_index_artifact_evicted_global_guard`
- `type_index_artifact_evicted_per_file_window`

### 5) Rollout
- `shadow`: считаем новый путь параллельно, ответ пользователю от legacy, собираем parity drift.
- `canary`: serve-only для части трафика/профилей (`large+churn` first).
- `on`: serve-only default.
- rollback: флаг mode возвращает legacy без изменения wire-контракта.

## Decision Rationale
- Precompute на ingest устраняет root-cause хвостов: тяжелая стадия выносится из user-facing запроса.
- Serve-only упрощает SLO: predictable bounded behavior без случайных дорогих query re-entry.
- Version-bound ключи решают correctness и mixed-state риски.

## Alternatives Considered
- Inline hotfix в completion (`if snapshot then build now`):
  - Rejected: уменьшает latency локально, но оставляет дрейф архитектуры и не дает четкого serving-контракта.
- Полный eager precompute всего semantic graph:
  - Rejected: слишком высокий риск/стоимость и рост ресурсоемкости.

## Perf/Observability Contract
MUST публиковаться low-cardinality метрики:
- precompute: queue wait, exec, build profile buckets;
- serving source: `exact`, `stale`, `degraded_incomplete`, `fallback_unavailable`;
- supersede/cancel причины precompute jobs;
- stage attribution для completion path без on-demand parse/index стадий.

Gate evidence MUST отдельно фиксировать churn-профиль и отсутствие секундных хвостов request-time type lookup.

## Risks / Mitigations
- Риск: cache miss всплески после deps/settings switch.
  - Mitigation: prewarm и bounded degraded fallback.
- Риск: drift между legacy/new semantics.
  - Mitigation: shadow parity + explicit reason-codes и rollback mode.
- Риск: memory pressure от artifact cache.
  - Mitigation: version window retention + per-file eviction.

## Open Questions
- Нужен ли отдельный precompute priority class для больших модулей, чтобы не ухудшить small-path under load.
- Нужен ли per-workspace global budget на количество хранимых artifacts помимо per-file retention.
