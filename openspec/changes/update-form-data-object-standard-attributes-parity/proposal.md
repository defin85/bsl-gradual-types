# Change: update-form-data-object-standard-attributes-parity

## Why
После устранения смешивания контекстов формы и applied-object в `FormModule.Объект` сохранился gap по parity с платформой 1С: `hover(Объект)` показывает только часть applied-object свойств и не включает полный ожидаемый набор стандартных реквизитов документа (например, `Дата`, `Номер`, `Проведен`) в проекции `ДанныеФормыСтруктура`.

Это ухудшает предсказуемость IntelliSense: разработчик видит более узкий контракт, чем в реальном runtime/отладчике.

## What Changes
- Уточнить контракт для `FormModule.Объект`:
  - тип остаётся `ДанныеФормыСтруктура`;
  - формируется applied-object проекция с документными реквизитами, табличными частями и стандартными реквизитами объекта;
  - form-only реквизиты (`Form.xml` attributes формы, не являющиеся реквизитами applied-object) не попадают в `Объект`.
- Зафиксировать regression-тестами parity для конкретного реального документа `РеализацияТоваровУслуг`.
- Зафиксировать, что `ЭтотОбъект` продолжает отражать контекст формы и содержит свойство `Объект: ДанныеФормыСтруктура`.

## Impact
- Affected specs:
  - `bsl-intellisense-ide-grade`
- Affected code (expected):
  - `shared/src/domain/metadata_lookup/*`
  - `analysis-v2/src/type_inference_v2.rs`
  - `bsl-runtime/src/data/loaders/config_metadata_parser/*`
  - `backend/tests/*form*` и `backend/tests/*conf_big*`
