## 1. Analysis & Contract
- [ ] 1.1 Сформировать эталонный набор свойств `Объект` для `Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая` на основе `Document.xml` + standard attributes + табличных частей.
- [ ] 1.2 Зафиксировать явный negative-set form-only реквизитов, которые не должны присутствовать в `FormModule.Объект`.

## 2. Implementation
- [ ] 2.1 Скорректировать построение/агрегацию свойств `FormDataObject` так, чтобы включались standard attributes applied-object.
- [ ] 2.2 Гарантировать, что leakage form-only реквизитов в `Объект` не возвращается.
- [ ] 2.3 Проверить, что `ЭтотОбъект` сохраняет form-context и `Объект: ДанныеФормыСтруктура`.

## 3. Validation
- [ ] 3.1 Добавить/обновить unit-тесты для provider-chain/aggregation контракта.
- [ ] 3.2 Добавить/обновить integration-тест для `conf_big/Documents/РеализацияТоваровУслуг` с проверкой required/minimum parity (`Дата`, `Номер`, `Проведен`) и absence form-only leakage.
- [ ] 3.3 Прогнать targeted test suite и зафиксировать результаты.
