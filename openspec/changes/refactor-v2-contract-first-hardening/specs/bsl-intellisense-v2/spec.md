## MODIFIED Requirements
### Requirement: Канонический event model является единственным источником observability semantics (MUST)
Система MUST описывать emission observability через единый канонический event model (transport-agnostic), общий для LSP/web/MCP.

Каноническое событие MUST включать:
- `family`;
- `origin`;
- `value`;
- `operation` и `stage` для stage-семейств.

Контекстные измерения (`outcome`, `reason`, `query_kind`, `work_class`) MAY применяться только там, где это разрешено schema правилом `family`.

Недопустимые сочетания измерений MUST NOT публиковаться как отдельные метрики и MUST фиксироваться контрактным сигналом нарушения schema.

Дополнительно для drift-hardening:
- наборы допустимых `operation/stage/reason` MUST задаваться typed registry (single source of truth);
- canonical normalization и legacy projection MUST строиться из этого же registry;
- добавление нового значения taxonomy без полного mapping MUST детектироваться contract tests до merge.

#### Scenario: Добавление нового stage без registry mapping блокируется валидацией
- **GIVEN** разработчик добавил новый runtime stage в pipeline
- **WHEN** не обновлены typed registry и projection mapping
- **THEN** contract/parity tests падают
- **AND** изменение не может быть принято до восстановления полной deterministic materialization

### Requirement: Dual-write rollout использует единый канонический observability контракт (MUST)
При внедрении drilldown слоя система MUST сохранять backward compatibility fixed-key метрик через dual-write из одного канонического источника событий.

Система MUST соблюдать следующие инварианты:
- канонический контракт задаёт семантику метрик;
- drilldown является primary representation канонического контракта;
- legacy fixed keys являются compatibility-проекцией канонического контракта и MUST NOT иметь отдельную независимую семантику;
- mapping каноника -> fixed keys MUST быть детерминированным и единым для LSP/web/MCP;
- dual-write materialization MUST выполняться в одном centralized projection pipeline (backend-first) в shared runtime;
- adapter-layer MUST NOT публиковать drilldown/legacy метрики напрямую в обход канонического event pipeline.

Дополнительно для precompute observability:
- queue/exec/build для `type_index_precompute` MUST иметь dedicated projection keys;
- эти события MUST NOT сворачиваться в `runtime_other_*` для legacy/canonical представлений;
- projection completeness MUST проверяться контрактным тестом.

#### Scenario: Type-index precompute queue/exec не смешивается с `other`
- **GIVEN** runtime публикует canonical события `type_index_precompute` queue/exec/build
- **WHEN** выполняется dual-write materialization
- **THEN** увеличиваются только dedicated precompute ключи
- **AND** `runtime_other_*` не получает вклад этих событий

## ADDED Requirements
### Requirement: Retention policy для `TypeIndexArtifact` детерминирован и count-based (MUST)
Система MUST трактовать retention policy для `TypeIndexArtifact` как count-based контракт:
- `max_versions_per_file_identity` задаёт точное количество хранимых версий на `(file_id, deps_id, settings_id)`;
- версия окна MUST определяться как "latest N", а не через неявную version-gap эвристику;
- eviction MUST быть детерминированным и observability-visible по reason-code taxonomy.

Global guard eviction MUST NOT удалять актуальный exact artifact latest key для текущего `(file_id, version, deps_id, settings_id)`.

#### Scenario: Version window сохраняет только latest N и защищает latest exact
- **GIVEN** для одного `(file_id, deps_id, settings_id)` построены artifacts версий `V1..V4`
- **AND** configured `max_versions_per_file_identity = 2`
- **WHEN** применяется retention + global guard
- **THEN** в окне остаются только `V4` и `V3`
- **AND** актуальный exact artifact latest key не удаляется

### Requirement: Serve-only `type_index` outcomes публикуются единообразно для всех interactive операций (MUST)
Для интерактивных операций, использующих serve-only type lookup (`completion`, `hover`, `signatureHelp`, `definition`), система MUST публиковать `type_index` serve outcome reason из bounded taxonomy:
- `type_index_exact_hit`
- `type_index_stale_served`
- `type_index_degraded_incomplete`
- `type_index_fallback_unavailable`

Unknown reason labels MUST быть сведены в `other` и сопровождаться контрактным сигналом нарушения, без увеличения cardinality.

#### Scenario: Hover cache miss фиксируется как `type_index_fallback_unavailable`
- **GIVEN** hover запрошен до готовности exact/stale artifact
- **WHEN** serve-only path завершает запрос без on-demand compute
- **THEN** публикуется reason `type_index_fallback_unavailable`
- **AND** reason учитывается в том же low-cardinality контракте, что и completion/signatureHelp/definition

### Requirement: Perf-gate artifacts должны быть traceable к активному `change_id` (MUST)
Perf reports и gate summaries MUST включать `change_id`, полученный из invocation context текущего прогона.

Hardcoded foreign `change_id` в runtime/perf path MUST NOT использоваться.
Mismatch или отсутствие ожидаемого `change_id` MUST приводить к fail-fast validation результата.

#### Scenario: Mismatch `change_id` блокирует принятие perf evidence
- **GIVEN** perf прогон выполняется для change `X`
- **WHEN** сформированный report содержит другой `change_id` или не содержит его
- **THEN** quality-gate validation завершает прогон как invalid evidence
- **AND** артефакт не используется как доказательство прохождения gate
