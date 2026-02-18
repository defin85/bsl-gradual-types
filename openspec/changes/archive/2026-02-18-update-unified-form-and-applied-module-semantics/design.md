## Context

Текущая спецификация разделена на два change, но технически изменения входят в один контур:
- identifier resolution (`infer_identifier`),
- metadata lookup provider chain,
- projection результата в completion/hover/diagnostics/type-at-position.

При раздельном применении легко получить регрессию: owner fallback applied modules может случайно вернуть object-facet примеси в `FormModule.Объект`.

## Goals / Non-Goals

- Goals:
  - Единая и недвусмысленная матрица семантики по `ModuleType`.
  - Runtime-совместимый strict form-data контракт для `FormModule`.
  - Runtime-совместимый owner fallback для applied object contexts.
  - First-class поддержка predefined manager members.
  - Детерминированный порядок members во всех consumers.
- Non-Goals:
  - Safe migration / feature-flag режим.
  - Частичный контракт с различающимся поведением между consumers.

## Decisions

### Decision 1: Матрица правил по module kind (единый контракт)

- `FormModule`:
  - `ЭтотОбъект/ЭтаФорма/Форма` -> form context.
  - `Объект` -> strict form-data.
  - object-facet fallback запрещен.
- `ObjectModule`/`RecordSetModule`/совместимые applied object contexts:
  - `infer_identifier` применяет порядок
    `local -> global -> explicit common module -> owner members -> undeclared`.
- `Manager path-call`:
  - `КоллекцияМетаданных.<Имя>.<ExportMethod>(...)` должен резолвиться через manager module export index.

### Decision 2: Единый source-of-truth lookup

Owner-member fallback (bare identifiers) и обычный member lookup (`obj.member`) MUST использовать общий `TypeMetadataLookup` путь, чтобы `diagnostics/hover/completion/type-at-position` видели один и тот же набор members.

### Decision 3: Predefined members в manager facet

Parser/model расширяется данными predefined элементов:
- источник: `Ext/Predefined.xml` и/или `PredefinedDataName`;
- проекция: readonly properties manager-фасета;
- тип значения: соответствующий `*Ссылка.<Имя>` metadata kind.

### Decision 4: Precedence и сортировка

Для manager properties merge порядок:
1. platform properties,
2. metadata-derived properties,
3. predefined properties.

При конфликте имени выигрывает более ранний слой.
Вывод в hover/completion сортируется детерминированно: Unicode case-insensitive + stable tie-break по original case.

### Decision 5: Breaking-only adoption

Контракт внедряется без safe migration:
- без feature flag,
- без compatibility fallback path,
- без dual-mode поведения для разных consumers.

## Implementation Considerations

1. Сначала внедрить form strict-guard на уровне lookup chain.
2. Затем добавить applied owner fallback в `infer_identifier` с guard по `ModuleType`.
3. Затем добавить ingestion predefined metadata и manager-facet projection.
4. После этого унифицировать label/ordering policy для hover/completion/diagnostics.
5. Завершить кросс-regression тестами на отсутствие "протечки" applied fallback в форму.

## Risks / Trade-offs

- Риск маскировки опечаток в applied modules при расширении owner fallback.
  - Митигация: строгий precedence `local/global` выше owner fallback.
- Риск регрессии форм при ошибочном применении fallback без guard.
  - Митигация: явная развилка по `ModuleType` + cross-regression tests.
- Риск вариативности `Predefined.xml` между версиями платформы.
  - Митигация: tolerant parser + fixture tests на разных конфигурациях.

## Open Questions

- Включать predefined members сразу для всех поддерживаемых metadata kinds или вводить батчами?
- Нужно ли в detailed hover отдельное помечание origin=`predefined`?
