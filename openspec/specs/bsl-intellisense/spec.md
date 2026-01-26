# bsl-intellisense Specification

## Purpose
TBD - created by archiving change audit-bsl-intellisense. Update Purpose after archive.
## Requirements
### Requirement: LSP‑сервер предоставляет базовые функции IntelliSense для BSL
Система SHALL предоставлять LSP‑сервер для BSL, который поддерживает базовые функции IntelliSense:
- `textDocument/completion` и `completionItem/resolve`,
- `textDocument/hover`,
- `textDocument/signatureHelp`,
- `textDocument/definition` (как минимум для навигации по определениям типов),
- публикацию диагностик через `textDocument/publishDiagnostics`.

#### Scenario: IDE получает подсказки и диагностику через LSP
- **GIVEN** LSP‑сервер запущен и рабочая область содержит `.bsl` файл
- **WHEN** клиент запрашивает completion/hover/signatureHelp/definition и получает diagnostics при изменении текста
- **THEN** сервер возвращает корректные ответы по протоколу LSP и публикует diagnostics для текущей версии документа

### Requirement: VS Code extension запускает LSP и предоставляет IDE‑интеграцию
Система SHALL предоставлять VS Code extension, который запускает LSP‑сервер, прокидывает настройки (пути к документации платформы/конфигурации) и обеспечивает базовую IDE‑интеграцию для BSL.

#### Scenario: Расширение VS Code поднимает LSP и включает базовые подсказки
- **GIVEN** пользователь открыл workspace с `.bsl` файлами в VS Code
- **WHEN** расширение активируется
- **THEN** LSP‑клиент стартует, и пользователь получает completion/hover/signatureHelp/diagnostics в редакторе

### Requirement: Стратегия форматирования BSL документирована до реализации
Система SHALL документировать выбранную стратегию форматирования BSL (цели, non-goals, ограничения и способ интеграции в IDE) до добавления LSP formatting.

#### Scenario: Команда согласовала форматирование до включения в IDE
- **GIVEN** проект планирует добавить форматирование в IDE
- **WHEN** change по форматированию проходит ревью
- **THEN** стратегия форматирования (и причины выбора) зафиксированы и понятны поддерживающим

### Requirement: LSP поддерживает форматирование BSL в IDE (SHOULD)
Система SHALL поддерживать форматирование BSL в IDE через LSP:
- `textDocument/formatting`,
- (опционально) `textDocument/rangeFormatting`,
при условии, что стратегия форматтера выбрана и документирована.

Поддержка форматирования SHALL быть конфигурируемой (включаемой/выключаемой).

#### Scenario: Пользователь форматирует документ
- **GIVEN** форматирование включено и стратегия форматтера определена
- **WHEN** IDE запрашивает `textDocument/formatting` для `.bsl` документа
- **THEN** сервер возвращает детерминированный набор правок с минимальным diff

#### Scenario: Форматирование можно отключить
- **GIVEN** форматирование отключено настройкой
- **WHEN** IDE запрашивает `textDocument/formatting`
- **THEN** сервер не заявляет поддержку formatting в capabilities либо возвращает предсказуемый отказ (и не создаёт ложных ожиданий)

### Requirement: LSP поддерживает структуру документа через documentSymbol
Система SHALL:
- объявлять `document_symbol_provider` в `ServerCapabilities`,
- реализовать `textDocument/documentSymbol` для `.bsl` документов.

Минимальная модель символов SHOULD включать:
- процедуры/функции,
- группировку по областям (`#Область`/`#КонецОбласти`) как вложенность.

Сервер MUST возвращать корректные ranges в терминах LSP (UTF-16 позиции) и детерминированный результат для одинакового текста.

#### Scenario: IDE строит outline по documentSymbol
- **GIVEN** открыт `.bsl` файл с процедурами/функциями и областями
- **WHEN** клиент вызывает `textDocument/documentSymbol`
- **THEN** сервер возвращает иерархию символов, пригодную для Outline

### Requirement: LSP поддерживает поиск символов workspace через workspace/symbol (MVP)
Система SHALL:
- объявлять `workspace_symbol_provider` в `ServerCapabilities`,
- реализовать `workspace/symbol`.

MVP-ограничение: поиск SHOULD выполняться по документам, доступным серверу в текущей сессии (минимально: открытые документы).

Сервер MUST:
- возвращать детерминированный порядок элементов,
- ограничивать размер ответа.

#### Scenario: Go to Symbol in Workspace находит символ в открытых документах
- **GIVEN** в сессии открыто несколько `.bsl` файлов с процедурами/функциями
- **WHEN** клиент вызывает `workspace/symbol` с query
- **THEN** сервер возвращает элементы, удовлетворяющие запросу, с корректными `Location` и `uri`

### Requirement: LSP поддерживает поиск ссылок и rename для поддерживаемых символов
Система SHALL поддерживать:
- `textDocument/references` для поиска ссылок на символ,
- `textDocument/rename` (и SHOULD `prepareRename`) для безопасного переименования поддерживаемых символов.

Ограничения (какие символы поддерживаются) MUST быть задокументированы и неизменно применяться сервером.

MVP поддерживаемые символы:
- локальные переменные, объявленные через `VarDeclaration` внутри процедуры/функции (rename/references в пределах одной процедуры/функции),
- процедуры/функции, объявленные в документе, и их прямые вызовы вида `Identifier(...)` в этом же документе.

Не поддерживаются (MVP):
- динамические обращения (строковые имена, косвенные вызовы, рефлексия),
- вызовы через property access (`Obj.Method()`), если под курсором не сам declaration.

#### Scenario: Find References возвращает ссылки на локальную переменную
- **GIVEN** в документе есть локальная переменная и её использования
- **WHEN** клиент вызывает `textDocument/references`
- **THEN** сервер возвращает locations всех ссылок (с учётом includeDeclaration)

#### Scenario: Rename меняет все ссылки на символ и не трогает другие имена
- **GIVEN** в документе есть символ и несколько ссылок на него
- **WHEN** клиент вызывает `textDocument/rename`
- **THEN** сервер возвращает WorkspaceEdit, который меняет только ссылки на этот символ

### Requirement: VS Code extension не регистрирует заглушки IntelliSense по умолчанию
Система SHALL обеспечивать, что VS Code extension не регистрирует “пустые” (stub) IntelliSense providers по умолчанию. Если provider зарегистрирован, он MUST возвращать осмысленный результат.

#### Scenario: Inlay hints / code actions не являются заглушками
- **GIVEN** пользователь включил `bsl.typeHints.enabled` и/или `bsl.codeActions.enabled`
- **WHEN** IDE запрашивает inlay hints или code actions
- **THEN** extension использует стандартный LSP pipeline (без кастомных заглушек), и фичи появляются только если сервер объявил соответствующие capabilities
- **AND** если сервер не объявил capability, extension явно логирует предупреждение и не обещает фичу пользователю

