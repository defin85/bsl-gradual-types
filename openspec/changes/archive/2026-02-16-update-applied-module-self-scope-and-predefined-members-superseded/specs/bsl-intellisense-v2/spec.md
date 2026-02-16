## ADDED Requirements

### Requirement: Applied object modules MUST резолвить bare identifiers через owner-member fallback
Для `ObjectModule` и `RecordSetModule` система MUST резолвить unqualified identifier в следующем порядке:
1. локальная область (параметры, локальные переменные, module vars),
2. глобальный контекст/коллекции/общие модули,
3. members implicit owner (`ЭтотОбъект`/`Объект`),
4. только затем `UndeclaredVariable`.

Система MUST применять этот контракт единообразно в `type-at-position`, `diagnostics`, `hover`, `completion`.
Система MUST NOT применять этот fallback к `FormModule`; для формы действует отдельный strict form-data контракт.

#### Scenario: Прямой реквизит документа в ObjectModule не считается необъявленным
- **GIVEN** `Documents/<Doc>/Ext/ObjectModule.bsl`
- **WHEN** код вызывает `ЗначениеЗаполнено(ДоговорКонтрагента)` без префикса `ЭтотОбъект.`
- **THEN** `ДоговорКонтрагента` резолвится как member типа `ДокументОбъект.<Doc>`
- **AND** диагностика `Необъявленная переменная` не генерируется

### Requirement: Applied owner fallback MUST NOT ослаблять FormModule strict semantics
Включение owner-member fallback для applied object modules MUST NOT возвращать dual-layer поведение в `FormModule`.

#### Scenario: FormModule остаётся strict form-data при включенном applied fallback
- **GIVEN** одновременно активны изменения `remove-form-module-dual-layer-semantics` и данный change
- **WHEN** пользователь запрашивает members для `FormModule.Объект`
- **THEN** выдача строится по strict form-data semantics
- **AND** members из `ДокументОбъект.*` / `СправочникОбъект.*` не подмешиваются fallback-ом applied modules

### Requirement: Системные members metadata object MUST быть доступны при прямом обращении
В applied object modules системные members владельца (`ОбменДанными`, `ДополнительныеСвойства` и эквивалентные object-context members) MUST резолвиться через owner-member fallback даже без явного квалификатора.

#### Scenario: DataExchange и AdditionalProperties в обработчике записи набора записей
- **GIVEN** `InformationRegisters/<Reg>/Ext/RecordSetModule.bsl`
- **WHEN** код использует `ОбменДанными.Загрузка` и `ДополнительныеСвойства.Свойство(...)`
- **THEN** оба идентификатора резолвятся как properties объекта набора записей
- **AND** `UndeclaredVariable` diagnostics отсутствует

### Requirement: Exported manager methods MUST резолвиться через metadata collection path
Exported процедуры/функции manager module MUST быть доступны в резолве вызовов вида `КоллекцияМетаданных.<ИмяОбъекта>.<Метод>(...)`.

#### Scenario: RecordSetModule вызывает exported метод manager module регистра
- **GIVEN** `InformationRegisters/<Reg>/Ext/ManagerModule.bsl` содержит `Функция ВладелецБезопасногоХранилища(...) Экспорт`
- **WHEN** код в `RecordSetModule` вызывает `РегистрыСведений.<Reg>.ВладелецБезопасногоХранилища(...)`
- **THEN** метод успешно резолвится как manager member
- **AND** не генерируется `Undefined function/procedure` или `NonExistentMethod`

### Requirement: Manager facet MUST включать predefined members из конфигурации
Система MUST парсить predefined metadata (`Predefined.xml`/`PredefinedDataName`) и добавлять эти элементы как readonly properties manager-фасета для поддерживаемых metadata kinds.

#### Scenario: Предопределенный счет доступен через ПланСчетовМенеджер
- **GIVEN** конфигурация содержит `ChartsOfAccounts/<Chart>/Ext/Predefined.xml` с элементом `ГотоваяПродукция`
- **WHEN** код обращается к `ПланыСчетов.<Chart>.ГотоваяПродукция`
- **THEN** member резолвится как predefined manager property
- **AND** выражение не даёт `Свойство не существует`

### Requirement: Hover/completion ordering MUST быть детерминированным после merge новых provider-слоёв
После добавления owner-member fallback и predefined manager members система MUST выдавать properties/methods в `hover` и `completion` в стабильном алфавитном порядке.

#### Scenario: Hover для Объект и ЭтотОбъект стабилен по сортировке
- **GIVEN** тип содержит metadata properties, platform members и predefined members
- **WHEN** пользователь запрашивает hover в одном и том же snapshot
- **THEN** порядок properties/methods детерминирован и алфавитный
- **AND** порядок не зависит от внутреннего порядка обхода provider-цепочки

## MODIFIED Requirements

### Requirement: Unified contextual type resolver MUST быть source-of-truth для implicit symbols
Система MUST применять единый резолвер не только для implicit symbol declaration, но и для fallback резолва bare identifiers в owner-context applied modules.

Система MUST NOT иметь safe migration/compatibility режим для этого контракта.

#### Scenario: Breaking-only adoption без fallback на старую undeclared-first модель
- **GIVEN** обновлённый v2 pipeline
- **WHEN** applied object module содержит bare identifier, являющийся member владельца
- **THEN** резолв выполняется через owner-member fallback
- **AND** система не переключается в legacy undeclared-first режим через feature flag
