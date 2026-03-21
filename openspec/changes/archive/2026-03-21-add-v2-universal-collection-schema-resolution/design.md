## Context
Сейчас три активных OpenSpec change описывают три близких сценария:

1. `Соответствие[key]` должно резолвить тип значения по local effects.
2. `Структура.<Поле>` должна резолвить snapshot-local схему полей экземпляра.
3. `СтрокаТаблицыЗначений.<Колонка>` должна резолвить snapshot-local схему колонок таблицы.

Во всех трёх случаях корневая проблема одна:
- `analysis-v2` вычисляет `TypeResolution` по месту, но не хранит per-instance effect/state;
- `completion`, `hover` и `semantic diagnostics` уже завязаны на единый v2 snapshot/type hints;
- `TypeMetadataLookup` пока знает только repository/facet/intrinsic источники свойств и не умеет раскрывать per-instance structural schema.

## Goals / Non-Goals
- Goals:
  - Ввести единый snapshot-local architecture contract для universal value collections.
  - Использовать один source of truth для `completion`, `hover`, `type-at-position`, `semantic diagnostics`.
  - Не мутировать глобальный `TypeRepository` synthetic-типами.
  - Сохранить предсказуемую strict/safe-degrade политику по каждому объекту.
- Non-Goals:
  - Полное межфайловое/межмодульное dataflow-распространение schema.
  - Поддержка произвольной runtime reflection и динамических имён без статической опоры.
  - Полноценный path-sensitive доказательственный анализ существования ключа в `Соответствие`.

## Architecture Drivers
- Единый v2 snapshot как source of truth.
- Минимизация расхождений между completion и diagnostics.
- Контролируемая производительность: без мутации global repository, без глобального synthetic type lifecycle.
- Ясная доставляемость: одна merge/alias policy вместо трёх независимых.

## Options Considered

### Option A: Три независимые overlay-модели
- Плюсы: можно двигать каждую тему локально.
- Минусы: дублирование merge/alias/invalidation logic, высокий риск расхождения consumers.

### Option B: Unified snapshot-local effect store
- Идея:
  - хранить instance-local effects в одном состоянии snapshot;
  - materialize разные выходы:
    - `Соответствие[key]` -> value resolution;
    - `Структура` -> typed structure resolution;
    - `ТаблицаЗначений` -> typed-row resolution.
- Плюсы: один source of truth, одна merge policy, один consumer contract.
- Минусы: нужно расширить `TypeResolution` / `TypeMetadataLookup`.

### Option C: Synthetic types в `TypeRepository`
- Плюсы: reuse существующего lookup API.
- Минусы: сложный lifecycle, риск утечки состояния между snapshot, плохой fit для incremental invalidation.

## Architecture Lock
Этот change нормирует реализацию однозначно:
- реализация MUST использовать только `Option B`;
- `Option A` и `Option C` MUST NOT использоваться ни как финальная архитектура, ни как временный compatibility path;
- реализация MUST сначала строить единый v2 resolved-type path в `analysis-v2`, и только затем подключать consumers;
- реализация MUST NOT добавлять consumer-local schema inference, который обходит общий snapshot contract.

Под `Option A` в рамках этого change понимаются:
- отдельные overlay/state-модели для `Соответствие`, `Структура`, `ТаблицаЗначений`;
- независимые merge/alias/invalidation policy по коллекциям;
- локальная реализация schema resolution только в `completion`, только в `hover` или только в `diagnostics`.

Под `Option C` в рамках этого change понимаются:
- synthetic `RawTypeData` или synthetic concrete types, зарегистрированные в глобальном `TypeRepository` ради per-instance schema;
- любой shared mutable cache, который переживает границы одного v2 snapshot и хранит instance-local collection schema.

## Decisions
- Decision: принять и зафиксировать только `Option B`.
  - Why: это единственный вариант, который одновременно согласуется с текущим v2 pipeline, сохраняет один source of truth и не создаёт глобального mutable state.

- Decision: использовать unified snapshot-local `InstanceEffectStore`.
  - Store MUST поддерживать:
    - map effects (`generic V` + literal-key specializations),
    - structure field schema,
    - value-table column schema.
  - Store MUST быть единственной точкой хранения per-instance collection schema внутри snapshot.

- Decision: `TypeRepository` MUST NOT мутироваться для per-instance schema.
  - Why: snapshot-local semantics не должны жить в глобальном shared cache.

- Decision: consumer contract MUST идти через один resolved type path.
  - `completion`, `hover`, `type-at-position`, `semantic diagnostics` используют один и тот же `TypeResolution` / type hints contract.
  - Любой consumer-local fallback MAY существовать только как thin adapter над этим resolved type path и MUST NOT иметь отдельную schema/effect логику.

