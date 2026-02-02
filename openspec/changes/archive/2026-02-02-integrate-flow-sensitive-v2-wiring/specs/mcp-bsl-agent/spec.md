## MODIFIED Requirements

### Requirement: MCP ответы явно сигнализируют effective flow-sensitive режим (MUST)
MCP сервер MUST поддерживать flow-sensitive режим (type narrowing и null-safety) в инструментах, где он может менять результат, но MUST NOT включать его по умолчанию.

Минимально это относится к:
- `bsl_type_at_position_start`
- `bsl_members_start`
- `bsl_diagnostics_start`

Для этих инструментов MUST быть предусмотрен параметр `include_flow_sensitive` (или эквивалентный), default `false`.

Ответы MUST содержать поле `flow_sensitive_enabled: bool`, которое отражает effective режим для конкретного вызова.

#### Scenario: `flow_sensitive_enabled` соответствует параметру запроса
- **GIVEN** рабочая сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_type_at_position_start` без `include_flow_sensitive`
- **THEN** ответ содержит `flow_sensitive_enabled=false`
- **AND WHEN** клиент вызывает `bsl_type_at_position_start` с `include_flow_sensitive=true`
- **THEN** ответ содержит `flow_sensitive_enabled=true`

#### Scenario: Включённый режим отличим от “narrowing не применился”
- **GIVEN** клиент вызывает `bsl_type_at_position_start` с `include_flow_sensitive=true`
- **WHEN** в конкретной позиции нет применимого narrowing (тип не сужается)
- **THEN** `flow_sensitive_enabled=true`, даже если возвращённый тип совпадает с базовым

#### Scenario: MCP diagnostics включает null-safety только при явном включении
- **GIVEN** рабочая сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `include_flow_sensitive=true`
- **THEN** сервер возвращает diagnostics, включающие null-safety правила (если применимо)
