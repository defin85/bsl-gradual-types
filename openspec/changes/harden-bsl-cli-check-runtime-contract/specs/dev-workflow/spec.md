## ADDED Requirements

### Requirement: Dev verification SHALL include a truthful `bsl-cli check` smoke for CLI diagnostic changes

Changes that affect `bsl-cli check`, CLI diagnostics formatting, runtime dependency preparation, syntax-helper loading, rules config loading, configuration-root plumbing, or exact-index evidence SHALL include a targeted CLI smoke in their validation evidence.

The smoke evidence SHALL state which mode was used: single-file no-config, configuration-backed, exact-index, or live/LSP. Validation notes SHALL NOT present a weaker mode as proof for a stronger one.

#### Scenario: CLI diagnostic changes include `bsl-cli check` smoke evidence

- **GIVEN** a change affects `bsl-cli check` or its runtime diagnostic inputs
- **WHEN** the implementation is handed off for review
- **THEN** the validation evidence includes the exact `bsl-cli check` command that was run
- **AND** the evidence states whether configuration metadata was loaded
- **AND** the evidence states whether exact type-index warmup was requested or completed
- **AND** the evidence states whether the run was single-file CLI smoke or live/LSP verification

#### Scenario: Regression smoke covers the conf_big авансовый отчет file

- **GIVEN** a change affects global context, global collection access, or CLI diagnostics
- **WHEN** the repo-owned targeted smoke suite is run
- **THEN** it includes `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` or a smaller fixture that preserves the same `Командировка` and `Выбрать` failure mode
- **AND** the smoke fails if those member accesses reappear as high-confidence unknown member diagnostics
