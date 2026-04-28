# Change: Add Syntax Helper global-context index

## Why

The recent `Метаданные` fix proved that Syntax Helper already contains `Global context.Metadata` with type `ОбъектМетаданныхКонфигурация`, but v2 inference does not consume global-context properties as data. The temporary typed bridge fixes the observed false diagnostic, but it hardcodes platform facts in source and will drift from the installed Syntax Helper.

Global context properties such as `Метаданные` must be loaded from the platform documentation the same way platform types and global functions are loaded.

## What Changes

- Parse and index `Global context/properties/*.html` from Syntax Helper as first-class global-context properties.
- Preserve global-context property provenance and bilingual short names during Syntax Helper loading; do not infer global-context membership from short property names alone.
- Wire the global-context property index into `SemanticDeps`/v2 deps snapshots so CLI, LSP, Web, and bsl-agent use the same immutable source.
- Resolve bare identifiers such as `Метаданные` through the data-driven global-context index after local/module scopes and before undeclared-variable diagnostics.
- Derive metadata object collection chains from loaded platform type/property data, including property-level collection item types, instead of hardcoded source tables wherever Syntax Helper data is available.
- Inventory existing hardcoded global metadata manager collections (`GLOBAL_COLLECTIONS_INFO`) and either migrate them to Syntax Helper/config-driven data in this change or leave an explicit follow-up with evidence for any remaining hardcoded entries.
- Ensure loaded Syntax Helper/global-context entries take precedence over legacy hardcoded global collection tables; hardcodes may remain only as centralized degraded/bootstrap fallback.
- Keep degraded behavior fail-closed when Syntax Helper or a specific global-context property is absent.
- Remove or demote the current hardcoded `Метаданные` bridge to a test-only/degraded fallback once the index is wired.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `bsl-runtime/src/data/loaders/syntax_helper/*`
  - `bsl-runtime/src/data/adapters/converters.rs`
  - `bsl-runtime/src/system/deps_bundle_v2.rs`
  - `bsl-runtime/src/system/system_coordinator/lifecycle.rs`
  - `analysis-v2/src/lib.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/ast_to_ir/global_collections.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - CLI/backend/LSP setup that builds `SemanticDeps`
  - regression tests for `examples/conf_big/AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl`
