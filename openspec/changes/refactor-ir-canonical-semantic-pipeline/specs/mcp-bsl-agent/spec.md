## MODIFIED Requirements

### Requirement: `bsl-agent` semantic tools используют общий v2 facade/runtime (MUST)
Система MUST выполнять production semantic tools `bsl-agent` через тот же shared v2 facade/runtime контракт, который используется в LSP и web, и этот контракт MUST быть rooted in canonical IR + derived semantic index текущей revision.

В рамках этого требования migration MUST быть полной: после завершения change в `bsl-agent` не должно оставаться production semantic tools с ad-hoc orchestration через локально собранный `AnalysisHostV2` pipeline или через adapter-local non-IR semantic path.

#### Scenario: Semantic tools MCP выполняются через canonical shared orchestration path
- **GIVEN** активная ready-сессия `bsl-agent`
- **WHEN** клиент вызывает `bsl_diagnostics`, `bsl_type_at_position`, `bsl_members` и `bsl_definition`
- **THEN** все операции выполняются через общий facade/runtime path, основанный на canonical IR и derived semantic index
- **AND** поведение cancellation/performance policy совпадает с контрактом LSP/web

## ADDED Requirements

### Requirement: Interactive MCP semantic tools читают только IR-derived semantic index (MUST)
Latency-critical MCP инструменты (`bsl_type_at_position`, `bsl_members`, `bsl_definition`) MUST читать base semantic truth только из derived semantic index текущей canonical IR revision.

При `include_flow_sensitive=true` они MAY добавлять canonical IR/CFG-based overlay поверх того же base contract, но MUST NOT использовать другой semantic source.

#### Scenario: MCP type-at-position использует тот же base contract, что и LSP/web
- **GIVEN** активная ready-сессия `bsl-agent`
- **WHEN** клиент вызывает `bsl_type_at_position_start` без `include_flow_sensitive`
- **THEN** сервер отвечает на основе derived semantic index текущей canonical IR revision
- **AND** не использует adapter-local serve-only/full-fallback semantic path

#### Scenario: MCP members использует canonical owner/member truth
- **GIVEN** активная ready-сессия `bsl-agent`
- **WHEN** клиент вызывает `bsl_members_start`
- **THEN** owner/member resolution опирается на canonical IR и derived semantic index той же revision
- **AND** результат не зависит от локального MCP-only semantic reconstruction

### Requirement: MCP semantic tools fail-closed при miss canonical artifacts (MUST)
Если canonical IR или derived semantic index текущей revision недоступны, semantic MCP tools MUST завершаться fail-closed.

Fail-closed для MCP означает:
- пустой или `None` semantic payload там, где это допустимо публичным contract;
- explicit warning/unavailable indication, если она уже предусмотрена surface;
- отсутствие adapter-local semantic fallback, усиливающего truth.

MCP MUST NOT использовать `serve_only -> full` semantic fallback как substitute для canonical IR-derived path.

#### Scenario: MCP members не строит alternate semantic result при miss current revision
- **GIVEN** active session revision ещё не имеет canonical IR или derived semantic index
- **WHEN** клиент вызывает `bsl_members_start`
- **THEN** сервер возвращает fail-closed empty/unavailable semantic result для этой revision
- **AND** не materialize-ит members из alternate non-IR semantic path
