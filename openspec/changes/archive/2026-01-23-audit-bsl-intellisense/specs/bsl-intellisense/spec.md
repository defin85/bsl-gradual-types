## ADDED Requirements

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
