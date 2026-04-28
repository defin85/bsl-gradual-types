# Architecture Audit: Syntax Helper Global Context Index

## Audit Verdict

Conditionally sound after wording fixes.

The core direction is correct: `Метаданные` and other global-context properties must come from Syntax Helper data, not source-level inference tables. The main delivery risk was under-specified provenance and item-type extraction. The design now requires first-class source provenance for `Global context/properties` and property-level item type annotations for metadata object collections.

## Locked Decisions

- Syntax Helper global-context properties are the source of truth.
- `analysis-v2` receives an immutable index through `SemanticDeps` and performs no Syntax Helper file IO.
- Local/module symbols shadow global-context properties.
- Metadata object chains use `TypeRepository` property data before any degraded fallback.
- Missing Syntax Helper data fails closed.
- Existing global metadata manager collection hardcodes must be inventoried and either migrated or explicitly left as fallback/follow-up.

## Audit Matrix

| Area | Verdict | Evidence / Follow-up |
| --- | --- | --- |
| Requirement coverage | Pass with patch | Added provenance and instance-specific collection item-type requirements. |
| Runtime architecture | Pass with patch | Added coordinated platform-docs semantic bundle to avoid raw-types/global-functions/global-context cache drift. |
| Data provenance | Fixed gap | Current loader loses property file path when storing `property_<name>`; tasks now require preserving source path/provenance. |
| Metadata collection semantics | Fixed gap | `КоллекцияОбъектовМетаданных` is reusable; item type must live on the collection value from the source property. |
| Performance | Pass | Index is small, O(1), built once with platform docs conversion. |
| Reliability | Pass with gate | Degraded/absent/docs-loaded states must be distinguishable in index availability. |
| Compatibility | Watch | Adding `SemanticDeps` fields will touch many test literals; builder tasks reduce churn. |
| Operability | Pass with patch | Added debug/status task for loaded/absent/degraded evidence. |
| Test strategy | Pass with patch | Added synthetic no-analysis-v2-edit test and distinct item-type regression. |
| Remaining hardcodes | Watch | `GLOBAL_COLLECTIONS_INFO` is now an explicit inventory/migration task instead of hidden scope drift. |

## Execution Plan

1. Extend Syntax Helper property parsing with provenance and bilingual short names.
2. Build a coordinated platform-docs semantic bundle containing raw types, global functions, and global-context index.
3. Add `SemanticDeps` builder/default APIs and wire runtime/CLI/LSP/bsl-agent deps.
4. Replace `lookup_global_context_property` and metadata-object tables with index-backed lookup and centralized degraded fallback.
5. Inventory `GLOBAL_COLLECTIONS_INFO`; migrate data-derivable entries or document fallback/follow-up entries.
6. Add parser/converter, inference, diagnostics, hover/type-at-position, completion, and CLI regressions.
7. Run strict OpenSpec and targeted cargo/CLI validation from `tasks.md`.

## Exact Wording Fixes Applied

- Required source provenance for `Global context/properties`; short property names alone are insufficient.
- Required stripping `Глобальный контекст.` / `Global context.` prefixes and Unicode-aware identifier normalization.
- Required property-level metadata collection item types; the reusable `КоллекцияОбъектовМетаданных` type must not be globally mutated.
- Required a coordinated platform-docs semantic bundle/cache identity instead of independent raw-type/global-function/global-context conversion paths.
- Required `SemanticDeps` constructors/builders to limit broad test churn and make degraded docs states explicit.
- Added explicit inventory/migration requirement for `GLOBAL_COLLECTIONS_INFO`.

## Assumptions and Open Questions

- Assumption: global-context property availability should follow loaded Syntax Helper docs, not a fixed platform-version table.
- Assumption: property-level item type can be extracted from the property page body or linked property evidence, as shown by `ConfigurationMetadataObject/properties/AccumulationRegisters6284.html`.
- Open question: if a platform version omits item-type prose for a metadata collection property, implementation should decide whether to use a centralized degraded fallback or leave that chain weak/unknown.

## External Evidence

- 1C Developer Guide: `Metadata` global context property gives access to `ConfigurationMetadataObject`.
- 1C training material: Global Context exposes properties and methods available from source code and lists metadata object managers in Syntax Assistant.
