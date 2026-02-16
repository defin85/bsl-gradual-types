## ADDED Requirements

### Requirement: Completion возвращает context implicit symbols в поддерживаемых module contexts (MUST)
Система MUST включать context implicit symbols в non-member completion для поддерживаемых модульных контекстов в соответствии с `ModuleType` и descriptor-based binding contract.

Для `FormModule` минимально MUST быть доступны:
- `ЭтотОбъект`
- `ЭтаФорма`
- `Форма`
- `Объект`
- `Элементы`
- `Параметры`

Для `ManagerModule`/`ObjectModule`/`RecordSetModule` MUST быть доступны соответствующие implicit symbols из контекстной матрицы.

#### Scenario: `FormModule` non-member completion включает implicit symbols
- **GIVEN** курсор находится в модуле формы в поддерживаемом контексте
- **WHEN** IDE запрашивает non-member completion
- **THEN** выдача включает `ЭтотОбъект`, `Объект`, `Форма`, `ЭтаФорма`, `Элементы`, `Параметры`

### Requirement: Member completion для implicit symbols включает свойства и методы (MUST)
Система MUST возвращать в member completion для implicit symbols и свойства, и методы, полученные через descriptor/facet-aware lookup.

Система MUST классифицировать items детерминированно по kind (`property`/`method`) и выполнять case-insensitive дедупликацию по canonical key.
Canonical key MUST включать semantic owner identity и scope, чтобы кандидаты из разных owner-контекстов не схлопывались в один item без явного правила объединения.

#### Scenario: `ЭтотОбъект.` возвращает свойства и методы object facet
- **GIVEN** код в модуле объекта документа использует `ЭтотОбъект.`
- **WHEN** IDE запрашивает member completion
- **THEN** completion включает свойства object facet (например, `Ссылка`)
- **AND** completion включает методы object facet (например, `Записать`)

### Requirement: `FormModule.Объект` completion использует фиксированный provider chain (MUST)
Для `FormModule.Объект` система MUST формировать members в порядке:
1. form shape members,
2. intrinsic supplement (whitelist),
3. applied object facet members,
4. fallback members (если применимо).

Intrinsic supplement MUST быть additive-only и MUST NOT переопределять/удалять members из facet metadata.
Система MUST разделять collection order и precedence policy:
- collection order определяет порядок формирования/показа;
- precedence policy определяет победителя при конфликте одноимённых members.
При конфликте intrinsic vs repository/facet members MUST выигрывать repository/facet member независимо от order обхода.

#### Scenario: `Объект.` в форме документа объединяет shape, intrinsic и facet members
- **GIVEN** модуль формы документа и курсор на `Объект.`
- **WHEN** IDE запрашивает member completion
- **THEN** completion включает реквизиты формы из form shape
- **AND** completion включает гарантированные intrinsic properties (минимум: `Ссылка`, `ПометкаУдаления`)
- **AND** completion включает методы applied object facet (например, `Записать`)

#### Scenario: Конфликт intrinsic и facet member имени не ломает precedence
- **GIVEN** для `FormModule.Объект` существует одноимённый member в intrinsic и repository/facet источнике
- **WHEN** IDE формирует member completion
- **THEN** в выдаче используется repository/facet версия member
- **AND** intrinsic версия не переопределяет repository/facet metadata

### Requirement: Completion для implicit symbols согласован с v2 type snapshot consumers (MUST)
Система MUST использовать тот же owner resolution результат для completion, hover, type-at-position и semantic member validation в рамках одного snapshot/revision.

#### Scenario: Member, предложенный completion, не даёт ложный `NonExistentProperty`
- **GIVEN** completion предложил member для `Объект.`
- **WHEN** пользователь выбирает member и выполняется semantic diagnostics/hover
- **THEN** diagnostics не возвращает ложный `NonExistentProperty` для этого member
- **AND** hover/type-at-position резолвят owner через тот же descriptor/facet контекст

### Requirement: Context-bound implicit symbols не предлагаются в `*БезКонтекста` (MUST)
Система MUST NOT предлагать context-bound implicit symbols в non-member completion внутри процедур/функций `*БезКонтекста`.

#### Scenario: `&НаСервереБезКонтекста` не предлагает `ЭтотОбъект`
- **GIVEN** курсор находится внутри процедуры `&НаСервереБезКонтекста`
- **WHEN** IDE запрашивает non-member completion
- **THEN** completion не содержит context-bound symbols, такие как `ЭтотОбъект` и `Объект`

### Requirement: Completion output остаётся bounded и детерминированным в интерактивном режиме (MUST)
Система MUST ограничивать количество возвращаемых completion items фиксированным limit.
Если после ranking/dedup кандидатов больше лимита, система MUST выставлять `isIncomplete = true`.
При одинаковом snapshot/revision порядок выдачи MUST быть детерминированным.

#### Scenario: Количество кандидатов превышает limit
- **GIVEN** completion контекст, где количество кандидатов превышает системный limit
- **WHEN** IDE запрашивает completion
- **THEN** сервер возвращает не более limit items
- **AND** `isIncomplete` установлен в `true`