- Decision: strict/safe-degrade policy различается по доменам:
  - `Соответствие`:
    1. literal-key specialization;
    2. generic `V`;
    3. `Произвольный`;
    4. dynamic key -> без hard-fail "ключ не найден".
  - `Структура`:
    - field остаётся в schema даже если тип не вычислен;
    - unknown field typed-structure -> hard-fail.
  - `ТаблицаЗначений`:
    - column остаётся в schema даже если тип не вычислен;
    - unknown column typed-row -> hard-fail.

- Decision: merge policy первой версии должна быть deterministic и ограниченной.
  - Для конфликтов типов допускается `union` или certainty downgrade.
  - Для ветвлений и alias поддерживаются только простые snapshot-local сценарии.

## InstanceEffectStore Contract
`InstanceEffectStore` хранится только в границах одного v2 snapshot и не сериализуется в глобальные shared-кэши.

Identity и ключи:
- `InstanceId` MUST включать стабильную привязку к snapshot и локальному происхождению значения.
- Минимальный состав `InstanceId` первой версии:
  - `snapshot_id`,
  - `symbol_key` (normal form имени переменной),
  - `scope_id`,
  - `creation_span_start`.

Store shape (v1):
- `MapEffects`:
  - `generic_key_type: Option<TypeResolution>`,
  - `generic_value_type: Option<TypeResolution>`,
  - `literal_keys: Map<NormalizedLiteralKey, ValueEffectEntry>`.
- `StructureEffects`:
  - `fields: Map<NormalizedMemberName, MemberEffectEntry>`.
- `ValueTableEffects`:
  - `columns: Map<NormalizedMemberName, MemberEffectEntry>`.

Entry shape:
- `canonical_name: String`,
- `value_type: TypeResolution`,
- `source_span: Span`,
- `certainty: Certainty`.

Normalization rules:
- ключи/имена MUST сравниваться регистронезависимо;
- для отображения MUST сохраняться `canonical_name` из первого валидного source.

## Merge And Alias Policy (v1)
Общая цель v1: детерминированный merge без path-explosion.

Alias (поддерживаемый минимум):
- Простое присваивание `B = A` MUST переносить ссылку на тот же `InstanceId` внутри текущего snapshot.
- Переопределение `A = <new expr>` MUST создавать новый `InstanceId` для `A`.

Branch merge (`Если`, `Попытка/Исключение`):
- Для одинаковых members/keys в обеих ветках:
  - тип MUST объединяться (`union`) с downgrade certainty при конфликте.
- Для members/keys, существующих только в одной ветке:
  - запись MAY сохраняться как доступная в merged snapshot с downgraded certainty.
- Merge MUST быть детерминированным и не зависеть от порядка обхода AST.

Unknown member policy после merge:
- `Соответствие` dynamic key: без hard-fail (safe degrade).
- typed `Структура` unknown field: hard-fail.
- typed-row unknown column: hard-fail.

## Consumer Contract Enforcement
`completion`, `hover`, `type-at-position`, `semantic diagnostics` MUST читать owner/type из одного resolved path.

Запрещено:
- consumer-local schema inference на AST/text chain как источник истины;
- отдельные overlay/state для отдельных consumers.

Допустимо:
- thin-adapter преобразование формата ответа (LSP/UI), если оно не меняет owner/type resolution.

## Rejected Approaches
- Rejected: реализовывать `Соответствие`, `Структура` и `ТаблицаЗначений` отдельными архитектурными ветками.
  - Reason: это ломает единый contract и увеличивает риск расхождения consumers.
- Rejected: закрывать change через расширение глобального `TypeRepository` synthetic instance-local типами.
  - Reason: это несовместимо со snapshot-local semantics и incremental invalidation.
- Rejected: сначала доставить локальную поддержку только в `completion`, а затем “дотянуть” `hover` и `diagnostics`.
  - Reason: capability spec требует единый v2 pipeline, а не поэтапные divergent paths.

## Unified Model
1. `analysis-v2` строит `InstanceEffectStore` внутри snapshot.
2. `Expression::IndexAccess` для `Соответствие` читает map effects и возвращает resolved value type.
3. `Expression::PropertyAccess` над typed `Структура` / typed-row использует structural members, а не repository-only metadata.
4. `TypeMetadataLookup.get_properties(...)` сначала должен уметь раскрывать structural members из `TypeResolution`, потом repository/facet/intrinsic layers.
5. `semantic-diagnostics` и `completion` используют уже готовый resolved owner type, без отдельных AST-only эвристик.

## Implementation Outline
1. `analysis-v2`
   - добавить unified effect-state в локальное окружение snapshot;
   - собирать effects из:
     - `Новый Соответствие`, `Вставить`, `Установить`;
     - поддерживаемых паттернов `Новый Структура(...)`, `Вставить`;
     - `ТЗ.Колонки.Добавить(...)`;
   - materialize typed results для `map[key]`, typed `Структура`, typed-row.
