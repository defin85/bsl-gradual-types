## ADDED Requirements
### Requirement: Incident bundle экспортирует diagnostics save timeline как отдельный authoritative source (MUST)
VS Code extension MUST экспортировать request-centric diagnostics save timeline как отдельный source внутри
observability incident bundle, если connected server поддерживает этот контракт.

Этот source MUST:

- быть отдельным от completion timeline;
- содержать raw attachment `raw/diagnostics_save_timeline.json`;
- отображаться в `incident.json` и `summary.md` как `authoritative_server_trace`;
- не реконструироваться из cumulative `observability_metrics`.

#### Scenario: Bundle содержит отдельный diagnostics save timeline source
- **GIVEN** connected server поддерживает diagnostics save timeline request
- **WHEN** пользователь экспортирует observability incident bundle
- **THEN** bundle содержит `raw/diagnostics_save_timeline.json`
- **AND** `incident.json` и `summary.md` показывают diagnostics save timeline как отдельный authoritative source
- **AND** completion timeline и diagnostics save timeline не смешиваются в один raw attachment

#### Scenario: Bundle fail-closed деградирует без diagnostics save timeline
- **GIVEN** connected server не поддерживает diagnostics save timeline request
- **WHEN** пользователь экспортирует observability incident bundle
- **THEN** bundle всё равно создаётся
- **AND** `incident.json` и `summary.md` явно помечают diagnostics save timeline как `unsupported` или `unavailable`
- **AND** extension не пытается восстановить diagnostics save trace из metrics snapshot

### Requirement: Incident bundle summary показывает didSave refresh как request-centric diagnostics cycle (MUST)
`summary.md` и `incident.json` MUST переносить diagnostics save timeline в человекочитаемом request-centric виде.

Human-readable projection MUST:

- показывать `uri`, `requested_version` и bounded first-publish outcome;
- различать `save_fastlane` first publish и `idle_heavy` follow-up;
- показывать, был ли first publish `syntax_only` или `full`;
- не переименовывать aggregate metrics p95/p99 в request-level факты.

#### Scenario: Summary показывает first publish и follow-up без guesswork
- **GIVEN** diagnostics save timeline trace содержит `save_fastlane` first publish и `idle_heavy` follow-up
- **WHEN** extension формирует `summary.md`
- **THEN** summary показывает оба bounded факта внутри одного save refresh cycle
- **AND** оператор может отличить first freshness boundary от final richer publish

#### Scenario: Summary не заменяет request trace cumulative histogram-ом
- **GIVEN** bundle содержит и diagnostics save timeline, и cumulative observability metrics
- **WHEN** extension формирует `incident.json` и `summary.md`
- **THEN** request summary использует authoritative diagnostics save trace для request-level фактов
- **AND** cumulative metrics остаются только snapshot supplement
