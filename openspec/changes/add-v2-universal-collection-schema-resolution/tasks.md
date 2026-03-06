## 1. Specification
- [x] 1.1 Добавить единый spec delta в `bsl-intellisense-v2`, который покрывает `Соответствие`, `Структура` и `ТаблицаЗначений` как один snapshot-local contract.
- [x] 1.2 Зафиксировать unified consumer contract для `completion` / `hover` / `type-at-position` / `semantic diagnostics`.
- [x] 1.3 Зафиксировать strict/safe-degrade policy:
  - `Соответствие`: literal-key -> generic `V` -> `Произвольный`, без hard-fail по динамическому ключу;
  - `Структура`: unknown field typed-structure -> hard-fail;
  - typed-row `ТаблицаЗначений`: unknown column -> hard-fail.
- [x] 1.4 Добавить MUST на запрет consumer-local schema/effect inference как источника истины.
- [x] 1.5 Добавить MUST на запрет мутации глобального `TypeRepository` для per-instance schema.
- [x] 1.6 Добавить acceptance scenarios на cross-consumer consistency для одной позиции (`completion`/`hover`/`type-at-position`/`semantic diagnostics`).

## 2. Design
- [x] 2.1 Зафиксировать `Option B` как единственно допустимый implementation approach для этого change.
- [x] 2.2 Описать unified snapshot-local модель effect/state для universal value collections.
- [x] 2.3 Описать materialization policy:
  - `Соответствие` -> resolved value type для `map[key]`;
  - `Структура` -> typed structure resolution;
  - `ТаблицаЗначений` -> typed-row resolution.
- [x] 2.4 Явно запретить `Option A` и `Option C`:
  - без отдельных overlay-моделей по коллекциям;
  - без synthetic per-instance типов в глобальном `TypeRepository`;
  - без consumer-local schema inference в обход общего v2 contract.
- [x] 2.5 Описать merge/alias policy для простых присваиваний и ветвлений.
- [x] 2.6 Описать интеграцию с `TypeMetadataLookup` и `TypeResolution` без мутации глобального `TypeRepository`.
- [x] 2.7 Зафиксировать структуру `InstanceEffectStore` (identity, map/structure/value-table entries, normalization rules).
- [x] 2.8 Зафиксировать rollout/rollback и observability contract (feature-flag, метрики, пороги rollback).

## 3. Implementation Guardrails
- [x] 3.1 Убрать/запретить consumer-local schema/effect inference в `completion_service` и связанных runtime-resolver путях.
- [x] 3.2 Обновить `hover`/`type-at-position`, чтобы они использовали тот же resolved owner/type path, что и diagnostics.
- [x] 3.3 Добавить explicit guardrails против synthetic per-instance типов в глобальном `TypeRepository`.
- [x] 3.4 Добавить интеграционные тесты на cross-consumer consistency для ключевых сценариев (`map["k"]`, `S.<Поле>`, `Стр.<Колонка>`).
- [x] 3.5 Подготовить traceability matrix `Requirement -> Code -> Test` для всех MUST-требований.

## 4. Validation
- [x] 4.1 `openspec validate add-v2-universal-collection-schema-resolution --strict --no-interactive`
- [x] 4.2 Проверить, что acceptance/review для этого change отклоняет реализации вне `Option B`.
- [x] 4.3 Архивировать superseded change и добавить явные `SUPERSEDED.md`.
- [x] 4.4 Review change с владельцами `analysis-v2`, `completion`, `diagnostics`, `metadata_lookup`.
