## ADDED Requirements

### Requirement: MCP ответы явно сигнализируют effective flow-sensitive режим (MUST)
MCP сервер MUST явно сигнализировать effective flow-sensitive режим в ответах инструментов, где flow-sensitive может менять результат.

Минимально это относится к:
- `bsl_type_at_position_start`
- `bsl_members_start`
- `bsl_diagnostics_start`

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
