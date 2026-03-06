## 1. Specification
- [ ] 1.1 Добавить единый spec delta в `bsl-intellisense-v2`, который покрывает `Соответствие`, `Структура` и `ТаблицаЗначений` как один snapshot-local contract.
- [ ] 1.2 Зафиксировать unified consumer contract для `completion` / `hover` / `type-at-position` / `semantic diagnostics`.
- [ ] 1.3 Зафиксировать strict/safe-degrade policy:
  - `Соответствие`: literal-key -> generic `V` -> `Произвольный`, без hard-fail по динамическому ключу;
  - `Структура`: unknown field typed-structure -> hard-fail;
  - typed-row `ТаблицаЗначений`: unknown column -> hard-fail.

## 2. Design
- [ ] 2.1 Зафиксировать `Option B` как единственно допустимый implementation approach для этого change.
- [ ] 2.2 Описать unified snapshot-local модель effect/state для universal value collections.
- [ ] 2.3 Описать materialization policy:
  - `Соответствие` -> resolved value type для `map[key]`;
  - `Структура` -> typed structure resolution;
  - `ТаблицаЗначений` -> typed-row resolution.
- [ ] 2.4 Явно запретить `Option A` и `Option C`:
  - без отдельных overlay-моделей по коллекциям;
  - без synthetic per-instance типов в глобальном `TypeRepository`;
  - без consumer-local schema inference в обход общего v2 contract.
- [ ] 2.5 Описать merge/alias policy для простых присваиваний и ветвлений.
- [ ] 2.6 Описать интеграцию с `TypeMetadataLookup` и `TypeResolution` без мутации глобального `TypeRepository`.

## 3. Validation
- [ ] 3.1 `openspec validate add-v2-universal-collection-schema-resolution --strict --no-interactive`
- [ ] 3.2 Проверить, что acceptance/review для этого change отклоняет реализации вне `Option B`.
- [ ] 3.3 Архивировать superseded change и добавить явные `SUPERSEDED.md`.
- [ ] 3.4 Review change с владельцами `analysis-v2`, `completion`, `diagnostics`, `metadata_lookup`.
