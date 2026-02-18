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
- `ЭтотОбъект`, `ЭтаФорма`, `Форма` -> runtime form context descriptor, чья canonical member-semantics MUST формироваться как композиция:
  1. platform base `ФормаКлиентскогоПриложения`,
  2. form extension по типу главного реквизита,
  3. form-local shape (реквизиты/элементы формы);
- `Объект` -> strict form-data semantics (`ДанныеФормыСтруктура`);
- `Элементы` -> дескриптор контейнера элементов формы;
- `Параметры` -> `Структура`.

Система MUST NOT трактовать `ЭтотОбъект/ЭтаФорма/Форма` как synthetic-only source без platform base/extension слоя.

#### Scenario: `ЭтотОбъект` включает platform form context и form-local shape
- **GIVEN** `FormModule` документной формы в контексте, где form symbols доступны
- **WHEN** IDE запрашивает member completion для `ЭтотОбъект.`
- **THEN** выдача содержит members из `ФормаКлиентскогоПриложения`
- **AND** выдача содержит form-local members текущей формы

## MODIFIED Requirements

### Requirement: Member completion для implicit symbols включает свойства и методы (MUST)
Система MUST возвращать в member completion для implicit symbols и свойства, и методы, полученные через descriptor/facet-aware lookup.

Для `FormModule.ЭтотОбъект/ЭтаФорма/Форма` completion MUST применять context-aware фильтрацию доступности members по текущему usage context (compiler directive + module execution context).

Completion MUST NOT предлагать members, недоступные в текущем usage context.

Система MUST классифицировать items детерминированно по kind (`property`/`method`) и выполнять case-insensitive дедупликацию по canonical key.
Canonical key MUST включать semantic owner identity и scope, чтобы кандидаты из разных owner-контекстов не схлопывались в один item без явного правила объединения.

#### Scenario: Completion не предлагает недоступный в контексте member формы
- **GIVEN** процедура `FormModule` выполняется в контексте, где конкретный member `ФормаКлиентскогоПриложения` недоступен
- **WHEN** IDE запрашивает member completion для `ЭтотОбъект.`
- **THEN** недоступный member отсутствует в completion

## ADDED Requirements

### Requirement: Context-aware доступность members form runtime context едина для всех v2 consumer-ов (MUST)
Система MUST применять одинаковую политику доступности members runtime form context (`ЭтотОбъект/ЭтаФорма/Форма`) в:
- `completion`,
- `hover`,
- `diagnostics`,
- `type-at-position`.

Если member существует у form runtime context, но недоступен в текущем usage context, система MUST возвращать контекстную диагностику недоступности при явном обращении к member.

При `Unknown` usage context система MUST использовать консервативную деградацию и MUST NOT генерировать жёсткую ошибку недоступности только из-за неопределённости контекста.

#### Scenario: Явный вызов недоступного member даёт контекстную диагностику
- **GIVEN** в `FormModule` выполнено явное обращение к member, который существует, но недоступен в текущем usage context
- **WHEN** запускается v2 semantic diagnostics
- **THEN** система возвращает диагностику недоступности member в текущем контексте
- **AND** система не возвращает ложную диагностику `NonExistentProperty/NonExistentMethod`

#### Scenario: Unknown context не даёт ложную ошибку недоступности
- **GIVEN** usage context не может быть определён однозначно
- **WHEN** выполняется v2 member validation
- **THEN** система не генерирует ошибку недоступности member только на основании `Unknown` контекста

