## 1. Contract Baseline
- [x] 1.1 Зафиксировать positive-set `FormModule.Объект` для `Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая`: applied-object attributes + standard attributes + табличные части.
- [x] 1.2 Зафиксировать negative-set form-only members (например, `Надпись*`, `ПоказыватьБаннер`, `СсылкаДляПереходаНаКарту`) для обязательной проверки отсутствия в `Объект`.
- [x] 1.3 Зафиксировать policy типизации standard attributes (в т.ч. `Number` по `NumberType`, `Posted` только для posting-capable документа).

## 2. Parser & Converter (Architecture Option 2)
- [x] 2.1 Расширить parser metadata-модель для извлечения standard attributes applied-object из `Document.xml` (и совместимых объектов).
- [x] 2.2 Обновить converter pipeline так, чтобы standard attributes попадали в `RawTypeData` как часть applied-object metadata source of truth.
- [x] 2.3 Добавить dedup/precedence policy для конфликтов имён (repository source wins, без leakage form-shape).

## 3. FormData Aggregation
- [x] 3.1 Обновить `TypeMetadataLookup` form-data provider chain для использования расширенного repository source (вариант 2, без hardcoded-only intrinsic решения).
- [x] 3.2 Добавить projection табличных частей applied-object в members `FormModule.Объект`.
- [x] 3.3 Проверить инварианты: strict `ДанныеФормыСтруктура`, запрет на form-only leakage, сохранение контекста `ЭтотОбъект`.

## 4. Tests & Quality Gates
- [x] 4.1 Добавить/обновить unit-тесты parser/converter для standard attributes (`Date`, `Number`, `Posted`, `Ref`, `DeletionMark`) и policy типизации.
- [x] 4.2 Добавить/обновить unit-тесты `TypeMetadataLookup` на provider-chain + tabular projection + no form-shape leakage.
- [x] 4.3 Добавить/обновить integration-тест `conf_big/Documents/РеализацияТоваровУслуг` с required/minimum parity (`Дата`, `Номер`, `Проведен`) и negative-set проверками.
- [x] 4.4 Устранить ложнозелёные проверки: критичные parity-asserts не должны silently pass при неполном окружении.

## 5. Validation
- [x] 5.1 Прогнать targeted test suite и зафиксировать результаты.
- [x] 5.2 `openspec validate update-form-data-object-standard-attributes-parity --strict --no-interactive`.
