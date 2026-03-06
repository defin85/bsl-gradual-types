# Change: Flow-sensitive field schema resolution для `Структура` в IntelliSense v2

## Why
Сейчас v2 pipeline не фиксирует schema-effect полей конкретного экземпляра `Структура` (например, после `Вставить("Поле", ...)`), из-за чего обращения `s.Поле` в completion/hover/diagnostics часто теряют точность и приводят к шумной деградации в `Unknown`.

Для 1С это критично: `Структура` активно используется как typed payload с фиксированным набором полей, и IDE должна стабильно показывать поля и их типы.

## What Changes
- Добавить в `bsl-intellisense-v2` контракт snapshot-local schema для экземпляров `Структура`.
- Зафиксировать `schema-effect` для операций, формирующих поле:
  - `Новый Структура(...)` (поддерживаемые статические паттерны),
  - `Структура.Вставить("ИмяПоля", Значение)`.
- Зафиксировать единое поведение completion/hover/type-at-position/semantic diagnostics для `s.<Поле>`.
- Зафиксировать strict-политику для typed-structure:
  - известные поля резолвятся как свойства;
  - неизвестные поля диагностируются как несуществующее свойство.
- Зафиксировать safe fallback по типу поля:
  - если тип значения вычислен статически -> использовать его;
  - если не вычислен -> `Произвольный`, но поле остаётся доступным.

## Impact
- Affected specs: `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/lib.rs` (type hints / snapshot propagation)
  - `shared/src/domain/metadata_lookup/*` (property lookup для typed structure)
  - `semantic-diagnostics/src/type_hints.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-types/src/types/*` (при необходимости отдельной модели structure schema)

## Non-Goals
- Полная интерпроцедурная резолюция всех структур между произвольными файлами/модулями.
- Точная модель динамических имён полей (вычисляемые строки, `Выполнить`, рефлексия).
- Ретрофит legacy inference path; change ограничен v2 pipeline.
