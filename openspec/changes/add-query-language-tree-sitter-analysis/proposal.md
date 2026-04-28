# Change: Add first-class query-language tree-sitter analysis

## Why

Real BSL modules build typed data flows through `Запрос.Выполнить()` and `РезультатЗапроса.Выбрать()`, but v2 currently treats `ВыборкаИзРезультатаЗапроса` as only a platform container type. This causes false diagnostics for valid row fields such as `ВыборкаПоДокументам.Период` when the field is declared by the query result.

The same missing embedded-query model also prevents diagnostics from checking query-side field references against known metadata and temporary table schemas.

## What Changes

- Add a dedicated tree-sitter grammar for the 1C query language as a separate vendored parser, not as part of the BSL grammar.
- Add a query syntax crate that exposes query AST, syntax diagnostics, source mapping back into BSL string literals, and recoverable parse trees.
- Extend v2 analysis to extract static query texts from BSL, parse them, derive query result schemas, and attach those schemas to `Запрос`, `РезультатЗапроса`, `ВыборкаИзРезультатаЗапроса`, and `ТаблицаЗначений` instances.
- Add query semantic diagnostics for known-bad field references inside query text and for BSL member accesses that are not present in the known query result schema.
- Keep behavior fail-closed for dynamic or unparsable query texts: do not invent fields and do not emit high-confidence query diagnostics when sources are unknown.

## Impact

- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `third_party/tree-sitter-*`
  - `syntax/`
  - new `bsl-query-syntax` crate
  - `analysis-v2/`
  - `semantic-diagnostics/`
  - `shared/`
  - CLI/LSP/Web consumers that surface semantic diagnostics and hover/completion details
