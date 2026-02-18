## ADDED Requirements
### Requirement: `FormModule.Объект` SHALL отражать applied-object проекцию с standard attributes
Система SHALL обеспечивать, что в модуле формы неявная переменная `Объект` имеет тип `ДанныеФормыСтруктура` и содержит:
- реквизиты applied-object владельца формы,
- табличные части applied-object,
- стандартные реквизиты applied-object (включая как минимум `Дата`, `Номер`, `Проведен` для документов),
- без включения form-only реквизитов формы.

Система SHALL строить этот набор через metadata pipeline (`parser -> converter -> repository/lookup`) как source of truth.
Система SHALL NOT полагаться только на hardcoded intrinsic supplement для достижения parity standard attributes.

#### Scenario: Hover по `Объект` в модуле формы документа
- **GIVEN** модуль `Documents/<Doc>/Forms/<Form>/Ext/Form/Module.bsl`
- **AND** форма имеет main attribute `Объект`
- **WHEN** IDE запрашивает hover по идентификатору `Объект`
- **THEN** отображается тип `ДанныеФормыСтруктура`
- **AND** список свойств включает applied-object реквизиты и standard attributes документа (`Дата`, `Номер`, `Проведен`)
- **AND** список свойств не включает form-only реквизиты (`Надпись*`, `ПоказыватьБаннер`, `СсылкаДляПереходаНаКарту` и аналогичные атрибуты формы)

#### Scenario: Form-context остаётся на `ЭтотОбъект`
- **GIVEN** тот же модуль формы
- **WHEN** IDE запрашивает hover по `ЭтотОбъект`
- **THEN** отображается тип `Формы.<...>`
- **AND** у `ЭтотОбъект` присутствует свойство `Объект: ДанныеФормыСтруктура`
- **AND** form-only реквизиты доступны в контексте `ЭтотОбъект`/формы

#### Scenario: Standard attributes берутся из metadata source, а не из form-shape
- **GIVEN** applied-object документа в metadata содержит standard attributes `Date`, `Number`, `Posted`
- **AND** `Form.xml` содержит form-only attributes, отсутствующие в applied-object metadata
- **WHEN** IDE формирует members для `FormModule.Объект`
- **THEN** `Дата`, `Номер`, `Проведен` присутствуют в выдаче
- **AND** form-only attributes отсутствуют в выдаче `Объект`
