## ADDED Requirements

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

#### Scenario: IDE показывает outline BSL файла
- **GIVEN** открыт `.bsl` файл
- **WHEN** IDE запрашивает `textDocument/documentSymbol`
- **THEN** сервер возвращает структуру символов с корректными диапазонами

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
