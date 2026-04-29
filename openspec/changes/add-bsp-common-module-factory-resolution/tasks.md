## 1. Investigation and Fixtures

- [x] 1.1 Add a minimal synthetic fixture for `ОбщегоНазначения.ОбщийМодуль("УправлениеДоступом")` returning a common module with an exported procedure.
- [x] 1.2 Add a conf_big regression fixture for `CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` lines 22-23.
- [x] 1.3 Verify that `УправлениеДоступом.ПриЧтенииНаСервере` is indexed under `ОбщиеМодули.УправлениеДоступом` when configuration metadata is loaded.

## 2. Call-Site Resolver

- [x] 2.1 Add a helper to recognize static string arguments, including literal concatenation only when fully static.
- [x] 2.2 Add a semantic pattern registry for common-module factory rules with built-in BSP defaults behind a registry API.
- [x] 2.3 Define and parse the `bsl-rules.toml` TOML schema for `[semantic.common_module_factories]`, `builtin_bsp`, and `rules`.
- [x] 2.4 Reuse `bslAnalyzer.rulesConfig` and `bslAnalyzer.configureRules` so VS Code opens or creates the repo-local rules file and passes the effective path to the backend.
- [x] 2.5 Add CLI support for the same rules file format, including default `bsl-rules.toml` discovery and an explicit `--rules-config <path>` override if no equivalent flag exists.
- [x] 2.6 Convert parsed rules into an immutable typed registry in the runtime/config boundary and pass it into v2 settings/dependency snapshots without file IO in `analysis-v2`.
- [x] 2.7 Include built-in registry schema/version, resolved rules path, rules content hash, parse status, and normalized enabled rule set in the semantic settings/cache key.
- [x] 2.8 Add config diagnostics for malformed rules files and ensure invalid project overrides are ignored fail-closed.
- [x] 2.9 Add a v2 special resolver for enabled common-module factory rules before generic method return inference.
- [x] 2.10 Resolve literal no-dot names to `MetadataKind::CommonModule` singleton targets through repository/signature-index evidence.
- [x] 2.11 Resolve dotted names through the existing metadata manager-module path when metadata is available.
- [x] 2.12 Ensure dynamic, missing, or unindexed targets degrade to unknown/weak rather than `Неопределено`.

## 3. Diagnostics and IDE Facts

- [x] 3.1 Ensure `call_receiver_type_by_span` and member/method validation receive the resolved target type.
- [x] 3.2 Ensure hover/type-at-position on the assigned variable shows the concrete module target.
- [x] 3.3 Ensure completion and signature help use the resolved module's exported members.
- [x] 3.4 Ensure go-to-definition for the resolved method keeps existing exported declaration locations.

## 4. Regression Coverage

- [x] 4.1 Add an analysis-v2 unit test proving `ОбщегоНазначения.ОбщийМодуль("УправлениеДоступом").ПриЧтенииНаСервере(...)` resolves against `ОбщиеМодули.УправлениеДоступом`.
- [x] 4.2 Add a semantic-diagnostics regression proving no false missing-method diagnostic is emitted for the conf_big call.
- [x] 4.3 Add a negative regression proving a missing method is still reported when the literal target module is known.
- [x] 4.4 Add a dynamic-argument regression proving no high-confidence missing-method diagnostic is emitted when the factory argument is unknown.
- [x] 4.5 Add a dotted-name regression for manager-module targets where metadata is available.
- [x] 4.6 Add registry override tests proving a built-in helper can be disabled and a custom helper can be added through `bsl-rules.toml` without changing inference code.
- [x] 4.7 Add rules-config identity tests proving a file content or path change invalidates v2 semantic artifacts.
- [x] 4.8 Add malformed `bsl-rules.toml` tests proving config diagnostics are emitted and invalid overrides are not partially applied.

## 5. Validation

- [x] 5.1 Run targeted `cargo test -p bsl-analysis-v2` tests.
- [x] 5.2 Run targeted `cargo test -p bsl-diagnostics` tests.
- [x] 5.3 Run targeted backend/runtime tests that cover configuration module indexing.
- [x] 5.4 Run `cargo fmt --all -- --check`.
- [x] 5.5 Run `openspec validate add-bsp-common-module-factory-resolution --strict --no-interactive`.
