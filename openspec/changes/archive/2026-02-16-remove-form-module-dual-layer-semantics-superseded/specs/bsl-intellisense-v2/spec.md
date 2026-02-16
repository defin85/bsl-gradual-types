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

### Requirement: Для `FormModule.Объект` v2 MUST использовать платформенную form-data модель
Система MUST представлять `Объект` в модуле формы через платформенную form-data семантику (`ДанныеФормыСтруктура` и связанные form-data типы), а не через owner object facet.

Система MUST NOT добавлять в `Объект.` members/методы из `ДокументОбъект.*` / `СправочникОбъект.*` автоматически.
Это ограничение MUST сохраняться даже если в других module kinds включён owner-member fallback для bare identifier.

#### Scenario: `Объект.` не подмешивает object-facet методы
- **GIVEN** модуль формы документа
- **WHEN** IDE запрашивает member completion для `Объект.`
- **THEN** выдача не содержит методов, источником которых является object facet applied object
- **AND** выдача ограничена form-data/runtime-формой members

### Requirement: Descriptor-aware member resolution для FormModule.Объект является детерминированным (MUST)
Для `FormModule.Объект` система MUST выполнять member-resolution через form-data-oriented provider chain без facet fallback applied object.

Детерминированная цепочка MUST включать только:
1. members данных формы (shape главного реквизита и связанных данных формы),
2. платформенные members form-data типа.

Система MUST NOT использовать provider-шаг applied object facet fallback для `FormModule.Объект`.

#### Scenario: Детеминированный form-data chain без applied facet fallback
- **GIVEN** `FormModule.Объект` в документной форме
- **WHEN** v2 pipeline строит members для hover/completion
- **THEN** используется только form-data-oriented chain
- **AND** applied object facet fallback не участвует в выдаче

### Requirement: User-facing label policy для FormModule.Объект отделён от canonical semantics (MUST)
Система MUST использовать для `FormModule.Объект` user-facing label, согласованный с form-data семантикой.

Система MUST NOT отображать `FormModule.Объект` как owner object facet label (`ДокументОбъект.X`, `СправочникОбъект.X`) в compact/full/detailed режимах.

Label policy MUST быть одинаковой между `hover`, `diagnostics`, `completion`, `type-at-position`.

#### Scenario: User-facing label `Объект` согласован с form-data семантикой
- **GIVEN** выражение `Объект` в модуле формы документа
- **WHEN** пользователь запрашивает hover и diagnostics
- **THEN** user-facing type label не использует owner object facet представление
- **AND** вывод согласован между всеми v2 consumers
