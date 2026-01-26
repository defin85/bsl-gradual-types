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

