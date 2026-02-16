## Context

В текущей реализации implicit symbols уже подмешиваются по `ModuleType`, но `infer_identifier` не использует metadata owner-members как fallback для unqualified identifier. Это ломает runtime-совместимые сценарии 1С в `ObjectModule`/`RecordSetModule`, где прямой доступ к реквизитам и системным свойствам объекта является нормой.

Дополнительно, manager member-модель не включает predefined элементы, потому что parser/RawTypeData не несут данные из `Predefined.xml`. В результате выражения вида `ПланыСчетов.Хозрасчетный.ГотоваяПродукция` ошибочно диагностируются как отсутствующее свойство.

## Goals / Non-Goals

- Goals:
  - Синхронизировать static resolution с runtime-контекстом applied object modules.
  - Убрать ложные `UndeclaredVariable` для прямых обращений к members metadata object.
  - Добавить first-class поддержку predefined manager members.
  - Сделать порядок members в hover/completion детерминированным и алфавитным.
- Non-Goals:
  - Введение compatibility toggle/safe migration слоя.
  - Сохранение старого поведения undeclared-first для applied object modules.

## Decisions

### Decision 1: Identifier resolution pipeline для applied object modules

Для `ObjectModule`, `RecordSetModule` (и совместимых applied object contexts) `infer_identifier` MUST работать по приоритету:
1. локальные переменные/параметры/module vars;
2. глобальные коллекции/глобальный контекст;
3. explicit common module type;
4. **implicit owner member fallback** (lookup в members owner-типа `ЭтотОбъект`/`Объект`);
5. `TypeResolution::undeclared_variable`.

Это гарантирует runtime-совместимость bare identifier обращений (`ДоговорКонтрагента`, `ОбменДанными`, `ДополнительныеСвойства`).
Fallback MUST NOT применяться к `FormModule`; для формы действует strict form-data контракт из `remove-form-module-dual-layer-semantics`.

### Decision 2: Единый source-of-truth owner-member lookup

Fallback для bare identifier и member-resolution (`obj.member`) MUST использовать общий provider в `TypeMetadataLookup`, чтобы diagnostics/hover/completion/type-at-position видели идентичный набор members.

### Decision 3: Predefined items как manager properties

Добавляется модель `predefined_items` (в parser + raw type data):
- источник: `Ext/Predefined.xml` (и/или `PredefinedDataName`);
- проекция: readonly properties manager-фасета;
- целевой тип значения: соответствующий `*Ссылка.<Имя>` для metadata kind.

Для `ChartOfAccounts` это даёт `ПланыСчетов.<Имя>.<ПредопределенныйСчет>`.

### Decision 4: Merge precedence и ordering

При сборке properties manager-фасета:
1. platform properties;
2. metadata-derived properties;
3. predefined properties.

При конфликте имён приоритет выше у более раннего слоя (platform > metadata > predefined), а вывод в hover/completion сортируется по Unicode case-insensitive имени с детерминированным tie-break (original case).

### Decision 5: Breaking-only adoption

Изменение принимается как breaking:
- без feature flag;
- без safe migration режима;
- без fallback к старой undeclared-first семантике.

### Decision 6: Coordination with form strict-semantics change
Изменения этого документа не должны ослаблять ограничения `FormModule.Объект`:
- без object-facet fallback в форме;
- без implicit подмешивания `DocumentObject/CatalogObject` members в `FormModule.Объект`.

Совместные regression tests MUST проверять, что applied fallback и predefined manager members не меняют form-only контракт.

## Alternatives considered

- Оставить текущий undeclared-first и лечить только отдельные системные имена (`ОбменДанными`, `ДополнительныеСвойства`) whitelist-ом.
  - Отклонено: не масштабируется и ломается на прикладных реквизитах (`ДоговорКонтрагента` и т.д.).
- Ввод compatibility toggle для staged rollout.
  - Отклонено: пользователь явно требует breaking-only архитектуру без safe migration.

## Risks / Trade-offs

- Увеличение числа разрешаемых bare identifiers может скрыть реальные опечатки, если member lookup станет слишком широким.
  - Митигация: fallback активировать только для module kinds с object context; сохранять strict precedence local vars > owner members.
- Риск регрессии формы: неявное распространение fallback в `FormModule`.
  - Митигация: явный guard по `ModuleType` + кросс-regression с `remove-form-module-dual-layer-semantics`.
- `Predefined.xml` формат может иметь вариации между версиями платформы.
  - Митигация: parser должен быть tolerant и покрыт fixture-тестами для нескольких конфигураций.
- Добавление новых manager properties может менять выдачу completion и baseline тестов.
  - Митигация: фиксированный deterministic sort и snapshot tests.

## Migration Plan

Миграционный режим отсутствует (breaking-only).
После внедрения:
1. обновить regression baseline;
2. удалить/переписать тесты, ожидающие `UndeclaredVariable` для bare metadata members;
3. зафиксировать новую семантику в `bsl-intellisense-v2`.

## Open Questions

- Нужна ли поддержка predefined members для всех metadata kinds сразу (`Catalog`, `ChartOfAccounts`, `ChartOfCharacteristicTypes`, `ChartOfCalculationTypes`) или поэтапно начиная с `ChartOfAccounts`?
- Нужно ли в hover помечать происхождение свойства (`predefined`) отдельным тегом в detailed-режиме?
