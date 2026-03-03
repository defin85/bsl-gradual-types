## Context
`refactor-v2-event-driven-type-index-cache` закрыл функциональный разрыв между ingest-time precompute и serve-only interactive path, но оставил несколько contract-level уязвимостей к drift:
- observability normalize/projection опирается на string mapping в нескольких местах;
- retention semantics формально описана как `N=2`, но реализация сейчас описывается через version-gap и может интерпретироваться неоднозначно;
- emission serve outcomes (`type_index_*`) реализован не как единый интерактивный контракт по всем операциям;
- perf evidence traceability может расходиться с активным `change_id`.

## Goals / Non-Goals
- Goals:
  - Сделать observability mapping drift-resistant by construction через единый typed источник truth.
  - Зафиксировать детерминированную retention semantics для `TypeIndexArtifact`.
  - Унифицировать emission serve outcomes для всех interactive операций.
  - Сделать perf artifacts воспроизводимо привязанными к активному `change_id`.
- Non-Goals:
  - Полный rewrite observability/perf pipeline (выделен в отдельный change).
  - Редизайн алгоритмов type inference / completion ranking.
  - Изменение пользовательского LSP wire-контракта.
  - Массовый рефакторинг несвязанных diagnostics/components.

## Decision Statement
Для этого change выбран и зафиксирован подход:
- `Contract-first hardening` как целевая стратегия текущего этапа;
- `registry-driven materialization` для канонической нормализации и legacy projection;
- `fail-closed provenance` для perf evidence (provided `change_id` mismatch MUST invalidate artifact).

Этот change не выполняет полный rewrite pipeline; rewrite ведётся отдельным планом с отдельным scope/рисками.

## Decision Log
### DS-01 Registry ownership
Единый typed registry для `operation/stage/reason` размещается в shared runtime (`bsl-runtime`) как single source of truth.
Canonical normalization и legacy projection MUST строиться только из registry; adapter-local/manual string mapping считается архитектурным нарушением.

### DS-02 Retention contract
`max_versions_per_file_identity = N` трактуется строго как "latest N версий" для ключа `(file_id, deps_id, settings_id)`.
Retention MUST NOT зависеть от неявной version-gap эвристики.

### DS-03 Global guard protection
Global guard eviction MUST NOT удалять latest exact artifact для текущего `(file_id, latest_version, deps_id, settings_id)`.
При конфликте eviction priority выбирает кандидата вне latest exact.

### DS-04 Unified serve outcomes
`completion`, `hover`, `signatureHelp`, `definition` MUST эмитить `type_index` serve outcomes из единой bounded taxonomy:
- `type_index_exact_hit`;
- `type_index_stale_served`;
- `type_index_degraded_incomplete`;
- `type_index_fallback_unavailable`.

### DS-05 Definition policy consistency
`Definition` приводится к interactive policy-модели (`queue priority`, `freshness knobs`, serve-only semantics), согласованной с `completion`/`hover`/`signatureHelp`.

### DS-06 Provenance v1 migration
В `intellisense-perf-gate` `v1` provenance-поля добавляются как backward-compatible optional:
- `change_id`;
- `generated_at`;
- `profile`;
- `schema_version`;
- `contract_version`.

### DS-07 Fail semantics for optional provenance
Для `v1` отсутствие provenance-полей допустимо только в legacy-local режиме, когда invocation context НЕ задаёт `expected_change_id`.
Если `expected_change_id` задан, validator MUST требовать `change_id` в report и MUST проверять формат/консистентность;
missing/mismatch/invalid provided provenance MUST приводить к fail-closed verdict.

### DS-08 Active change_id source
Источником активного change-id объявляется invocation context с фиксированным приоритетом:
1. `--change-id` CLI argument (authoritative для CI/gate);
2. `OPENSPEC_CHANGE_ID` environment variable;
3. отсутствие значения => legacy-local режим (артефакт невалиден для cutover evidence).

Hardcoded `change_id` в production/perf report path запрещён.

### DS-09 Rollout guardrail
Cutover запрещён при `parity_drift_rate > 0.01` (1.0%) по warm-path gate.
Определение:
- `parity_drift_rate = parity_drift_total / parity_pairs_total`;
- evidence валиден только при `parity_pairs_total >= 100` (иначе fail-closed как insufficient evidence).

Rollback readiness MUST быть подтверждён canary-сценарием до merge.

## Architecture
### 1) Observability Taxonomy Hardening
Вводится единый typed registry для runtime stage/reason taxonomy:
- canonical labels;
- legacy projection keys;
- allowed contract lists.

Ключевой инвариант:
- canonical normalization и legacy projection MUST строиться из одного registry;
- добавление нового stage/reason без записи в registry MUST детектироваться тестами до merge.

### 2) Deterministic Retention Semantics
Retention для `TypeIndexArtifact` фиксируется как count-based контракт, а не как косвенная version-gap эвристика:
- `max_versions_per_file_identity = 2` означает "latest + previous";
- версия старше окна MUST удаляться детерминированно;
- global guard eviction MUST NOT удалять актуальный exact artifact latest key.

### 3) Unified Serve Outcome Emission
Serve-only `type_index` outcome emission централизуется в едином контракте для:
- `completion`;
- `hover`;
- `signatureHelp`;
- `definition`.

Инварианты:
- reason labels только из bounded taxonomy;
- unknown reason -> `other` + contract-violation signal.

### 4) Perf Evidence Traceability
Perf/gate артефакты получают provenance-контур с optional-полями `v1`:
- `change_id` из invocation context;
- `generated_at`;
- `profile`;
- `schema_version`;
- `contract_version`.

Hardcoded foreign `change_id` в gate path запрещается.
В `v1` отсутствие provenance поля допустимо только без `expected_change_id` (legacy-local, non-authoritative);
при заданном `expected_change_id` missing/mismatch/invalid provenance MUST приводить к fail-fast validation.

## Alternatives Considered
- Локальные точечные патчи по каждому найденному mismatch.
  - Rejected: устраняет симптомы, но сохраняет systemic drift риск.
- Сохранение string mapping + расширение тестов без typed registry.
  - Rejected: повышает покрытие, но не даёт единого источника truth.

## Risks / Trade-offs
- Риск: accidental breaking changes в observability keys.
  - Mitigation: dual-write migration policy + versioned contract checks/changelog.
- Риск: переход на retention count-based может изменить memory profile.
  - Mitigation: explicit tests + perf smoke на churn/large сценариях.
- Риск: расширение serve-outcome emission увеличит число counters.
  - Mitigation: только low-cardinality reason set, unknown collapse в `other`.

## Migration Plan
1. Зафиксировать requirements и design как contract baseline.
2. Внедрить typed registry и переподключить projection.
3. Привести retention implementation к count-based контракту.
4. Унифицировать serve outcome emission для всех interactive операций.
5. Обновить perf gate traceability и validation.
6. Обновить versioned contracts/changelog и пройти contract tests.
