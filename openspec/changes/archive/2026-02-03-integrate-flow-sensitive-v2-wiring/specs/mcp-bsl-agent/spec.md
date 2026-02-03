## MODIFIED Requirements

### Requirement: MCP tools поддерживают flow-sensitive режим как опциональный флаг (MUST)
Система MUST поддерживать flow-sensitive режим (type narrowing и null-safety) в MCP tools, но MUST NOT включать его по умолчанию.

Для инструментов, которые зависят от “типа в позиции” и members/diagnostics, MUST быть предусмотрен параметр `include_flow_sensitive` (или эквивалентный), default `false`.

Под “инструментами, зависящими от flow-sensitive” в рамках этого требования понимаются минимум:
- `bsl_type_at_position_start`
- `bsl_members_start`
- `bsl_diagnostics_start`

#### Scenario: MCP type-at-position возвращает flow-sensitive тип только при явном включении
- **GIVEN** рабочая сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_type_at_position_start` без `include_flow_sensitive`
- **THEN** сервер возвращает базовый v2 тип (без flow-sensitive уточнений)
- **AND WHEN** клиент вызывает `bsl_type_at_position_start` с `include_flow_sensitive=true`
- **THEN** сервер возвращает уточнённый flow-sensitive тип (если применимо) и явно указывает, что режим включён

#### Scenario: MCP diagnostics включает null-safety только при явном включении
- **GIVEN** рабочая сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `include_flow_sensitive=true`
- **THEN** сервер возвращает diagnostics, включающие null-safety правила (если применимо)

