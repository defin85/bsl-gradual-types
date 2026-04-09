## Context
Новый observability patch уже показал, что current representative non-member completion tail больше не является “непонятным collect”, а концентрируется в трёх immutable deps-wide source families:

- global functions;
- repository types;
- metadata items.

Сейчас [completion_service.rs](/home/egor/code/bsl-gradual-types/bsl-runtime/src/application/type_system/services/completion_service.rs) каждый раз:

- итерирует эти deps-wide families заново;
- materialize-ит `Candidate` values для всего набора;
- передаёт их в ranking, где prefix discrimination происходит уже после этой materialization.

Для warm path это нерационально: источники immutable относительно deps/settings snapshot, а request-specific остаётся только prefix/context filtering и последующее rank/format.

## Goals
- Снизить warm non-member collect latency.
- Уменьшить needless per-request materialization для immutable deps-wide families.
- Сохранить существующую correctness model и source-priority ordering.
- Сделать collect improvement acceptance-testable отдельно от readiness/runtime contention.

## Non-Goals
- Не менять member-access owner resolution.
- Не переносить local/contextual/module-routine logic в immutable catalog.
- Не менять public completion protocol.

## Decisions

### 1. Catalog precompute живёт на deps/settings snapshot boundary
Observed hotspot зависит не от file revision, а от metadata/deps surface. Значит immutable catalog должен быть привязан к deps/settings snapshot или semantically equivalent identity.

Это даёт:

- reuse across many warm requests одного и того же deps snapshot;
- честную invalidation, когда metadata/deps реально меняются;
- отсутствие stale cross-deps leakage.

### 2. Prefix-aware filtering происходит до полной materialization
Текущая схема materialize-everything-then-rank wasteful для запросов с уже известным prefix.

После change immutable catalog может хранить lightweight templates/normalized labels и:

- быстро отфильтровывать candidates по prefix;
- materialize-ить полные `Candidate` только для surviving subset;
- передавать дальше уже уменьшенный ranking input.

### 3. Dynamic families остаются dynamic
Local symbols, contextual symbols и module routines остаются revision/context-sensitive. Их не нужно пытаться “затолкать” в immutable catalog, иначе change распухнет и рискует сломать correctness.

### 4. Acceptance идёт по collect substeps, а не только по total completion
Optimization должна быть видна в уже существующей collect observability:

- `collect_non_member_global_functions_ms`;
- `collect_non_member_repository_types_ms`;
- `collect_non_member_metadata_items_ms`;
- aggregate `collect_ms`.

Иначе regression/улучшение снова растворятся в total completion latency.

## Alternatives Considered

### A. Оптимизировать только ranking
Rejected.

Observed hotspot уже находится до ranking, внутри collect substeps.

### B. Поднять completion budget и оставить rebuild per request
Rejected.

Это не root-cause fix, а tolerance increase.

### C. Кэшировать уже готовые `Vec<Candidate>` per request shape
Rejected.

Слишком взрывоопасно по cardinality и invalidation surface. Snapshot-scoped immutable template catalog проще и устойчивее.

## Risks
- Ошибка в invalidation может дать stale deps-wide candidates.
- Prefix prefilter может accidentally изменить candidate set, если нормализация будет неэквивалентна текущему ranking input.
- Memory footprint immutable catalogs вырастет.

## Mitigations
- Ключевать catalog по deps/settings snapshot.
- Сравнить prefilter output с текущим path deterministic tests.
- Хранить lightweight templates/normalized labels, а не fully materialized heavyweight structs, где это возможно.