2. `bsl-types`
   - расширить модель `TypeResolution` / concrete type contract для structural members.
3. `shared`
   - расширить `TypeMetadataLookup` и `TypeValidator` structural-member support.
4. `semantic-diagnostics`
   - оставить единый path через `member_access_object_type_by_span`.
5. `bsl-runtime`
   - completion/hover должны использовать тот же resolved type path и не расходиться с diagnostics.

## Test Strategy
- Unit:
  - `Соответствие`: literal key, generic `V`, fallback `Произвольный`.
  - `Структура`: known field, unknown field, unknown field type -> `Произвольный`.
  - `ТаблицаЗначений`: column schema registration, typed-row, fallback `Произвольный`.
- Integration:
  - completion на `map["k"].`, `S.`, `Стр.`.
  - hover/type-at-position на `map["k"]`, `S.Поле`, `Стр.Колонка`.
  - diagnostics для unknown field / unknown column.
- Regression:
  - существующие generic index-access tests остаются зелёными;
  - новый unified change не ломает snapshot consistency, требуемую `add-lsp-functional-ga-readiness`.

## Rollout And Rollback
Rollout MUST идти через единственный production path Option B и наблюдаемость. Для этого change проект больше не использует отдельный runtime feature-flag: новый universal-collection contract всегда живёт на общем `intellisense-v2` пути и защищается acceptance/perf gates, а не параллельным переключателем поведения.

Rollout steps:
1. Держать один production path для `completion` / `hover` / `type-at-position` / `semantic diagnostics`; не добавлять consumer-local fallback path специально для `Соответствие` / `Структура` / `ТаблицаЗначений`.
2. Перед merge прогонять exact cross-consumer acceptance и strict-policy suite для `map["k"]`, `S.<Поле>`, `Стр.<Колонка>`.
3. Подтверждать стабильность по существующему scale-aware gate, который потребляет экспортируемые `intellisense_v2_*` latency/counter metrics.
4. После merge наблюдать тот же always-on path через telemetry и baseline reports; если gate начинает падать, откатывать commit, а не вводить временный feature flag.

Rollback triggers:
- любой провал exact acceptance/regression tests для `map["k"]`, `S.<Поле>` или `Стр.<Колонка>`;
- провал `backend/src/perf_gate_evaluator.rs` по large/small completion ratios, cancelled rate или ratio `completion_fallback_unavailable / interactive_wait_budget_exhausted`;
- воспроизводимый drift в stage latency / stale-fallback counters на том же fixture set и baseline report.

Rollback action:
- revert offending commit(s), восстановление последнего passing baseline report и повторный прогон acceptance/perf gate suite;
- не добавлять отдельный feature-flagged path или consumer-local workaround ради rollback.

## Observability Contract
Минимальный observability contract для этого change совпадает с существующим `intellisense-v2` telemetry surface и его perf-gate projections.

Обязательные histograms:
- `intellisense_v2_wait_for_file_version_completion_ms`,
- `intellisense_v2_snapshot_completion_ms`,
- `intellisense_v2_ir_query_completion_ms`,
- `intellisense_v2_syntax_diagnostics_query_ms`,
- `intellisense_v2_semantic_diagnostics_query_ms`,
- `intellisense_v2_singleflight_wait_ms`.

Обязательные counters:
- `intellisense_v2_interactive_wait_budget_exhausted_total`,
- `intellisense_v2_interactive_stale_served_total`,
- `intellisense_v2_completion_stale_fallback_total`,
- `intellisense_v2_completion_fallback_unavailable_total`,
- `intellisense_v2_singleflight_leader_total`,
- `intellisense_v2_singleflight_shared_total`,
- `intellisense_v2_singleflight_key_unavailable_total`,
- `intellisense_v2_revision_lag_sample_total`.

Drilldown contract MUST сохранять dimensions `origin`, `operation`, `completion_mode`, `stage` через `intellisense_v2_drilldown_stage_total_*` и `intellisense_v2_drilldown_stage_latency_ms_*`.

Rollback thresholds закреплены в существующем gate:
- `large wait ratio <= 0.60`;
- `large completion ratio <= 0.75`;
- `small completion ratio <= 1.25`;
- `cancelled rate <= 0.10`;
- если stale fastpath был задействован, то `completion_fallback_unavailable / interactive_wait_budget_exhausted <= 0.20`.

## Risks / Trade-offs
- Риск: перетащить слишком много state в каждый `TypeResolution`.
  - Mitigation: держать store snapshot-local, а structural materialization делать только на границах результата.
- Риск: completion продолжит использовать свой fallback path и разойдётся с diagnostics.
  - Mitigation: обновлять не только owner hint path, но и recursive `member_resolution`.
- Риск: merge policy станет слишком сложной.
  - Mitigation: первая версия ограничивается простыми alias и deterministic policy.
