## ADDED Requirements

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
