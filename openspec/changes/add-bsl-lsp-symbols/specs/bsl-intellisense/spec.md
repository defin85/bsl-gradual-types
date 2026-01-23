## ADDED Requirements

### Requirement: LSP поддерживает символы документа и workspace для BSL
Система SHALL поддерживать:
- `textDocument/documentSymbol` (outline/structure),
- `workspace/symbol` (поиск символов по workspace).

#### Scenario: IDE строит outline по BSL файлу
- **GIVEN** открыт `.bsl` файл
- **WHEN** клиент вызывает `textDocument/documentSymbol`
- **THEN** сервер возвращает список символов (процедуры/функции) с корректными диапазонами

#### Scenario: IDE ищет символ по workspace
- **GIVEN** workspace содержит BSL‑код
- **WHEN** клиент вызывает `workspace/symbol` с текстовым запросом
- **THEN** сервер возвращает релевантные символы с локациями
