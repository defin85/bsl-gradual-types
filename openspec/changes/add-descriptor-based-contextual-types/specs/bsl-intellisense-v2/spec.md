## MODIFIED Requirements

### Requirement: v2 context-aware implicit symbols для модулей определяются по ModuleType/фасету (MUST)
Система MUST определять и подмешивать platform implicit symbols в v2 pipeline на основе модульного контекста (`ModuleType`) и фасета владельца метаданных через descriptor-based semantic model.

Система MUST представлять implicit типы структурно (descriptor-based), а не только строковым именем типа.
Descriptor MUST сохранять контекст, достаточный для детерминированного преобразования в `TypeResolution` (минимум: owner metadata, module context, required facet, form context при наличии).

Система MUST использовать единый descriptor-based контракт для:
- AST→IR symbol registration;
- type inference seeding;
- последующей семантической диагностики undeclared variable и member-resolution.

#### Scenario: Единые правила не расходятся между AST→IR и type inference
- **GIVEN** файл анализируется через v2 pipeline
- **WHEN** система строит symbols и type hints для одинакового snapshot
- **THEN** один и тот же implicit symbol считается объявленным и имеет согласованный тип во всех этапах pipeline
- **AND** при преобразовании descriptor -> `TypeResolution` сохраняется ожидаемый facet context

### Requirement: FormModule предоставляет фиксированный набор implicit symbols (MUST)
Для `FormModule` система MUST предоставлять следующие implicit symbols:
- `ЭтотОбъект`
- `ЭтаФорма`
- `Форма`
- `Объект`
- `Элементы`
- `Параметры`

Типы MUST вычисляться контекстно через descriptor-based модель:
- `ЭтотОбъект`, `ЭтаФорма`, `Форма` -> дескриптор контекста формы с user-facing представлением `Формы.<Коллекция>.<Объект>.<Форма>`;
- `Объект` -> form-data descriptor, связанный с applied object фасетом владельца для guaranteed members;
- `Элементы` -> дескриптор контейнера элементов формы с user-facing представлением `ЭлементыФормы.<Коллекция>.<Объект>.<Форма>`;
- `Параметры` -> `Структура`.

Система MUST NOT использовать `ДанныеФормыОбъект.*` как внутренний semantic source of truth для `FormModule.Объект`.
Canonical semantic interpretation для `FormModule.Объект` MUST соответствовать form-data semantics (`ДанныеФормыСтруктура`), даже если user-facing label использует owner object facet.

#### Scenario: `ПриСозданииНаСервере` использует `ЭтотОбъект` и `Параметры` без undeclared diagnostic
- **GIVEN** модуль формы документа содержит вызов `...ПриСозданииНаСервереДокумент(ЭтотОбъект, Параметры)`
- **WHEN** клиент запрашивает semantic diagnostics
- **THEN** система не возвращает diagnostics `Необъявленная переменная` для `ЭтотОбъект` и `Параметры`

## ADDED Requirements

### Requirement: Descriptor-aware member resolution для FormModule.Объект является детерминированным (MUST)
Для `FormModule.Объект` система MUST выполнять member-resolution через отдельный descriptor-aware provider chain в фиксированном порядке:
1. form shape (реквизиты формы и привязанные элементы/ТЧ по контексту формы),
2. guaranteed members applied object (например, `Ссылка` для документа),
3. applied facet fallback.

Система MUST деградировать в `InferredWeak` при отсутствии достаточных метаданных вместо ложных `NonExistentProperty`.

#### Scenario: `Объект.Ссылка` в форме документа не даёт ложный `NonExistentProperty`
- **GIVEN** код формы документа обращается к `Объект.Ссылка`
- **WHEN** выполняется v2 semantic diagnostics
- **THEN** система не возвращает диагностику `Свойство 'Ссылка' не существует`
- **AND** type-at-position для `Объект` резолвится через descriptor-based контекст формы

### Requirement: Legacy form-object alias не участвует в descriptor-based semantic contract (MUST)
Система MUST трактовать `ДанныеФормыОбъект.*` только как migration compatibility alias на входе/нормализации.

Система MUST NOT использовать `ДанныеФормыОбъект.*` как canonical semantic type в seed/inference/lookup и MUST NOT показывать его в user-facing результатах (`diagnostics`, `hover`, `completion`, `type-at-position`).

#### Scenario: Пользовательская выдача и внутренний контракт не используют legacy alias как canonical type
- **GIVEN** пользователь запрашивает hover/diagnostics/completion для form-object выражений
- **WHEN** v2 pipeline возвращает результаты
- **THEN** в сообщениях и type labels отсутствует `ДанныеФормыОбъект.*`
- **AND** internal implicit binding/member-resolution используют descriptor-based model, а не legacy alias

### Requirement: User-facing label policy для FormModule.Объект отделён от canonical semantics (MUST)
Система MUST применять dual-layer policy для `FormModule.Объект`:
- internal canonical semantics: form-data descriptor (`ДанныеФормыСтруктура` semantics),
- compact/standard user-facing label: owner object facet (`ДокументОбъект.X`, `СправочникОбъект.X`, и т.д.),
- detailed user-facing label: owner object facet + явная form-data пометка (например, `ДокументОбъект.X (данные формы: ДанныеФормыСтруктура)` или эквивалент).

Система MUST обеспечивать согласованность этой политики между `hover`, `diagnostics`, `completion` и `type-at-position`.

#### Scenario: Compact и detailed режимы отображают согласованные слои семантики
- **GIVEN** выражение `Объект` в `FormModule` документной формы
- **WHEN** пользователь запрашивает тип в compact/standard режиме
- **THEN** label показывает owner object facet (`ДокументОбъект.<ИмяДокумента>`)
- **AND WHEN** пользователь запрашивает detailed представление
- **THEN** вывод содержит owner object facet и явную form-data пометку
