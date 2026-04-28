## Context

The concrete regression is in:

`examples/conf_big/AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl`

The code uses:

```bsl
Метаданные.РегистрыНакопления.АвансовыеПлатежиИностранцевПоНДФЛ.Измерения.ГоловнаяОрганизация.Имя
```

Local Syntax Helper evidence exists under:

`examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Metadata974.html`

That page describes `Глобальный контекст.Метаданные (Global context.Metadata)` and gives the type `ОбъектМетаданныхКонфигурация`. The parser already has a generic `PropertyInfo` model, and `convert_syntax_helper_global_functions` already exposes global functions, but there is no equivalent conversion/wiring for global-context properties.

The temporary fix adds a source-level binding for `Метаданные`. This change replaces that with a data-driven index.

Official platform evidence:

- 1C Developer Guide states that the `Metadata` global context property gives access to a `ConfigurationMetadataObject`: https://kb.1ci.com/1C_Enterprise_Platform/Guides/Developer_Guides/1C_Enterprise_8.3.23_Developer_Guide/Chapter_2._Managing_configurations/2.23._Operating_with_configurations_using_1C_Enterprise_language/
- 1C training material describes Global Context as providing properties and methods available from anywhere in source code, with metadata object managers listed in Syntax Assistant: https://kb.1ci.com/1C_Enterprise_Platform/Tutorials/1C_Enterprise_Platform_Training_Course_-_Junior_Application_Developer_Level/Module_3._1C_script_language_basics/Episode_1._Working_with_Metadata_Objects/

## Goals / Non-Goals

Goals:

- Make Syntax Helper global-context properties the source of truth for bare platform global properties.
- Keep local/module declarations shadowing global-context properties.
- Reuse the existing repository/type metadata model for property/member validation.
- Make `Метаданные` and equivalent English names resolve without hardcoding them in type inference.
- Preserve deterministic deps/settings invalidation when Syntax Helper content changes.
- Fail closed when Syntax Helper data is unavailable or incomplete.

Non-Goals:

- Do not introduce live 1C runtime calls.
- Do not invent global-context properties absent from the loaded Syntax Helper.
- Do not globally loosen `UndeclaredVariable` or `NonExistentProperty` diagnostics.
- Do not solve configuration object existence checks for every metadata name beyond the known metadata index contract.
- Do not change query-language analysis; that remains owned by `add-query-language-tree-sitter-analysis`.

## Data Model

Add a typed global-context property model near the platform documentation boundary, not inside inference:

```rust
pub struct GlobalContextPropertyData {
    pub name: String,
    pub english_name: Option<String>,
    pub prop_type: Option<String>,
    pub is_readonly: bool,
    pub description: Option<String>,
    pub contexts: Vec<String>,
    pub source_key: String,
    pub source_path_hash: String,
}

pub struct GlobalContextIndex {
    by_ru_key: HashMap<String, GlobalContextPropertyData>,
    by_en_key: HashMap<String, GlobalContextPropertyData>,
    source_hash: String,
    availability: GlobalContextAvailability,
}
```

Identifier keys use one shared normalization helper for BSL identifiers:

- trim whitespace;
- strip `Глобальный контекст.` / `Global context.` prefixes for global-context property titles;
- apply Unicode-aware case folding/lowercasing for both Cyrillic and Latin names.

Do not use ASCII-only comparison for this index.

The exact module may be adjusted during implementation, but the ownership rule is fixed:

- `bsl-runtime` parses and converts Syntax Helper data.
- `shared` may host serializable domain structs if they must cross crate boundaries.
- `analysis-v2` receives an immutable index in `SemanticDeps`.
- `analysis-v2` must not read Syntax Helper files directly.

## Syntax Helper Extraction

The loader must classify property pages under the `Global context/properties` subtree as global-context properties. The parser should preserve enough path/source identity to distinguish:

- ordinary type properties;
- global-context properties;
- unrelated property pages with the same short name.

The existing `PropertyInfo` currently stores name/type/readonly/description and the loader stores properties by synthetic `property_<name>` keys, which loses the original file path. That is not sufficient for this change.

