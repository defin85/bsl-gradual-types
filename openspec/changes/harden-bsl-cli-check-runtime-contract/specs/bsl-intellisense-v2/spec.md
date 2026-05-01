## ADDED Requirements

### Requirement: `bsl-cli check` SHALL expose a machine-readable diagnostics contract

`bsl-cli check` SHALL provide a documented diagnostics output contract for automation. When JSON output is requested, stdout SHALL contain parseable JSON with the checked path, diagnostic counts, diagnostic entries, exit classification, and runtime evidence metadata.

The JSON diagnostics contract SHALL NOT require `--verbose` to expose diagnostic entries. Human-oriented output MAY use a summary by default, but the detailed human diagnostics mode SHALL be explicit and documented.

#### Scenario: JSON check output contains diagnostics without verbose mode

- **GIVEN** a BSL file that produces at least one diagnostic
- **WHEN** the user runs `bsl-cli check --format json <file>`
- **THEN** stdout contains valid JSON
- **AND** the JSON contains the checked file path
- **AND** the JSON contains diagnostic counts
- **AND** the JSON contains diagnostic entries with severity, message, and source range when available
- **AND** the command does not require `--verbose` to include those diagnostic entries

#### Scenario: JSON stdout is safe for automation

- **GIVEN** the user requests `--format json`
- **WHEN** `bsl-cli check` emits runtime logs, warnings, or progress messages
- **THEN** those human messages do not corrupt the JSON stdout stream
- **AND** automation can parse stdout as JSON without stripping terminal text

#### Scenario: Human diagnostics mode is explicit

- **GIVEN** a BSL file that produces diagnostics
- **WHEN** the user requests a documented human diagnostics format or flag combination
- **THEN** the command prints the diagnostics in a human-readable form
- **AND** the documentation identifies which command form prints counts only and which command form prints diagnostic details

### Requirement: `bsl-cli check` SHALL report runtime evidence for its analysis mode

`bsl-cli check` SHALL report enough runtime evidence to distinguish a fast single-file diagnostics run from a configuration-backed workspace run and from exact-index or live/LSP verification.

The report SHALL include whether a configuration root was provided, whether configuration metadata was loaded, whether syntax-helper resources were available, which rules config was applied when known, and whether exact type-index readiness was requested or achieved.

The CLI SHALL NOT present no-config single-file diagnostics as proof that configuration metadata, exact type-index snapshots, or live LSP state were used.

#### Scenario: No-config check reports its limitations

- **GIVEN** the user runs `bsl-cli check --format json <file>` without a configuration root or workspace input
- **WHEN** the report is produced
- **THEN** the runtime evidence marks configuration metadata as not provided or not loaded
- **AND** exact type-index readiness is marked as not requested or not applicable for that run
- **AND** the report does not imply live LSP verification

#### Scenario: Configuration-backed check reports loaded configuration evidence

- **GIVEN** the user runs `bsl-cli check` with an explicit configuration root or workspace input
- **WHEN** the configuration metadata is loaded successfully
- **THEN** the runtime evidence marks the configuration as loaded
- **AND** the report includes stable evidence for the effective configuration input, such as an effective path or equivalent fingerprint

#### Scenario: Malformed explicit configuration input fails closed

- **GIVEN** the user passes an explicit configuration root or workspace input
- **AND** that input is missing, malformed, or cannot be loaded
- **WHEN** `bsl-cli check` runs
- **THEN** the report marks configuration loading as failed
- **AND** the command does not silently downgrade to a no-config run while implying configuration-backed evidence

### Requirement: `bsl-cli check` SHALL preserve a truthful exact-index policy

`bsl-cli check` SHALL distinguish diagnostics-ready dependency snapshots from exact type-index snapshots. Diagnostics MAY run without exact type-index warmup by default, but the CLI report SHALL expose whether exact type-index preparation was requested and whether it completed.

Exact type-index warmup SHALL only be performed when the selected CLI mode or operation requires it, or when the user explicitly requests a mode that promises exact-index evidence.

#### Scenario: Default diagnostics do not claim exact-index readiness

- **GIVEN** the user runs the default diagnostics check mode
- **WHEN** no exact type-index warmup is performed
- **THEN** the report marks exact type-index status as not requested, not applicable, or equivalent
- **AND** downstream documentation does not treat that result as exact snapshot proof

#### Scenario: Exact-index mode reports readiness

- **GIVEN** the user requests a mode that promises exact type-index evidence
- **WHEN** exact type-index warmup completes
- **THEN** the report marks exact type-index status as ready
- **AND** failures to prepare exact-index evidence are visible in the report and exit classification

### Requirement: `bsl-cli check` SHALL keep known global collection manager chains stable in single-file mode

In the default single-file diagnostics mode with syntax-helper global context available, `bsl-cli check` SHALL NOT emit high-confidence unknown member diagnostics for known global collection manager chains that can be resolved from platform manager types.

This requirement covers both literal global collection names and syntax-helper manager types such as document and enumeration managers.

#### Scenario: Conf_big авансовый отчет global collection chains remain resolved

- **GIVEN** syntax-helper global context is available to `bsl-cli check`
- **AND** the user checks `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl`
- **WHEN** diagnostics are produced in default single-file mode
- **THEN** `Перечисления.ВидыОперацийАвансовыйОтчет.Командировка` does not produce a high-confidence unknown member diagnostic for `Командировка`
- **AND** `Документы.АвансовыйОтчет.Выбрать()` does not produce a high-confidence unknown member diagnostic for `Выбрать`
