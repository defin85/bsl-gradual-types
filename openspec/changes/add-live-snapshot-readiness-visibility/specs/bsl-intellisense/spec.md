## ADDED Requirements

### Requirement: VS Code extension показывает active-document snapshot readiness truthfully (MUST)

VS Code extension MUST показывать live snapshot readiness для активного BSL документа через
server-driven snapshot-status contract.

The extension MUST:

- показывать краткий state в правом `Status Bar` item для активного BSL editor;
- показывать detail view inside existing observability UI как минимум с requested/ready revision,
  state, `exact`, task state, coarse phase, and fallback reason when available;
- использовать authoritative snapshot-status request/notification как source of truth;
- не реконструировать readiness из diagnostics save timeline, completion timeline, Output logs, или
  aggregate observability metrics;
- явно различать `building`, exact `ready`, `stale`, `shadow_only`, and `failed`.

#### Scenario: Active document exact snapshot ещё строится
- **GIVEN** connected server reports snapshot status `state=building` for the active BSL document
- **WHEN** пользователь держит этот editor активным
- **THEN** extension показывает в статус-баре building state для текущей revision
- **AND** extension не маркирует документ как exact-ready

#### Scenario: Active document exact snapshot готов
- **GIVEN** connected server reports `state=ready` and `exact=true` for the active BSL document
- **WHEN** extension обновляет live snapshot readiness UI
- **THEN** status bar and observability detail show the document as exact-ready
- **AND** detail view exposes the ready revision that matches the requested revision

#### Scenario: `shadow_only` fallback не маскируется под ready
- **GIVEN** connected server reports `state=shadow_only` for the active BSL document
- **WHEN** extension renders snapshot readiness
- **THEN** extension показывает degraded snapshot state explicitly
- **AND** extension does not collapse that state into generic ready wording

#### Scenario: Unsupported server деградирует fail-closed
- **GIVEN** connected server does not support the snapshot-status contract
- **WHEN** extension initializes snapshot readiness UI
- **THEN** extension keeps the feature explicitly unavailable or hidden
- **AND** extension does not guess readiness from other observability surfaces
