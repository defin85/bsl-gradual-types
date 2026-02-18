## Context

Текущий контракт v2 фиксирует descriptor-модель implicit symbols, но не задаёт строгий source-of-truth для members `ЭтотОбъект/ЭтаФорма/Форма` как композиции платформенного form runtime context.

Практический эффект:
- часть платформенных members `ФормаКлиентскогоПриложения` не попадает в выдачу;
- доступность members по контексту вызова (директивы/фасет использования) не унифицирована для completion/hover/diagnostics/type-at-position.

## Goals / Non-Goals

- Goals:
  - Нормативно закрепить runtime form context для `ЭтотОбъект/ЭтаФорма/Форма`.
  - Ввести единое правило контекстной доступности members для v2 consumer-ов.
  - Сохранить строгую изоляцию `FormModule.Объект` как form-data канала.
- Non-Goals:
  - Пересмотр семантики `FormModule.Объект`.
  - Feature-flag/dual-mode поведение.
  - Расширение на новые модульные типы вне текущей матрицы.

## Decisions

### Decision 1: Source-of-truth для `ЭтотОбъект/ЭтаФорма/Форма`

`ЭтотОбъект/ЭтаФорма/Форма` MUST резолвиться как runtime form context из трёх слоёв:
1. `ФормаКлиентскогоПриложения` (platform base),
2. form extension по типу главного реквизита,
3. shape формы (локальные реквизиты/элементы).

Слой shape остаётся формозависимой надстройкой; platform/extension слой обязателен.

### Decision 2: Context-aware доступность members

Members platform/extension/shape MUST фильтроваться по `UsageContext` (минимум: compiler directive + модульный execution context).

Политика:
- Completion: только доступные members.
- Diagnostics: при явном обращении к недоступному member — контекстная ошибка доступности.
- Hover/type-at-position: используют ту же модель доступности для консистентности.

### Decision 3: Unknown-context fallback

При неопределённом контексте система не должна генерировать жёсткие false-positive ошибки недоступности.
Разрешается консервативная деградация (не блокировать member только из-за `Unknown`).

### Decision 4: Границы change

`FormModule.Объект` остаётся strict form-data contract и не получает members из `ФормаКлиентскогоПриложения` через этот change.

## Implementation Considerations

1. Дополнить модель метаданных properties контекстом доступности, чтобы методы и свойства фильтровались одинаково.
2. Расширить lookup API до context-aware режима и сделать его единым источником для всех v2 consumer-ов.
3. Обновить completion/hover/diagnostics/type-at-position на новый API без локальных divergent-фильтров.
4. Закрыть матрицу регрессий тестами:
   - проброс platform form members в `ЭтотОбъект`,
   - контекстная фильтрация client/server/*БезКонтекста*,
   - отсутствие протечки в `FormModule.Объект`,
   - кросс-consumer parity.

## Risks / Trade-offs

- Риск ложных отрицаний при агрессивной фильтрации в ambiguous-контексте.
  - Митигация: explicit Unknown fallback policy.
- Риск расхождения правил между methods и properties.
  - Митигация: единая модель availability для обоих типов members.
- Риск роста latency completion из-за дополнительных слоёв merge/filter.
  - Митигация: bounded merge и кэш по `(owner, usage_context)`.

## Open Questions

- Нужен ли отдельный user-facing badge для members, исключённых по контексту, в detailed hover, или достаточно строгой фильтрации?
- Нужна ли отдельная диагностика для недоступного свойства (не только метода) или достаточно общего member-уровня?

