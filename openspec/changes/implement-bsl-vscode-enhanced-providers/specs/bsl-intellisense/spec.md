## ADDED Requirements

### Requirement: VS Code extension не регистрирует заглушки IntelliSense по умолчанию
Система SHALL обеспечивать, что VS Code extension не регистрирует “пустые” (stub) IntelliSense providers по умолчанию. Если provider зарегистрирован, он MUST возвращать осмысленный результат.

#### Scenario: Inlay hints / code actions не являются заглушками
- **GIVEN** включены соответствующие настройки/фичи extension
- **WHEN** IDE запрашивает inlay hints или code actions
- **THEN** extension возвращает осмысленные результаты, либо фича явно отключена и не обещается пользователю
