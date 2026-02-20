## MODIFIED Requirements

### Requirement: VS Code extension запускает LSP и предоставляет IDE‑интеграцию
Система SHALL предоставлять VS Code extension, который запускает LSP-сервер, прокидывает настройки (пути к документации платформы/конфигурации) и обеспечивает базовую IDE-интеграцию для BSL.

В части completion extension MUST:
- не блокировать и не подменять trigger-character completion pipeline LSP для BSL;
- обеспечивать предсказуемую диагностику клиентской конфигурации, когда effective editor settings отключают автотриггер suggestions по trigger symbols.

#### Scenario: Расширение VS Code поднимает LSP и включает базовые подсказки
- **GIVEN** пользователь открыл workspace с `.bsl` файлами в VS Code
- **WHEN** расширение активируется
- **THEN** LSP-клиент стартует, и пользователь получает completion/hover/signatureHelp/diagnostics в редакторе

#### Scenario: Автотриггер completion по `.` отключён в effective settings
- **GIVEN** для BSL effective конфигурация редактора отключает trigger-based suggestions (например, `editor.suggestOnTriggerCharacters=false`)
- **WHEN** расширение инициализирует LSP-интеграцию
- **THEN** extension явно логирует предупреждение с причиной и шагом исправления
- **AND** extension не меняет пользовательские editor settings автоматически
