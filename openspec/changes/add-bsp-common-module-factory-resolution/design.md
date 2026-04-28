## Context

The motivating real case is `examples/conf_big/CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl`:

```bsl
МодульУправлениеДоступом = ОбщегоНазначения.ОбщийМодуль("УправлениеДоступом");
МодульУправлениеДоступом.ПриЧтенииНаСервере(Форма, ТекущийОбъект);
```

The target procedure exists as an exported procedure in `examples/conf_big/CommonModules/УправлениеДоступом/Ext/Module.bsl`.

The BSP helper implementation in `CommonModules/ОбщегоНазначения/Ext/Module.bsl` checks `Метаданные.ОбщиеМодули.Найти(Имя)`, evaluates the literal module name through `Вычислить(Имя)`, and returns the module object. The helper can also route dotted names to manager modules. Generic return inference sees only dynamic `Вычислить(...)` and fallback `Неопределено`, so the result can be narrowed incorrectly.

Primary source:
- 1C documentation describes common-module calls: exported procedures/functions of global common modules can be called directly, while non-global common modules are called as `ModuleName.MethodName(...)`: https://kb.1ci.com/1C_Enterprise_Platform/Tutorials/Practical_developer_guide_8.3/Lesson_5._Theory/Modules/Form_module_context/

## Goals / Non-Goals

- Goals:
  - Resolve literal BSP common-module factory calls to precise configuration module types.
  - Preserve exported method validation, hover, completion, signature help, and definition for the resolved target module.
  - Support both direct common-module targets and dotted manager-module targets where metadata is available.
  - Make factory-rule storage explicit and manageable without rebuilding the analyzer.
  - Avoid false high-confidence diagnostics when the helper argument or target is not statically known.
- Non-Goals:
  - Do not evaluate arbitrary `Вычислить(...)` expressions.
  - Do not globally change the platform return type of `ОбщийМодуль`.
  - Do not make every unknown helper call dynamic-successful; missing literal targets should remain diagnosable when the configuration index is available.
  - Do not require live 1C runtime execution.

## Resolution Model

Add a dedicated call-site resolver before generic method return inference:

1. Load common-module factory rules from a centralized semantic pattern registry.
2. Detect a method call whose receiver and method name match an enabled factory rule.
3. Accept the receiver when it is one of the built-in BSP defaults, a project-configured rule, or a structurally inferred rule with explicit evidence.
4. Read the first argument only if it is a static string expression.
5. If the string contains no dot:
   - look up `ОбщиеМодули.<Имя>` in the configuration repository or signature index;
   - return `TypeResolution::metadata_type(MetadataKind::CommonModule, <Имя>, Singleton)` with strong or inferred certainty depending on evidence.
6. If the string contains one dot:
   - treat it as `<metadata collection>.<object name>`;
   - route through the existing metadata resolver to the manager-module/applied-object manager type.
7. Otherwise return an unknown/weak result and record uncertainty rather than pretending the helper returned `Неопределено`.

The resolver must be call-site-specific. It must not alter the stored signature of `ОбщегоНазначения.ОбщийМодуль`, because the same function can return different modules for different literal arguments.

## Factory Rule Registry

The implementation should not scatter helper names as hardcoded string checks inside type inference. It should introduce a small semantic pattern registry with three input layers:

1. Built-in defaults for common BSP helpers. These defaults are source-controlled and versioned with the analyzer.
2. Project/user overrides that can enable, disable, or add factory rules without recompiling the analyzer.
3. Optional structural inference for helper functions whose implementation/signature clearly matches the BSP contract, gated behind conservative evidence.

The physical project storage is `bsl-rules.toml` at the workspace or configuration root. VS Code already exposes `bslAnalyzer.rulesConfig` as the path to a rules configuration file and `bslAnalyzer.configureRules` as the command that opens or creates `<workspace>/bsl-rules.toml`; this change should reuse that surface instead of adding a second rules file. When `bslAnalyzer.rulesConfig` is empty, the default discovery path is `<workspace>/bsl-rules.toml`.

CLI entrypoints should use the same rules file format and discovery rule. If no CLI rules-config flag exists at implementation time, add `--rules-config <path>` for explicit override while keeping `bsl-rules.toml` discovery as the default.

