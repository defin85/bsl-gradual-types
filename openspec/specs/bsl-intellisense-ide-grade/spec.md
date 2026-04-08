# bsl-intellisense-ide-grade Specification

## Purpose
TBD - created by archiving change define-bsl-intellisense-ide-grade. Update Purpose after archive.
## Requirements
### Requirement: Core IntelliSense (LSP) как базовая линия (MUST)
Система SHALL поддерживать и не регрессировать базовые функции IntelliSense, зафиксированные в capability `bsl-intellisense`:
- `textDocument/completion` и `completionItem/resolve`,
- `textDocument/hover`,
- `textDocument/signatureHelp`,
- `textDocument/definition` (как минимум для навигации по определениям типов),
- публикацию диагностик через `textDocument/publishDiagnostics`.

#### Scenario: IDE использует базовые функции IntelliSense
- **GIVEN** LSP‑сервер запущен и рабочая область содержит `.bsl` файл
- **WHEN** IDE запрашивает completion/hover/signatureHelp/definition и получает diagnostics при изменении текста
- **THEN** сервер возвращает корректные ответы по протоколу LSP и публикует diagnostics для текущей версии документа

### Requirement: IDE‑grade completion по выражениям + stdlib + metadata (MUST)
Система SHALL обеспечивать IDE‑grade автодополнение для BSL, ориентированное на реальные 1С‑кодовые базы:
- completion для цепочек выражений и неполного кода (незакрытые скобки/строки, `expr.` без идентификатора),
- типизация receiver‑выражений для completion: вызовы, индексаторы, скобки, `?()` и `Выбор...Конец`,
- интеграция stdlib + metadata как first‑class сценарий:
  - `Документы.`/`Справочники.`/`РегистрыСведений.`/...,
  - фасеты (Manager/Object/Reference/Selection) и переходы,
  - табличные части/реквизиты,
  - методы/свойства платформенных типов по синтаксис‑докам.

#### Scenario: Completion работает на неполном коде и выражениях
- **GIVEN** пользователь редактирует BSL и код может быть синтаксически неполным
- **WHEN** IDE запрашивает completion в позиции `expr.`
- **THEN** сервер возвращает релевантные members для выведенного receiver‑типа (stdlib + metadata), без зависаний и с предсказуемым fallback

### Requirement: Детерминизм, инкрементальность и отсутствие блокирующего I/O (MUST)
Система SHALL обеспечивать нефункциональные свойства IntelliSense:
- одинаковый текст+контекст → одинаковая выдача (порядок/`sortText`/идентификаторы кандидатов),
- корректность при инкрементальном редактировании (`didChange` не приводит к mixed state),
- поддержка отмены запросов (cancelability),
- отсутствие блокирующих операций I/O в hot path completion/resolve/hover/signatureHelp/diagnostics.

#### Scenario: Повторный запрос completion даёт стабильную выдачу
- **GIVEN** текст документа и зависимости не менялись
- **WHEN** IDE дважды вызывает completion в одной и той же позиции
- **THEN** результаты совпадают (состав и порядок) и имеют стабильные идентификаторы кандидатов

### Requirement: Symbols (document/workspace) для навигации (MUST)
Система SHALL поддерживать:
- `textDocument/documentSymbol` (outline/structure),
- `workspace/symbol` (поиск символов по рабочей области).

Для `textDocument/documentSymbol` сервер MUST рассматривать outline refresh как auxiliary navigation surface, а не как interactive semantic gate.

`textDocument/documentSymbol` для одного файла MAY завершаться одним из bounded outcome-классов:
- `current_ready` — структура requested revision ready и возвращается сразу;
- `latest_ready` — requested revision ещё не готова, но возвращается наиболее свежая готовая структура того же файла;
- `unavailable` — в bounded auxiliary policy нет ни current-ready, ни latest-ready структуры.

Если сервер выбирает `latest_ready` или `unavailable`, это MUST NOT задерживать первый пользовательский interactive semantic ответ (`completion`, `hover`, `signatureHelp`, `definition`) для того же файла.

Auxiliary outline maintenance, включая latest-ready cache materialization и same-version refresh после `didOpen` / `didChange` / `didSave`, MUST выполнять CPU-heavy parse/symbol derivation через bounded auxiliary execution boundary и MUST NOT монополизировать async LSP runtime, который обслуживает transport ingress/egress и interactive request polling.

