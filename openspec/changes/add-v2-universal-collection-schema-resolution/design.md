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

## Risks / Trade-offs
- Риск: перетащить слишком много state в каждый `TypeResolution`.
  - Mitigation: держать store snapshot-local, а structural materialization делать только на границах результата.
- Риск: completion продолжит использовать свой fallback path и разойдётся с diagnostics.
  - Mitigation: обновлять не только owner hint path, но и recursive `member_resolution`.
- Риск: merge policy станет слишком сложной.
  - Mitigation: первая версия ограничивается простыми alias и deterministic policy.
