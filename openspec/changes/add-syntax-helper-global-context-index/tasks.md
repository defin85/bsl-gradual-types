## 1. Syntax Helper Extraction

- [x] 1.1 Add or extend parser data structures so global-context property pages preserve Russian name, English name, type, readonly flag, contexts, description, and source key.
- [x] 1.2 Preserve property source path/provenance through loader `save_node`; do not rely on `property_<name>` keys for global-context classification.
- [x] 1.3 Normalize global-context property names by stripping `Глобальный контекст.` / `Global context.` prefixes and using Unicode-aware identifier keys.
- [x] 1.4 Add a converter that builds `GlobalContextIndex` only from `Global context/properties` provenance.
- [x] 1.5 Extract property-level metadata collection item types from Syntax Helper property evidence where the property returns `КоллекцияОбъектовМетаданных`.
- [x] 1.6 Add unit tests using the real `Metadata974.html` fixture proving `Метаданные`/`Metadata` maps to `ОбъектМетаданныхКонфигурация`.
- [x] 1.7 Add a synthetic fixture/global-context property test proving a new property can be resolved without editing `analysis-v2`.

## 2. Runtime and Deps Wiring

- [x] 2.1 Introduce a platform-docs semantic conversion bundle or equivalent coordinated cache payload containing raw types, global function signatures, and `GlobalContextIndex`.
- [x] 2.2 Add immutable global-context index storage to the shared/runtime boundary and `SemanticDeps`.
- [x] 2.3 Add `SemanticDeps` constructors/builders for empty, loaded, and degraded docs states, then migrate direct field literals where this change touches tests/support.
- [x] 2.4 Wire CLI, backend/LSP, Web, and bsl-agent deps construction to pass the same index.
- [x] 2.5 Include global-context index identity in deps snapshot hashing/invalidation.
- [x] 2.6 Expose debug/status evidence showing whether global-context properties were loaded, absent, or degraded.

## 3. v2 Inference

- [x] 3.1 Resolve bare identifiers through the data-driven global-context index after local/module/context scopes and before undeclared-variable diagnostics.
- [x] 3.2 Preserve local shadowing of global-context properties.
- [x] 3.3 Resolve `Метаданные.<collection>` through `ОбъектМетаданныхКонфигурация` repository properties instead of hardcoded collection tables when Syntax Helper data is present.
- [x] 3.4 Attach instance-specific metadata collection item type notes from property-level evidence, not from the reusable `КоллекцияОбъектовМетаданных` type alone.
- [x] 3.5 Resolve metadata collection element names through collection item type metadata without treating object names as fixed platform properties.
- [x] 3.6 Remove or demote `GLOBAL_CONTEXT_PROPERTIES_INFO` and metadata-object source tables to a centralized degraded fallback or test fixture.
- [x] 3.7 Ensure loaded data-driven global-context entries win over legacy global collection tables; legacy lookup may run only as explicit degraded/bootstrap fallback.

## 4. Diagnostics and IDE Consumers

- [x] 4.1 Ensure `UndeclaredVariable` is not emitted for loaded global-context properties.
- [x] 4.2 Ensure `NonExistentProperty` is not emitted for dynamic configuration object names inside metadata object collections.
- [x] 4.3 Ensure hover/type-at-position show `Метаданные` as `ОбъектМетаданныхКонфигурация` with Syntax Helper/global-context provenance.
- [x] 4.4 Ensure completion includes global-context properties in non-member global completion only when the index is available and context rules allow them.

## 5. Regression Coverage

- [x] 5.1 Add a regression for `examples/conf_big/AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl` lines 32 and 36.
- [x] 5.2 Add a local-shadowing regression for `Метаданные = ...`.
- [x] 5.3 Add a degraded/no-Syntax-Helper regression proving the analyzer does not invent the binding.
- [x] 5.4 Add a member-chain regression proving final `.Имя` resolves from platform property data to `Строка`.
- [x] 5.5 Add a regression proving `КоллекцияОбъектовМетаданных` can carry different item types for different source properties without global type mutation.

## 6. Validation

- [x] 6.1 Run `cargo fmt --all -- --check`.
- [x] 6.2 Run targeted runtime converter/parser tests for global-context properties.
- [x] 6.3 Run targeted `bsl-analysis-v2` inference tests.
- [x] 6.4 Run targeted `bsl-diagnostics` tests.
- [x] 6.5 Run CLI check for `examples/conf_big/AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl` and confirm 0 errors / 0 warnings for the `Метаданные` scenario. Final closure evidence: `validation/final-cli-diagnostics-closure.md`.
- [x] 6.6 Run `openspec validate add-syntax-helper-global-context-index --strict --no-interactive`.

## 7. Global Metadata Manager Collections

- [x] 7.1 Inventory every entry in `GLOBAL_COLLECTIONS_INFO` and map it to Syntax Helper/configuration metadata evidence when available.
- [x] 7.2 Migrate data-derivable entries to a data-driven index or document why they must remain a degraded/bootstrap fallback.
- [x] 7.3 Add regression coverage proving at least one global manager collection such as `РегистрыНакопления` still resolves after removing direct inference-table dependency.
- [x] 7.4 If any entries remain hardcoded, create a follow-up OpenSpec change or validation note listing the exact entries, reason, and owner.
- [x] 7.5 Add a precedence regression proving a loaded Syntax Helper/global-context manager property is not shadowed by the legacy table entry with the same name.
