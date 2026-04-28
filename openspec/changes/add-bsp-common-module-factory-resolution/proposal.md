# Change: Resolve BSP common-module factory calls

## Why

Real BSP-style configurations often call optional subsystem modules through helpers such as `ОбщегоНазначения.ОбщийМодуль("УправлениеДоступом")`. v2 currently can treat the helper result as `Неопределено` or unknown, which produces false diagnostics for valid exported calls like `МодульУправлениеДоступом.ПриЧтенииНаСервере(...)`.

The helper is intentionally dynamic at runtime, but many calls pass a literal module name. Those cases are statically resolvable against the configuration common-module index and should keep the precise target module type.

## What Changes

- Add v2 call-site resolution for known BSP common-module factory helpers with literal module names.
- Return a precise configuration common-module type for literal common-module names such as `"УправлениеДоступом"`.
- Support manager-module targets for dotted names such as `"Справочники.НастройкиОбменСБанками"` when metadata is available.
- Keep dynamic, missing, or unindexed targets fail-closed: do not invent modules and do not emit high-confidence method errors from guessed targets.
- Manage factory rules through the existing rules configuration surface: repo-local `bsl-rules.toml` by default, with `bslAnalyzer.rulesConfig` or CLI flags able to point to another file.
- Add regression coverage for `CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl` lines 22-23.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/` method-call inference and semantic facts
  - `semantic-diagnostics/` member/method validation inputs
  - `bsl-runtime/`, `shared/`, or startup/runtime wiring for loading typed semantic pattern rules into `SemanticDeps` or settings snapshots
  - CLI/LSP/Web/VS Code consumers that surface diagnostics, hover, completion, definition, and rules configuration management
