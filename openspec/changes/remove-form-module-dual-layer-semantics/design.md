## Context
В текущем контракте `FormModule.Объект` моделируется как form-data descriptor с подмешиванием applied object facet members.
Это даёт полезные подсказки, но расходится с реальным runtime-контекстом 1С, где:
- `ЭтотОбъект` — это форма (`ФормаКлиентскогоПриложения` + extension по главному реквизиту);
- `Объект` — это данные формы (`ДанныеФормыСтруктура`), а не прямой `DocumentObject/CatalogObject`.

Требуется намеренный breaking переход без safe migration.

## Goals / Non-Goals
- Goals:
  - Привести implicit-symbol semantics в модуле формы к runtime-модели 1С.
  - Убрать архитектурное смешение form-data и object facet для `Объект`.
  - Синхронизировать поведение hover/completion/type-at-position/diagnostics.
- Non-Goals:
  - Сохранение dual-layer UX совместимости.
  - Feature flags, canary rollout, staged migration.
  - Попытка удержать старые member-наборы для `Объект.`.

## Decisions

### 1) `FormModule.Объект` — strict form-data
`Объект` в модуле формы интерпретируется как `ДанныеФормыСтруктура` (и связанные form-data типы).

В member-resolution:
- допускаются members, происходящие из form-data shape/структуры данных формы;
- допускаются платформенные members form-data типа;
- не допускается автоматическое подмешивание members из applied object facet (`ДокументОбъект.*`, `СправочникОбъект.*`) как fallback.

### 2) `FormModule.ЭтотОбъект/ЭтаФорма/Форма` — контекст формы
`ЭтотОбъект`, `ЭтаФорма`, `Форма` в модуле формы резолвятся как контекст формы:
- базовый `ФормаКлиентскогоПриложения`;
- extension в зависимости от типа главного реквизита формы (например, документная форма);
- реквизиты формы.

### 3) Единая модель для всех consumers
Новая семантика обязательна и едина для:
- `completion`,
- `hover`,
- `type-at-position`,
- `diagnostics`.

Расхождение owner-resolution между каналами недопустимо.

### 4) Breaking policy: no safe migration
Из архитектуры исключаются:
- compatibility feature flags,
- dual-path rollout,
- fallback на legacy dual-layer поведение.

Любые регрессии по ожиданиям старого поведения считаются ожидаемым результатом breaking change.

### 5) Scope boundary with applied-module self-scope change
Этот документ описывает только `FormModule` semantics.
Он MUST NOT:
- расширять/изменять fallback bare identifier для `ObjectModule`/`RecordSetModule`/`ManagerModule`;
- описывать manager predefined members.

Эти аспекты находятся в `update-applied-module-self-scope-and-predefined-members` и должны интегрироваться без ослабления strict form-data контракта.

## Implementation Considerations
- Требуется пересмотр `FormDataObject` резолва в `analysis-v2` и `completion_service`, чтобы type label и member lookup не эмулировали object facet.
- Требуется пересборка contract tests: текущие dual-layer тесты должны быть заменены runtime-совместимыми.
- Источником проверочных сценариев выступают реальные дампы отладчика (`Объект.txt`, `ЭтотОбъект.txt`) для документной формы.
- Нужны кросс-regression тесты на совместное внедрение с `update-applied-module-self-scope-and-predefined-members`: fallback applied modules не должен влиять на `FormModule.Объект`.

## Risks / Trade-offs
- Непосредственная потеря части привычных подсказок на `Объект.` (из object facet).
- Рост числа пользовательских вопросов после релиза из-за изменения состава completion.
- Более строгая модель может выявить текущие зависимости тестов/кода от старого не-runtime поведения.

## Rollout / Migration
Не предусмотрены.

Change внедряется как breaking cutover в одном контракте без safe migration режима.
