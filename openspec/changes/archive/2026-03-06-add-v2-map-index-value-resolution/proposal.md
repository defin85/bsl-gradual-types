# Change: Flow-sensitive index value resolution для `Соответствие` в IntelliSense v2

## Why
Сейчас v2 pipeline слабо типизирует индексный доступ `map[ключ]` для `Соответствие`, поэтому цепочки вида `map["k"].Свойство` и hover/type-at-position после index access часто деградируют в `Unknown`.

Для 1С это снижает ценность IDE: `Соответствие` широко используется как dictionary payload, и пользователю нужен предсказуемый вывод типа значения по ключу и базовому generic `V`.

## What Changes
- Добавить в `bsl-intellisense-v2` требования на flow-sensitive резолюцию значения `Соответствие` при index access.
- Зафиксировать источники map-effect:
  - `Новый Соответствие`,
  - `map.Вставить(Ключ, Значение)` / `map.Установить(Ключ, Значение)`.
- Зафиксировать поведение для `map[ключ]`:
  - если известен тип значения для литерального ключа -> использовать его;
  - иначе использовать generic value type `V`, если известен;
  - иначе безопасно деградировать в `Произвольный`.
- Зафиксировать единое поведение completion/hover/type-at-position для выражений после index access.

## Impact
- Affected specs: `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/src/type_inference_v2.rs` (`Expression::IndexAccess`)
  - `analysis-v2/src/lib.rs` (snapshot hints propagation)
  - `semantic-diagnostics/src/type_hints.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `shared/src/domain/metadata_lookup/*`
  - `bsl-types/src/types/*` (при необходимости map schema overlay)

## Non-Goals
- Доказательство существования конкретного ключа во всех ветках исполнения.
- Full inter-file/inter-module dataflow для соответствий.
- Жёсткая диагностика "ключ не найден" для динамических ключей (избегаем FP).
