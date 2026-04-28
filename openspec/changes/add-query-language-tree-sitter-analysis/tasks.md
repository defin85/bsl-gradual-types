## 1. Parser and Syntax Layer

- [ ] 1.1 Add a separate vendored tree-sitter grammar for the 1C query language.
- [ ] 1.2 Add grammar corpus tests for SELECT lists, aliases, joins, temp tables, packages, unions, aggregates, parameters, `ВЫБОР`, `ЗНАЧЕНИЕ`, `ИТОГИ`, `УПОРЯДОЧИТЬ`, and syntax recovery.
- [ ] 1.3 Generate and commit parser artifacts, grammar JSON, and Rust bindings.
- [ ] 1.4 Add `bsl-query-syntax` crate with parse result, AST facade, syntax diagnostics, and embedded source-map types.
- [ ] 1.5 Add tests for Russian and English query keywords where the platform supports both forms.

## 2. Embedded Query Extraction

- [ ] 2.1 Fix/extend multiline BSL string extraction so query text keeps every string content segment and maps query offsets back to BSL byte ranges.
- [ ] 2.2 Extract query candidates from `Запрос.Текст = ...` and `Новый Запрос(...)`.
- [ ] 2.3 Support bounded static string concatenation without treating dynamic query text as known.
- [ ] 2.4 Add fixture tests for real multiline query strings from `examples/conf_big`.

## 3. Query Schema and Field Validation

- [ ] 3.1 Implement package-order schema derivation for select statements and `ПОМЕСТИТЬ` temporary tables.
- [ ] 3.2 Resolve output field names from aliases, direct field refs, known `*` expansions, and safe aggregate/function aliases.
- [ ] 3.3 Resolve source aliases and field references against known metadata, temporary tables, and virtual table schemas.
- [ ] 3.4 Emit query-text diagnostics for missing known fields, unknown aliases, ambiguous unqualified fields, and duplicate output aliases.
- [ ] 3.5 Keep unknown/dynamic/unparsable query parts fail-closed without high-confidence missing-field diagnostics.

## 4. v2 Semantic Integration

- [ ] 4.1 Add query-aware bindings for query objects, query results, query selections, and query-produced value tables.
- [ ] 4.2 Attach `QuerySchema` to `Запрос.Выполнить()`, `РезультатЗапроса.Выбрать()`, and `РезультатЗапроса.Выгрузить()` results.
- [ ] 4.3 Materialize query result fields as instance-specific structural members.
- [ ] 4.4 Update hover/completion/type-at-position to show query-derived fields without globally widening platform types.
- [ ] 4.5 Add bounded same-file parameter shape propagation for local procedures receiving query selections.

## 5. Diagnostics and Acceptance Tests

- [ ] 5.1 Add regression test proving `ВыборкаПоДокументам.Период` and analogous fields in `КнигаУчетаДоходовПатент/Ext/ManagerModule.bsl` are accepted when sourced from the final query result schema.
- [ ] 5.2 Add regression test proving `Выборка.НесуществующееПоле` is diagnosed when the selection schema is known.
- [ ] 5.3 Add regression test proving a typo inside a query field reference is diagnosed when the source schema is known.
- [ ] 5.4 Add regression test proving dynamic query text degrades without invented fields.
- [ ] 5.5 Verify CLI, LSP diagnostics, hover, completion, and semantic-diagnostics consumers use the same v2 query facts.

## 6. Validation

- [ ] 6.1 Run tree-sitter grammar tests for the query grammar.
- [ ] 6.2 Run `cargo fmt --all -- --check`.
- [ ] 6.3 Run targeted `cargo test` suites for `bsl-query-syntax`, `bsl-syntax`, `bsl-analysis-v2`, `bsl-diagnostics`, and `bsl-runtime`.
- [ ] 6.4 Run `cargo clippy --all-targets -- -D warnings` or document any repo-existing blockers.
- [ ] 6.5 Run `openspec validate add-query-language-tree-sitter-analysis --strict --no-interactive`.
