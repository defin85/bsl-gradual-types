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
  - Редизайн алгоритмов type inference / completion ranking.
  - Изменение пользовательского LSP wire-контракта.
  - Массовый рефакторинг несвязанных diagnostics/components.

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
Perf/gate артефакты получают обязательный provenance-контур:
- `change_id` из invocation context;
- `generated_at`, `profile`, `schema_version`.

Hardcoded foreign `change_id` в gate path запрещается. Mismatch MUST приводить к fail-fast validation.

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

## Open Questions
- Нужно ли поднимать `contracts/observability-completion-v2` `schema_version` при сохранении key names, но усилении provenance/validation правил.