Под same-file mixed load (`didChange`, `didSave`, burst `textDocument/documentSymbol` и interactive semantic request`) outline maintenance MUST NOT быть причиной seconds-scale ingress или handoff wait для interactive semantic ответа.

#### Scenario: IDE показывает current-ready outline
- **GIVEN** открыт `.bsl` файл
- **AND** структура requested revision уже готова
- **WHEN** IDE запрашивает `textDocument/documentSymbol`
- **THEN** сервер возвращает структуру символов requested revision с корректными диапазонами

#### Scenario: Outline временно отстаёт, но completion остаётся интерактивным
- **GIVEN** пользователь только что изменил и сохранил `.bsl` файл
- **AND** requested revision symbol tree ещё не materialized
- **WHEN** IDE почти одновременно обновляет Outline и запрашивает completion
- **THEN** `documentSymbol` возвращает `latest_ready` или `unavailable` в рамках bounded auxiliary policy
- **AND** completion не ждёт завершения outline refresh как prerequisite

#### Scenario: Auxiliary outline refresh не уводит transport/runtime loop в starvation
- **GIVEN** same-file parse gap после `didChange` и `didSave`
- **AND** сервер почти одновременно обслуживает burst `textDocument/documentSymbol` и interactive semantic request
- **WHEN** background outline maintenance materializes `latest_ready` cache или same-version refresh
- **THEN** CPU-heavy outline work не выполняется inline на async LSP transport/runtime loop
- **AND** interactive semantic request не копит seconds-scale wait только из-за auxiliary outline maintenance

### Requirement: Find References и Rename для поддерживаемых символов (MUST)
Система SHALL поддерживать:
- `textDocument/references`,
- `textDocument/rename` (и SHOULD `prepareRename`),
минимум для **поддерживаемого множества символов**, которое MUST быть документировано (и стабильно обрабатываться сервером).

#### Scenario: Rename безопасно меняет все ссылки
- **GIVEN** символ поддерживаемого класса (например, локальная переменная в пределах процедуры) и несколько его использований
- **WHEN** IDE вызывает `textDocument/rename`
- **THEN** сервер возвращает `WorkspaceEdit`, который меняет только ссылки на этот символ

### Requirement: Formatting в IDE (SHOULD)
Система SHALL поддерживать форматирование BSL в IDE (например, `textDocument/formatting` и/или `textDocument/rangeFormatting`) при наличии согласованной стратегии форматтера. Поддержка форматирования SHALL быть конфигурируемой (включаемой/выключаемой).

#### Scenario: Пользователь форматирует документ
- **GIVEN** форматирование включено и стратегия форматтера определена
- **WHEN** IDE запрашивает форматирование документа
- **THEN** сервер возвращает детерминированный набор правок с минимальным diff

### Requirement: Code Actions / Quick Fixes (SHOULD)
Система SHALL предоставлять code actions/quick fixes для типичных диагностик и простых рефакторингов, либо SHALL не заявлять поддержку code actions в IDE. Система SHALL не регистрировать “пустые” заглушки по умолчанию.

#### Scenario: IDE предлагает quick fix для диагностики
- **GIVEN** в документе есть диагностика с известным исправлением
- **WHEN** IDE запрашивает code actions для диапазона
- **THEN** пользователь видит применимые quick fixes/рефакторинги

### Requirement: Inlay hints (type hints) (SHOULD)
Система SHALL предоставлять inlay hints по типам (например, типы переменных/возвратов) в IDE, с возможностью настройки порога уверенности/шумности и отключения.

#### Scenario: IDE показывает type hints без лишнего шума
- **GIVEN** включены type hints и настроены пороги
- **WHEN** IDE запрашивает inlay hints
- **THEN** hints отображаются только там, где они полезны и не перегружают код

### Requirement: Formatting не спамит `-32600` и корректно гейтится
Система SHALL обеспечивать, что при выключенном форматировании:
- сервер не заявляет `documentFormattingProvider`/`documentRangeFormattingProvider` в capabilities,
- IDE не получает ошибок `Invalid request (-32600)` при `textDocument/formatting` на save.

#### Scenario: Format on Save включён глобально, но форматирование выключено в BSL
- **GIVEN** в IDE включён `editor.formatOnSave`
- **AND** форматирование BSL выключено настройкой сервера/расширения
- **WHEN** IDE отправляет `textDocument/formatting`
- **THEN** запрос завершается предсказуемо (например, `null`/пустые правки или “not supported”), без `-32600` и без спама ошибок

### Requirement: Локальные функции/процедуры доступны для вызова до объявления
Система SHALL не генерировать диагностику `UndeclaredVariable` для идентификатора,
который является локальной функцией/процедурой текущего модуля, даже если вызов расположен выше объявления.

#### Scenario: Вызов функции до объявления не считается необъявленной переменной
- **GIVEN** в модуле есть вызов `F()` выше объявления `Функция F()`
- **WHEN** сервер публикует diagnostics
- **THEN** диагностика “Необъявленная переменная 'F'” не создаётся

### Requirement: Go to Definition работает для конфигурационных common modules
Система SHALL поддерживать `textDocument/definition` для навигации по конфигурации:
- `CommonModules.<Name>` как namespace модуля,
- `CommonModules.<Name>.<ExportProc>` как переход к объявлению процедуры/функции в модуле.

#### Scenario: Переход к экспортной процедуре общего модуля
- **GIVEN** в форме есть вызов `ИмяОбщегоМодуля.ИмяПроцедуры(...)`
- **AND** в конфигурации существует общий модуль с таким именем и экспортная процедура/функция
- **WHEN** IDE делает `textDocument/definition` по `ИмяПроцедуры`
- **THEN** сервер возвращает `Location` объявления в файле общего модуля

### Requirement: Реквизиты формы и `Элементы` формы типизируются по `Form.xml`
Система SHALL обеспечивать, что в модуле формы:
- реквизиты формы (`<Attribute name="...">`) доступны как идентификаторы с корректным типом,
- `Элементы.<ИмяЭлемента>` резолвится как свойство контейнер‑типа элементов формы, если элемент существует в `Form.xml`.

#### Scenario: Реквизит формы имеет тип и доступен в модуле формы
- **GIVEN** в `Form.xml` формы есть `<Attribute name="СчетФактура">` с типом `cfg:DocumentRef.*`
- **WHEN** пользователь делает hover по `СчетФактура` в модуле формы
- **THEN** сервер показывает тип реквизита (не “unknown/undeclared”)

#### Scenario: `Элементы` содержит реальные элементы формы (в т.ч. группы)
- **GIVEN** в `Form.xml` есть `<UsualGroup name="СчетФактураПросмотр">`
- **WHEN** в модуле формы встречается `Элементы.СчетФактураПросмотр`
- **THEN** сервер не выдаёт диагностику “property not exists” для этого свойства

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

