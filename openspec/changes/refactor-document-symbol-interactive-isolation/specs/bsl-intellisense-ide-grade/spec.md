## MODIFIED Requirements
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
