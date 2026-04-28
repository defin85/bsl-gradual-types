# Final CLI Diagnostics Closure

Date: 2026-04-28
Change: `add-syntax-helper-global-context-index`

## Gap Closed

The implementation review found that OpenSpec task 6.5 required `0 errors / 0
warnings`, while the live CLI check for
`examples/conf_big/AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl`
still produced two `UnknownTypeAccess` warnings for final `.Имя` accesses in
valid `Метаданные.<collection>.<object>.Измерения.<field>.Имя` chains.

## Fix Evidence

Nested metadata collection properties such as `Измерения`, `Реквизиты`, and
`Ресурсы` now attach the existing degraded/bootstrap item-type fallback when
Syntax Helper exposes the property as `КоллекцияОбъектовМетаданных` but does
not provide property-level `collection_item_type` evidence.

This keeps generic unknown-type diagnostics intact and only fills the missing
metadata collection item type for known metadata object field collections.

## Verification

- `cargo test -p bsl-analysis-v2 nested_metadata_collection_item_type_falls_back_when_source_property_has_no_item_type`
- `cargo test -p bsl-analysis-v2 semantic_diagnostics_allow_nested_metadata_collection_names_without_source_item_type`
- `cargo test -p bsl-analysis-v2 resolves_conf_big_metadata_manager_module_lines_32_and_36`
- `cargo test -p bsl-diagnostics test_unknown_type_access_is_warning_by_default`
- `cargo test -p bsl-diagnostics test_dynamic_like_skips_nonexistent_property_validation`
- `cargo run -p bsl-cli -- --format plain check --strict examples/conf_big/AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl`

Strict CLI result:

- Errors: 0
- Warnings: 0
- Exit code: 0
