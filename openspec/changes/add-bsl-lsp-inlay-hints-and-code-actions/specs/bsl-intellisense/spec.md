## ADDED Requirements

### Requirement: LSP поддерживает inlay hints по типам (конфигурируемо)
Система SHALL поддерживать `textDocument/inlayHint` для `.bsl/.os` документов, чтобы IDE могла показывать подсказки типов (type hints) в коде.

Поддержка inlay hints MUST:
- быть конфигурируемой (включаемой/выключаемой),
- быть детерминированной (одинаковый текст → одинаковый результат),
- иметь лимит на размер ответа.

#### Scenario: Inlay hints включены и возвращают осмысленные результаты
- **GIVEN** включены type hints (feature gate) и настроены пороги шумности
- **WHEN** IDE вызывает `textDocument/inlayHint` для диапазона в документе
- **THEN** сервер возвращает hints типа `: <TypeName>` в релевантных местах (минимум: локальные переменные)
- **AND** результат детерминирован и ограничен по размеру

#### Scenario: Inlay hints выключены
- **GIVEN** type hints выключены настройкой
- **WHEN** IDE пытается использовать hints
- **THEN** сервер не заявляет `inlayHintProvider` в capabilities либо возвращает предсказуемый отказ (без ложных ожиданий)

### Requirement: LSP поддерживает code actions (MVP) без заглушек
Система SHALL поддерживать `textDocument/codeAction` для предоставления пользователю quick fixes и/или простых refactors, при этом:
- сервер SHALL не заявлять `codeActionProvider`, если не способен вернуть осмысленные результаты,
- поддерживаемое множество действий MUST быть задокументировано (MVP-границы).

#### Scenario: IDE показывает code actions, когда они применимы
- **GIVEN** code actions включены (feature gate)
- **WHEN** IDE запрашивает `textDocument/codeAction` для диапазона в документе
- **THEN** сервер возвращает применимые code actions (минимум: один refactor и один quick fix в пределах документа)
- **AND** server не возвращает “пустые заглушки” вместо действий

#### Scenario: Code actions выключены
- **GIVEN** code actions выключены
- **WHEN** IDE запрашивает code actions
- **THEN** сервер не заявляет `codeActionProvider` в capabilities либо возвращает предсказуемый отказ

