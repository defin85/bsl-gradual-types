## ADDED Requirements

### Requirement: v2 SHALL resolve BSP common-module factory calls with literal common-module names

The system SHALL recognize enabled common-module factory rules such as `ОбщегоНазначения.ОбщийМодуль("<ИмяОбщегоМодуля>")` when the first argument is statically known.

For a literal argument that names a common module available in the configuration repository or signature index, the system SHALL return the concrete configuration common-module singleton type for that literal target. The resolution SHALL be call-site-specific and SHALL NOT globally replace the helper method's return type.

When the argument is dynamic, the target module is unavailable, or configuration module evidence is missing, the system SHALL fail closed by returning an unknown or weak type instead of incorrectly narrowing the result to `Неопределено`.

Common-module factory recognition SHALL be backed by a centralized semantic pattern registry. The registry SHALL include built-in BSP defaults and SHALL provide a project/user override mechanism to enable, disable, or add rules without modifying core inference code.

Project/user overrides SHALL be stored in `bsl-rules.toml` by default. VS Code SHALL use `bslAnalyzer.rulesConfig` as the explicit rules file path when it is set and SHALL use the repo-local `<workspace>/bsl-rules.toml` path when it is not set. CLI entrypoints SHALL use the same file format and SHALL provide default discovery plus an explicit path override.

The effective registry SHALL participate in semantic settings/cache identity so registry changes invalidate affected semantic artifacts. The identity SHALL include the built-in registry schema/version, the resolved rules file path when present, the rules file content hash, parse status, and the normalized enabled rule set.

Malformed project rules SHALL fail closed: the system SHALL report a configuration diagnostic, SHALL NOT partially apply invalid overrides, and SHALL NOT disable bundled built-ins based on an invalid file.

#### Scenario: Literal BSP helper target resolves to a common module

- **GIVEN** the configuration index contains common module `УправлениеДоступом`
- **AND** `ОбщиеМодули.УправлениеДоступом` contains exported procedure `ПриЧтенииНаСервере`
- **WHEN** v2 analyzes `МодульУправлениеДоступом = ОбщегоНазначения.ОбщийМодуль("УправлениеДоступом")`
- **THEN** `МодульУправлениеДоступом` resolves to the concrete common-module target `ОбщиеМодули.УправлениеДоступом`
- **AND** the result is not narrowed to `Неопределено`

#### Scenario: Dynamic helper target fails closed

- **GIVEN** `ИмяМодуля` is not statically known
- **WHEN** v2 analyzes `Модуль = ОбщегоНазначения.ОбщийМодуль(ИмяМодуля)`
- **THEN** the factory result is marked unknown or weak
- **AND** v2 does not invent a concrete common-module target
- **AND** v2 does not emit a high-confidence missing-method diagnostic based on a guessed target

#### Scenario: Project override adds a custom helper

- **GIVEN** `bsl-rules.toml` enables a common-module factory rule for `МояБиблиотека.Модуль("<ИмяОбщегоМодуля>")`
- **AND** the configuration index contains the literal target module
- **WHEN** v2 analyzes a call through that custom helper
- **THEN** the helper result resolves using the configured rule
- **AND** no core inference code change is required for that helper name

#### Scenario: Project override disables a built-in helper

- **GIVEN** `bsl-rules.toml` disables the built-in rule for `ОбщегоНазначения.ОбщийМодуль`
- **WHEN** v2 analyzes `ОбщегоНазначения.ОбщийМодуль("УправлениеДоступом")`
- **THEN** the BSP factory special-case is not applied
- **AND** the semantic cache identity reflects the changed registry

#### Scenario: Explicit VS Code rules config path overrides the default

- **GIVEN** `bslAnalyzer.rulesConfig` points to `/project/config/custom-bsl-rules.toml`
- **WHEN** v2 builds its effective semantic settings for that workspace
- **THEN** it loads factory rules from `/project/config/custom-bsl-rules.toml`
- **AND** the default `<workspace>/bsl-rules.toml` file is not used for factory-rule overrides

#### Scenario: Rules file changes invalidate semantic artifacts

- **GIVEN** v2 has cached semantic artifacts for a workspace using `bsl-rules.toml`
- **WHEN** the rules file content changes or the explicit rules path changes
- **THEN** the effective semantic settings identity changes
- **AND** cached semantic artifacts for the old registry are not reused

#### Scenario: Malformed rules config fails closed

- **GIVEN** `bsl-rules.toml` contains invalid common-module factory rule syntax
- **WHEN** v2 builds the effective semantic settings
- **THEN** the system reports a configuration diagnostic for the rules file
- **AND** invalid project overrides are not applied
- **AND** bundled built-ins are not disabled by the malformed file

### Requirement: v2 SHALL validate exported members on resolved BSP factory results

When a variable receives a known common-module target from a BSP common-module factory call, the system SHALL validate member and method access using the exported members indexed for that concrete target module.

This behavior SHALL apply consistently to diagnostics, hover, completion, signature help, and definition where those features already consume v2 semantic facts.

#### Scenario: Conf_big УправлениеДоступом factory call is accepted

- **GIVEN** `CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` calls `ОбщегоНазначения.ОбщийМодуль("УправлениеДоступом")`
- **AND** the configuration index contains exported procedure `ПриЧтенииНаСервере` in `CommonModules/УправлениеДоступом/Ext/Module.bsl`
- **WHEN** v2 diagnostics analyze `МодульУправлениеДоступом.ПриЧтенииНаСервере(Форма, ТекущийОбъект)`
- **THEN** no missing-method diagnostic is emitted for `ПриЧтенииНаСервере`
- **AND** hover/type-at-position for `МодульУправлениеДоступом` indicates the resolved `УправлениеДоступом` common-module target

#### Scenario: Known target still reports absent exported method

- **GIVEN** a BSP factory call resolves to known common module `УправлениеДоступом`
- **AND** `УправлениеДоступом` does not export method `НесуществующийМетод`
- **WHEN** v2 diagnostics analyze `МодульУправлениеДоступом.НесуществующийМетод()`
- **THEN** the system emits a missing-method diagnostic against the known target module

### Requirement: v2 SHALL support dotted BSP factory names as manager-module targets when metadata is available

When a BSP common-module factory literal contains a single dot, the system SHALL treat it as a metadata manager-module target name and SHALL resolve it through existing metadata resolver paths when metadata is available.

If metadata is unavailable or the dotted target cannot be resolved, the system SHALL fail closed without claiming the result is `Неопределено`.

#### Scenario: Dotted literal resolves to manager-module target

- **GIVEN** metadata contains `Справочники.НастройкиОбменСБанками`
- **WHEN** v2 analyzes `МодульМенеджера = ОбщегоНазначения.ОбщийМодуль("Справочники.НастройкиОбменСБанками")`
- **THEN** the factory result resolves to the corresponding manager-module/applied-object manager target
- **AND** subsequent exported method validation uses that resolved target
