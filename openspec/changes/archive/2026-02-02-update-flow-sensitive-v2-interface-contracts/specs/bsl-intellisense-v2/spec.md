## ADDED Requirements

### Requirement: Единый контракт включения flow-sensitive в v2 интерфейсах (MUST)
Система MUST иметь единый, недвусмысленный контракт включения flow-sensitive режима во всех v2 интерфейсах (IDE/LSP и Web API), чтобы клиент мог надёжно управлять производительностью и результатами.

#### Scenario: Web API принимает только `includeFlowSensitive` (breaking)
- **GIVEN** клиент вызывает Web API endpoint, поддерживающий flow-sensitive режим
- **WHEN** клиент передаёт `includeFlowSensitive=true`
- **THEN** сервер включает flow-sensitive вычисления для запроса
- **AND WHEN** клиент передаёт `include_flow_sensitive=true`
- **THEN** сервер отвечает `400 Bad Request` и явно сообщает, что поддерживается только `includeFlowSensitive`

### Requirement: `bsl.getSemanticTree` подчиняется `enableFlowSensitive`, если параметр не указан (MUST)
Если LSP custom request `bsl.getSemanticTree` не содержит явного параметра `include_flow_sensitive`, система MUST определять effective режим по workspace setting `enableFlowSensitive`.

Если `include_flow_sensitive` указан явно, он MUST иметь приоритет над `enableFlowSensitive`.

#### Scenario: `bsl.getSemanticTree` без параметра использует `enableFlowSensitive`
- **GIVEN** workspace setting `enableFlowSensitive=false`
- **WHEN** IDE вызывает `bsl.getSemanticTree` без `include_flow_sensitive`
- **THEN** сервер вычисляет результат без flow-sensitive вычислений

#### Scenario: `bsl.getSemanticTree` с параметром переопределяет настройку
- **GIVEN** workspace setting `enableFlowSensitive=false`
- **WHEN** IDE вызывает `bsl.getSemanticTree` с `include_flow_sensitive=true`
- **THEN** сервер включает flow-sensitive вычисления для запроса