Implementation must introduce first-class provenance through one of these shapes:

1. add a dedicated `GlobalContextPropertyInfo` / `SyntaxNode::GlobalContextProperty` plus a `SyntaxHelperDatabase.global_context_properties` collection; or
2. extend `PropertyInfo` with `english_name`, `contexts`, `source_path`, and `source_kind`, and preserve that data before `save_node`.

The converter must not use short-name lookup alone for global-context properties. It should explicitly require `Global context/properties` provenance.

Global-context property title normalization must strip the owner prefix from both languages. For example:

- `Глобальный контекст.Метаданные` -> `Метаданные`;
- `Global context.Metadata` -> `Metadata`.

The original qualified title remains available in source/debug metadata.

## Platform Docs Semantic Bundle

Current startup caches `convert_syntax_helper_to_raw(&database)` as platform raw types and separately derives global function signatures. This change should avoid creating another uncoordinated cache path.

Introduce a single platform-docs semantic conversion output:

```rust
pub struct PlatformDocsSemanticBundle {
    pub raw_types: Vec<RawTypeData>,
    pub global_function_signatures: Vec<MethodSignature>,
    pub global_context_index: GlobalContextIndex,
    pub schema_version: &'static str,
}
```

The exact struct location may vary, but raw platform types, global functions, and global-context properties must be built from the same parsed Syntax Helper database and cache identity. Disk cache payloads may store this bundle or store compatible sub-parts, but the deps snapshot must be able to prove which global-context index was used.

## SemanticDeps Wiring

Extend `SemanticDeps` with a global-context index:

```rust
pub global_context: GlobalContextIndex,
```

or an equivalent optional/Arc field if the crate boundary requires it.

The index identity must participate in the deps snapshot hash, alongside repository/signature-index identity. At minimum it includes:

- Syntax Helper source root or recovered cache identity;
- parser/converter schema version;
- normalized property names and type strings;
- parse/degraded status.

When no Syntax Helper is loaded, the index is empty and marked unavailable. Consumers must be able to distinguish "empty because docs are absent" from "docs loaded and property absent" for diagnostics/debug output.

Use constructors/builders for `SemanticDeps` in test/support code rather than manually repeating field literals everywhere. This prevents new deps fields from becoming a broad mechanical churn source and keeps empty/degraded global-context behavior explicit.

## Inference Algorithm

Bare identifier resolution order remains conservative:

1. local variables, parameters, module variables, and explicit declarations;
2. context-specific implicit symbols;
3. data-driven global-context properties, including Syntax Helper global metadata manager properties when present;
4. configuration/common-module symbols and centralized legacy/degraded global metadata collection fallback for entries not available from the docs/config index;
5. owner-member fallback where applicable;
6. undeclared variable.

Legacy `GLOBAL_COLLECTIONS_INFO`, `GLOBAL_CONTEXT_PROPERTIES_INFO`, and
`METADATA_OBJECT_COLLECTIONS_INFO` lookups must never override a loaded
global-context index entry. If the same property exists in Syntax Helper or in a
configuration-backed index, the index-backed result wins and carries provenance;
legacy tables may run only when the data-driven entry is absent/unavailable and
the fallback was explicitly inventoried.

For a global-context property:

- create `TypeResolution` from `prop_type` using the same type-name normalization as platform properties;
- include certainty/source metadata identifying `SyntaxHelperGlobalContext`;
- keep the user-facing label as the platform type, not as an ad-hoc synthetic type.
- attach an uncertainty/source note when the index is unavailable so hover/diagnostics can explain degraded documentation state where supported.

Local shadowing must win:

```bsl
Метаданные = "x";
```

After this assignment, `Метаданные` has the local inferred type, not the global-context property type.

## Metadata Object Chains

For `Метаданные.<collection>` and nested metadata object access, the preferred data path is:

