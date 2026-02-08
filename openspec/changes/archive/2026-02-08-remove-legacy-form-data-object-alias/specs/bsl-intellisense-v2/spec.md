## ADDED Requirements

### Requirement: Implicit-symbols в v2 MUST резолвиться контекстно по ModuleType
Система MUST определять типы implicit-symbols через единый контекстный резолвер, учитывающий `ModuleType`, владельца метаданных и директивы компиляции.

Для одинакового имени symbol (например, `Объект`) тип MUST зависеть от модульного контекста:
- `FormModule` -> form-data модель;
- `ManagerModule` -> manager facet владельца;
- `ObjectModule`/`RecordSetModule` -> object/recordset facet владельца.

#### Scenario: Один и тот же symbol `Объект` получает разные типы в разных модулях
- **GIVEN** есть `FormModule` и `ManagerModule` одного владельца метаданных
- **WHEN** v2 pipeline строит type hints для идентификатора `Объект`
- **THEN** тип `Объект` в `FormModule` соответствует form-data модели
- **AND** тип `Объект` в `ManagerModule` соответствует manager facet

### Requirement: Для `FormModule.Объект` v2 MUST использовать платформенную form-data модель
Система MUST представлять `Объект` в модуле формы через платформенную семантику form data (`ДанныеФормыСтруктура` и связанные form-data типы), а не через внутренний synthetic alias.

Система MUST поддерживать доступ к гарантированным членам applied object, релевантным для form-data контекста (включая `Ссылка` для документных форм).

#### Scenario: `Объект.Ссылка` в форме документа не даёт ложный `NonExistentProperty`
- **GIVEN** код формы документа обращается к `Объект.Ссылка`
- **WHEN** выполняется v2 semantic diagnostics
- **THEN** система не возвращает диагностику `Свойство 'Ссылка' не существует`

### Requirement: Legacy `ДанныеФормыОбъект.*` MUST быть удалён из user-facing v2 outputs
Система MUST NOT использовать или показывать `ДанныеФормыОбъект.*` в user-facing результатах v2 (`diagnostics`, `hover`, `completion`, `type-at-position`).

#### Scenario: Пользовательская выдача не содержит legacy alias
- **GIVEN** пользователь запрашивает hover и diagnostics для form-object выражений
- **WHEN** v2 pipeline возвращает результаты
- **THEN** в сообщениях и type labels отсутствует `ДанныеФормыОбъект.*`
