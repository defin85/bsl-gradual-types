## ADDED Requirements

### Requirement: MCP tools имеют opt-in flow-sensitive режим с явной сигнализацией (MUST)
Система MUST поддерживать flow-sensitive режим (type narrowing и null-safety) в MCP tools, но MUST NOT включать его по умолчанию.

Для инструментов, которые зависят от “типа в позиции” и members/diagnostics, MUST быть предусмотрен параметр `include_flow_sensitive`,
default `false`.

Инструменты, которые MUST поддерживать этот параметр (минимум):
- `bsl_type_at_position_start`
- `bsl_members_start`
- `bsl_diagnostics_start`

Ответы этих инструментов MUST содержать явный индикатор effective режима (например, `flow_sensitive_enabled: bool`),
чтобы клиент мог отличить “режим выключен” от “режим включён, но narrowing не применился”.

#### Scenario: MCP type-at-position возвращает flow-sensitive тип только при явном включении
- **GIVEN** рабочая сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_type_at_position_start` без `include_flow_sensitive`
- **THEN** сервер возвращает базовый v2 тип (без flow-sensitive уточнений) и `flow_sensitive_enabled=false`
- **AND WHEN** клиент вызывает `bsl_type_at_position_start` с `include_flow_sensitive=true`
- **THEN** сервер возвращает flow-sensitive тип (если применимо) и `flow_sensitive_enabled=true`

#### Scenario: MCP diagnostics включает null-safety только при явном включении
- **GIVEN** рабочая сессия открыта и `ready=true`
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `include_flow_sensitive=true`
- **THEN** сервер возвращает diagnostics, включающие null-safety правила (если применимо), и `flow_sensitive_enabled=true`