The initial TOML shape should be data-driven enough to support:

```toml
[semantic.common_module_factories]
builtin_bsp = true

[[semantic.common_module_factories.rules]]
id = "bsp-common-purpose"
owner = "ОбщиеМодули.ОбщегоНазначения"
method = "ОбщийМодуль"
argument_index = 0
target_mode = "common_module_or_manager"
enabled = true

[[semantic.common_module_factories.rules]]
id = "custom-library-module"
owner = "ОбщиеМодули.МояБиблиотека"
method = "Модуль"
argument_index = 0
target_mode = "common_module"
enabled = true
```

To disable a built-in rule, the project file should override the same rule `id` with `enabled = false`. If the whole BSP built-in pack must be disabled, `builtin_bsp = false` disables the bundled defaults before explicit project rules are applied.

The effective registry is built in this order:

1. analyzer built-ins, guarded by a built-in registry schema/version;
2. workspace `bsl-rules.toml` or the path from `bslAnalyzer.rulesConfig` / `--rules-config`;
3. normalized project overrides keyed by stable `id`.

The file is parsed at the runtime/config boundary (`bsl-runtime`, backend startup, CLI preparation, or the nearest existing config loader). `analysis-v2` receives only an immutable typed registry in its settings/dependency snapshot and must not perform file IO.

The effective registry identity must include:

- built-in registry schema/version;
- resolved rules config path, when present;
- content hash of the parsed rules file;
- parse status and normalized enabled rule set.

This identity participates in the effective `settings_id` and semantic cache key, so changing `bsl-rules.toml`, `bslAnalyzer.rulesConfig`, or `--rules-config` invalidates cached semantic artifacts.

Malformed project config must fail closed: keep bundled built-ins only when they were not explicitly disabled by a valid parsed config, ignore invalid project rules, report a configuration diagnostic, and avoid partially applying a broken override.

Built-in defaults are acceptable as a bootstrap, but they must live behind the registry API. Adding another BSP-like helper must not require editing the core call-resolution logic.

## Repository and Runtime Requirements

The high-confidence path depends on configuration module indexing:

- common module metadata such as `CommonModules/УправлениеДоступом.xml` must produce a `RawTypeData` or signature-index owner for `ОбщиеМодули.УправлениеДоступом`;
- exported declarations from `CommonModules/УправлениеДоступом/Ext/Module.bsl` must be indexed under that owner;
- definition locations should continue to point at the exported procedure declaration.

If a single-file CLI check is run without configuration metadata/signatures loaded, the resolver must degrade to unknown/weak instead of returning `Неопределено` or emitting a precise missing-method error. A separate workspace-loading issue can improve CLI discovery, but this change owns the helper call semantics once the configuration index is present.

## Diagnostics Behavior

For a resolved literal module target:

- `МодульУправлениеДоступом.ПриЧтенииНаСервере(...)` uses the exported methods of `ОбщиеМодули.УправлениеДоступом`;
- unknown method diagnostics are valid when the target module is known and the method is absent;
- hover/completion should show the resolved common-module target rather than `Неопределено`;
- go-to-definition can use existing method definition locations.

For unresolved targets:

- dynamic argument: no high-confidence missing-method diagnostic from the factory result;
- missing literal module with loaded configuration metadata: emit a targeted unresolved-module diagnostic or weak type note, but do not claim the factory result is `Неопределено`;
- missing configuration index: mark uncertainty as configuration-not-loaded or equivalent.

## Alternatives Considered

- Trust the inferred return type of `ОбщегоНазначения.ОбщийМодуль`: rejected because the body uses `Вычислить` and fallback branches, so local body inference cannot recover argument-specific module identity.
- Change the helper's signature to return platform type `ОбщийМодуль`: rejected because it is too imprecise for exported-method validation and would still not identify `УправлениеДоступом`.
- Treat all factory results as dynamic: rejected because it suppresses real mistakes and loses completion/definition for the common case where the module name is a literal.

## Open Questions

- Should unresolved literal module names become warnings only when full configuration metadata is loaded, or remain informational until coverage of configuration indexing is stricter?
