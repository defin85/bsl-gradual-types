## ADDED Requirements

### Requirement: VS Code extension не регистрирует заглушки IntelliSense по умолчанию
Система SHALL обеспечивать, что VS Code extension не регистрирует “пустые” (stub) IntelliSense providers по умолчанию. Если provider зарегистрирован, он MUST возвращать осмысленный результат.

#### Scenario: Inlay hints / code actions не являются заглушками
- **GIVEN** пользователь включил `bsl.typeHints.enabled` и/или `bsl.codeActions.enabled`
- **WHEN** IDE запрашивает inlay hints или code actions
- **THEN** extension использует стандартный LSP pipeline (без кастомных заглушек), и фичи появляются только если сервер объявил соответствующие capabilities
- **AND** если сервер не объявил capability, extension явно логирует предупреждение и не обещает фичу пользователю