1. `Метаданные` resolves from `GlobalContextIndex` to `ОбъектМетаданныхКонфигурация`.
2. `ОбъектМетаданныхКонфигурация` is looked up in `TypeRepository`.
3. Its property `РегистрыНакопления` is resolved from `RawTypeData.properties`.
4. The property-level metadata collection item type is read from Syntax Helper evidence for that property, for example `РегистрыНакопления` -> `ОбъектМетаданных: РегистрНакопления`.
5. The resulting collection value keeps the platform type `КоллекцияОбъектовМетаданных` plus an instance-specific item type annotation derived from the property, not from the reusable collection type alone.
6. Dynamic element names such as `АвансовыеПлатежиИностранцевПоНДФЛ` resolve to the annotated item type without treating the element name as a fixed platform property.
7. Nested properties such as `Измерения` and `Имя` are resolved from the item type's platform properties.

This distinction is required because `КоллекцияОбъектовМетаданных` is reused for many metadata collections and cannot globally mean only one item type.

If Syntax Helper lacks property-level item-type evidence for a metadata collection, the implementation may keep a narrowly scoped degraded fallback, but it must be behind a documented adapter/API. New platform facts must not be added as scattered string checks in `type_inference_v2`.

## Diagnostics Behavior

The diagnostics layer should consume the same inferred type facts:

- no `UndeclaredVariable` for `Метаданные` when Syntax Helper global-context index contains it;
- no `NonExistentProperty` for configuration object names inside `КоллекцияОбъектовМетаданных` when the object name is treated as a dynamic collection element;
- keep `NonExistentProperty` for fixed platform types when the property is absent and no dynamic metadata-collection rule applies.

When Syntax Helper is missing, diagnostics fail closed:

- do not claim the identifier is `Неопределено`;
- do not invent the platform type;
- if an undeclared-variable diagnostic is emitted, attach uncertainty/configuration detail only if the existing diagnostic model supports it.

## Interaction With Active Changes

- `add-bsp-common-module-factory-resolution` may use `Метаданные.ОбщиеМодули` as source evidence inside BSP helper bodies. This change supplies the platform-global `Метаданные` foundation; BSP factory call-site rules remain separate.
- `add-query-language-tree-sitter-analysis` may use metadata schemas when resolving query sources. This change only improves the platform metadata object access path and does not parse query text.

## Global Metadata Manager Collections

The current code also has a separate hardcoded table for global metadata manager collections such as `Справочники`, `Документы`, and `РегистрыНакопления`.

This change must not silently leave that table as an unreviewed peer source of platform truth. Implementation must inventory `GLOBAL_COLLECTIONS_INFO` and choose one of two outcomes:

1. migrate entries that can be derived from Syntax Helper/configuration metadata into the same data-driven platform/config index family; or
2. keep specific entries only behind a documented degraded/runtime bootstrap fallback, with a follow-up OpenSpec change for any entries that cannot yet be data-driven.

When a loaded Syntax Helper/global-context entry and a legacy table entry share
the same global property name, the loaded entry is authoritative. The legacy
entry is not a peer source and must not be consulted first.

The acceptance evidence must state which entries were migrated, which remain fallback-only, and why.

## Migration From Temporary Bridge

Implementation should first introduce the data-driven path and make the current `GLOBAL_CONTEXT_PROPERTIES_INFO` behavior redundant under loaded Syntax Helper. After regression coverage passes through the data-driven path, remove the source-level table or restrict it to explicit test/degraded fixtures.

Acceptance should include a negative check that adding a new fixture global-context property does not require editing `analysis-v2/src/type_inference_v2.rs`.

## Performance

The global-context index is small and should be built once during Syntax Helper conversion. Per-identifier lookup must be O(1) by normalized name.

The index must be immutable inside `SemanticDeps`, cheap to clone through `Arc`, and included in deps snapshot hashing without serializing large HTML/source payloads on hot paths.

## Risks

- Existing `PropertyInfo` may not preserve English names or contexts for property pages; implementation may need a small parser extension.
- Some Syntax Helper collection types may not expose `collection_item_type` cleanly. The fallback must be centralized and documented rather than spread through inference.
- Single-file CLI checks must load enough Syntax Helper data for this feature. If they do not, the implementation must report degraded docs availability clearly during debugging.
