# Change: update-form-data-object-standard-attributes-parity

## Why
После устранения смешивания контекстов формы и applied-object в `FormModule.Объект` сохранился gap по parity с платформой 1С: `hover(Объект)` показывает только часть applied-object свойств и не включает полный ожидаемый набор стандартных реквизитов документа (например, `Дата`, `Номер`, `Проведен`) в проекции `ДанныеФормыСтруктура`.

Это ухудшает предсказуемость IntelliSense: разработчик видит более узкий контракт, чем в реальном runtime/отладчике.

## What Changes
- Зафиксировать архитектурное решение **вариант 2**: источник истины для parity строится через metadata pipeline (`parser -> converter -> lookup`), без hardcoded подмешивания members в intrinsic-слой.
- Уточнить контракт для `FormModule.Объект`:
  - тип остаётся `ДанныеФормыСтруктура`;
  - формируется applied-object проекция с реквизитами объекта, табличными частями и standard attributes;
  - form-only реквизиты (`Form.xml` attributes формы, не являющиеся реквизитами applied-object) не попадают в `Объект`.
- Добавить parser/converter поддержку standard attributes applied-object (минимум для документов: `Date`, `Number`, `Posted`; также `Ref`, `DeletionMark` как часть стандартного набора платформы).
- Обновить form-data aggregation в `TypeMetadataLookup`, чтобы:
  - использовать расширенный metadata source из repository,
  - включать проекцию табличных частей applied-object в form-data members,
  - сохранять strict запрет на form-shape leakage.
- Усилить regression-тесты parity на реальном документе `РеализацияТоваровУслуг`:
  - required/minimum positive-set (`Дата`, `Номер`, `Проведен`),
  - negative-set form-only members (`Надпись*`, `ПоказыватьБаннер`, `СсылкаДляПереходаНаКарту`),
  - инвариант `ЭтотОбъект` как form-context c `Объект: ДанныеФормыСтруктура`.

## Impact
- Affected specs:
  - `bsl-intellisense-ide-grade`
- Affected code (expected):
  - `bsl-runtime/src/data/loaders/config_metadata_parser/parser.rs`
  - `bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs`
  - `shared/src/domain/metadata_lookup/core.rs`
  - `shared/src/domain/metadata_lookup/tests.rs`
  - `analysis-v2/src/type_inference_v2.rs` (verification only, без смены semantic contract)
  - `backend/tests/*form*` и `backend/tests/*conf_big*`
