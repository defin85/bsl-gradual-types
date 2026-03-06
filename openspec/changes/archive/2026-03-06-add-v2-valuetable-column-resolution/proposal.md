# Change: ValueTable column schema resolution в IntelliSense v2

## Why
Сейчас v2 pipeline не отслеживает динамически добавленные колонки `ТаблицаЗначений` (`ТЗ.Колонки.Добавить(...)`) как структурную схему конкретного экземпляра таблицы. Из-за этого LSP не может стабильно резолвить обращения к колонкам строк таблицы, completion теряет доменные поля, а diagnostics дают ложные/шумные результаты.

Это критично для типовых паттернов 1С, где таблица создается в функции, колонки объявляются через `ОписаниеТипов`, после чего таблица возвращается и используется как typed dataset.

## What Changes
- Добавить в `bsl-intellisense-v2` требования на flow-sensitive отслеживание схемы колонок конкретного экземпляра `ТаблицаЗначений`.
- Зафиксировать поддержку паттерна `ТЗ.Колонки.Добавить("ИмяКолонки", ОписаниеТипов...)` как источника schema-effect.
- Зафиксировать, что строка таблицы (`ТЗ.Добавить()` / `Для каждого Стр Из ТЗ`) должна получать тип с колонками, доступными через member access.
- Зафиксировать единое поведение для completion/hover/semantic diagnostics:
  - известные колонки резолвятся и не дают FP-ошибок;
  - неизвестные колонки для typed-row дают hard-fail диагностику.
- Зафиксировать baseline-политику типов колонок:
  - если тип колонки извлечен из `ОписаниеТипов` -> использовать его;
  - если извлечь тип нельзя -> `Произвольный` (без потери имени колонки).

## Impact
- Affected specs: `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/lib.rs` (type hints propagation)
  - `semantic-diagnostics/src/type_hints.rs`
  - `semantic-diagnostics/src/visitor.rs` (typed-row property validation behavior)
  - `shared/src/domain/metadata_lookup/*` (property lookup для typed value-table rows)
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-types/src/types/*` (если потребуется отдельная доменная модель row-schema)

## Non-Goals
- Полная интерпроцедурная резолюция любых таблиц значений между произвольными модулями/файлами.
- Эмуляция runtime-побочных эффектов, не выраженных явно в AST (например, `Выполнить`, рефлексия, динамические имена колонок из непостоянных выражений).
- Ретрофит legacy inference путей; change ограничен v2 pipeline.
