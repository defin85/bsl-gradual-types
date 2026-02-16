## MODIFIED Requirements

### Requirement: FormModule предоставляет фиксированный набор implicit symbols (MUST)
Для `FormModule` система MUST предоставлять следующие implicit symbols:
- `ЭтотОбъект`
- `ЭтаФорма`
- `Форма`
- `Объект`
- `Элементы`
- `Параметры`

Типы MUST вычисляться контекстно через descriptor-based модель:
- `ЭтотОбъект`, `ЭтаФорма`, `Форма` -> runtime-контекст формы (`ФормаКлиентскогоПриложения` + extension главного реквизита + реквизиты формы);
- `Объект` -> strict form-data semantics (`ДанныеФормыСтруктура`);
- `Элементы` -> контейнер элементов формы;
- `Параметры` -> `Структура`.

Система MUST NOT интерпретировать `Объект` как owner object facet source в модуле формы.

#### Scenario: `ЭтотОбъект` и `Объект` в форме имеют разные runtime-слои
- **GIVEN** код в `FormModule` документной формы
- **WHEN** клиент запрашивает hover/type-at-position для `ЭтотОбъект` и `Объект`
- **THEN** `ЭтотОбъект` резолвится как контекст формы
- **AND** `Объект` резолвится как `ДанныеФормыСтруктура`

### Requirement: Unified contextual type resolver MUST быть source-of-truth для implicit symbols и owner fallback
Система MUST применять единый contextual resolver для:
- implicit symbol declaration,
- fallback резолва bare identifiers в applied object contexts,
- member lookup для diagnostics/hover/completion/type-at-position.

Система MUST NOT иметь safe migration/compatibility режим для этого контракта.

#### Scenario: Breaking-only adoption без возврата к legacy undeclared-first
- **GIVEN** обновлённый v2 pipeline
- **WHEN** applied object module содержит bare identifier, являющийся member владельца
- **THEN** резолв выполняется через owner-member fallback
- **AND** система не переключается в legacy режим через feature flag

## ADDED Requirements

### Requirement: Для `FormModule.Объект` v2 MUST использовать платформенную strict form-data модель
Система MUST представлять `Объект` в модуле формы через платформенную form-data семантику (`ДанныеФормыСтруктура` и связанные form-data типы), а не через owner object facet.

Система MUST NOT добавлять в `Объект.` members/методы из `ДокументОбъект.*` / `СправочникОбъект.*` автоматически.

#### Scenario: `Объект.` не подмешивает object-facet методы
- **GIVEN** модуль формы документа
- **WHEN** IDE запрашивает member completion для `Объект.`
- **THEN** выдача не содержит методов, источником которых является object facet applied object
- **AND** выдача ограничена form-data/runtime-формой members

### Requirement: Descriptor-aware member resolution для FormModule.Объект является детерминированным (MUST)
Для `FormModule.Объект` система MUST выполнять member-resolution через form-data-oriented provider chain без applied object facet fallback.

Детерминированная цепочка MUST включать только:
1. members данных формы (shape главного реквизита и связанных данных формы),
2. платформенные members form-data типа.

Система MUST NOT использовать provider-шаг applied object facet fallback для `FormModule.Объект`.

#### Scenario: Детерминированный form-data chain без applied facet fallback
- **GIVEN** `FormModule.Объект` в документной форме
- **WHEN** v2 pipeline строит members для hover/completion
- **THEN** используется только form-data-oriented chain
- **AND** applied object facet fallback не участвует в выдаче

### Requirement: User-facing label policy для FormModule.Объект отделён от owner-facet labels (MUST)
Система MUST использовать для `FormModule.Объект` user-facing label, согласованный с form-data семантикой.

Система MUST NOT отображать `FormModule.Объект` как owner object facet label (`ДокументОбъект.X`, `СправочникОбъект.X`) в compact/full/detailed режимах.

Label policy MUST быть одинаковой между `hover`, `diagnostics`, `completion`, `type-at-position`.

#### Scenario: User-facing label `Объект` согласован с form-data семантикой
- **GIVEN** выражение `Объект` в модуле формы документа
- **WHEN** пользователь запрашивает hover и diagnostics
- **THEN** user-facing type label не использует owner object facet представление
- **AND** вывод согласован между всеми v2 consumers

### Requirement: Applied object modules MUST резолвить bare identifiers через owner-member fallback
Для `ObjectModule` и `RecordSetModule` система MUST резолвить unqualified identifier в следующем порядке:
1. локальная область (параметры, локальные переменные, module vars),
2. глобальный контекст/коллекции/общие модули,
3. explicit common module type,
4. members implicit owner (`ЭтотОбъект`/`Объект`),
5. только затем `UndeclaredVariable`.

Система MUST применять этот контракт единообразно в `type-at-position`, `diagnostics`, `hover`, `completion`.

#### Scenario: Прямой реквизит документа в ObjectModule не считается необъявленным
- **GIVEN** `Documents/<Doc>/Ext/ObjectModule.bsl`
- **WHEN** код вызывает `ЗначениеЗаполнено(ДоговорКонтрагента)` без префикса `ЭтотОбъект.`
- **THEN** `ДоговорКонтрагента` резолвится как member типа `ДокументОбъект.<Doc>`
- **AND** диагностика `Необъявленная переменная` не генерируется

### Requirement: Applied owner fallback MUST NOT ослаблять FormModule strict semantics
Включение owner-member fallback для applied object modules MUST NOT возвращать dual-layer поведение в `FormModule`.

#### Scenario: FormModule остаётся strict form-data при включенном applied fallback
- **GIVEN** owner-member fallback для applied object modules включен
- **WHEN** пользователь запрашивает members для `FormModule.Объект`
- **THEN** выдача строится по strict form-data semantics
- **AND** members из `ДокументОбъект.*` / `СправочникОбъект.*` не подмешиваются

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

### Requirement: Hover/completion ordering MUST быть детерминированным после merge provider-слоёв
После добавления owner-member fallback и predefined manager members система MUST выдавать properties/methods в `hover` и `completion` в стабильном алфавитном порядке.

#### Scenario: Hover для Объект и ЭтотОбъект стабилен по сортировке
- **GIVEN** тип содержит metadata properties, platform members и predefined members
- **WHEN** пользователь запрашивает hover в одном и том же snapshot
- **THEN** порядок properties/methods детерминирован и алфавитный
- **AND** порядок не зависит от внутреннего порядка обхода provider-цепочки
