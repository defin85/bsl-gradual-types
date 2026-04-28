## Context

The motivating real case is `examples/conf_big/AccumulationRegisters/КнигаУчетаДоходовПатент/Ext/ManagerModule.bsl`: the final query statement selects `Период`, `Регистратор`, `Покупатель`, `ДоговорСПокупателем`, `ДругойКонтрагент`, `ДоговорДругогоКонтрагента`, and `КоличествоДоговоров`, then BSL reads those fields from `ВыборкаПоДокументам`.

The current analyzer only carries the platform type `ВыборкаИзРезультатаЗапроса`, so property validation reports valid query-result fields as missing. Existing `analysis-v2` already has an instance-specific structural-member mechanism for structures and value tables; query result rows should use the same pattern instead of globally modifying the platform type.

Primary platform/source references:
- 1C query selection fields define the result field list and aliases: https://kb.1ci.com/1C_Enterprise_Platform/Guides/Developer_Guides/1C_Enterprise_8.3.23_Developer_Guide/Chapter_8._Working_with_queries/8.4._Query_language/8.4.8._Selection_field_description/
- 1C language query execution APIs expose `QueryResult`, selection, and unload flows: https://kb.1ci.com/1C_Enterprise_Platform/Guides/Developer_Guides/1C_Enterprise_8.3.23_Developer_Guide/Chapter_8._Working_with_queries/8.5._Execution_and_working_with_queries_in_the_1C_Enterprise_language/8.5.1._Working_with_queries/
- Tree-sitter grammars are authored as separate parser grammars and generated artifacts: https://tree-sitter.github.io/tree-sitter/creating-parsers/1-getting-started.html

## Goals / Non-Goals

- Goals:
  - Treat the 1C query language as a first-class embedded language with its own tree-sitter grammar.
  - Derive deterministic query result schemas from static query texts.
  - Propagate query result schemas through `Запрос.Выполнить()`, `РезультатЗапроса.Выбрать()`, and `РезультатЗапроса.Выгрузить()`.
  - Validate field references inside queries when sources are known.
  - Validate BSL member access against known query result schemas.
  - Preserve v2's canonical snapshot/revision discipline and fail-closed behavior.
- Non-Goals:
  - Do not add a global open-ended property bag to `ВыборкаИзРезультатаЗапроса`.
  - Do not require network access or live 1C runtime data.
  - Do not make query analysis mandatory for dynamic string construction in the first delivery.
  - Do not block all BSL diagnostics when a query parse fails; degrade only the query-dependent facts.

## Architecture

### Parser boundary

Add a separate vendored grammar under `third_party/tree-sitter-1c-query` or `third_party/tree-sitter-bsl-query`. The BSL grammar remains responsible for BSL source only. The query parser consumes normalized embedded query text extracted from BSL string literals.

This separation keeps:
- BSL parser recovery independent from query grammar changes.
- query grammar versioning/test corpus independent from BSL language corpus.
- future LSP query-text highlighting and diagnostics possible through embedded source maps.

### Query syntax crate

Add a workspace crate `bsl-query-syntax` that wraps the generated tree-sitter parser and exposes a stable domain API:

- `QueryParseResult`
  - tree-sitter tree
  - recoverable syntax diagnostics
  - normalized query text
  - `EmbeddedTextMap` from query byte ranges back to BSL byte ranges
- `QueryPackage`
  - ordered statements split by semicolon
  - `SelectStatement`
  - `SourceRef`
  - `Join`
  - `SelectItem`
  - `QueryExpr`
- `QuerySchema`
  - output fields for final result statements
  - temporary table schemas created by `ПОМЕСТИТЬ`
  - query semantic diagnostics

The crate must not depend on `analysis-v2`. It may depend on `bsl-shared` for spans and diagnostic-compatible data types if that keeps boundaries clean.

### BSL embedded text extraction

Extend `syntax/` or `analysis-v2` with a reusable embedded-text extractor for query candidates:

- `Запрос.Текст = <static string expression>`
- `Новый Запрос(<static string expression>)`
- future: bounded literal concatenation and `Запрос.Текст = Запрос.Текст + "..."`

The extractor must normalize multiline BSL strings and keep offset mapping. It should support escaped quotes and `|` multiline prefixes. If expression text is not statically known, it returns `Dynamic`.

### Query schema resolution

The query semantic pass resolves schemas in package order:

1. Build source schemas from known metadata/config sources, virtual tables, temporary tables, and aliases.
2. Resolve each select item's output name:
   - explicit `КАК Alias` wins;
   - `Source.Field` without alias outputs `Field`;
   - expression/function without alias is schema-unknown unless the grammar provides an unambiguous platform field name;
   - `*` is expanded only when the source schema is known.
3. Register `ПОМЕСТИТЬ TempTable` schemas for subsequent statements.
4. The final non-temporary result statement becomes the `QuerySchema` attached to the BSL query result.

Field types are layered:
- Phase 1: field existence with `Unknown`/weak type is enough to suppress false `NonExistentProperty`.
- Phase 2: infer obvious scalar and metadata types for aliases, aggregate functions, `ЗНАЧЕНИЕ(...)`, `ВЫБОР`, and direct metadata fields.
- Phase 3: propagate nullability and union types from joins and conditional expressions.

### v2 semantic integration

Extend `analysis-v2` with query-aware bindings:

- `QueryObjectBinding`
  - static text hash
  - parse status
  - `QuerySchema`
- `QueryResultBinding`
  - `QuerySchema`
- `QuerySelectionBinding`
  - row structural members from `QuerySchema`
- `QueryValueTableBinding`
  - value table columns from `QuerySchema`

The binding should reuse the existing instance-specific structural-member materialization path where possible. A `ВыборкаИзРезультатаЗапроса` value produced by one query gets only that query's fields; another selection does not inherit them.

Method handling:
- `Запрос.Выполнить()` returns platform type `РезультатЗапроса` plus `QueryResultBinding`.
- `РезультатЗапроса.Выбрать()` returns platform type `ВыборкаИзРезультатаЗапроса` plus row structural members.
- `РезультатЗапроса.Выгрузить()` returns platform type `ТаблицаЗначений` plus columns.
- generic platform method resolution remains the fallback when no query binding exists.

### Diagnostics

Add two related diagnostic families:

1. Query-text diagnostics:
   - unknown source alias when no matching `ИЗ`/temporary table/source exists;
   - missing field when the referenced source schema is known;
   - ambiguous unqualified field when more than one known source provides it;
   - duplicate output aliases when they make row schema ambiguous.

2. BSL query-result diagnostics:
   - BSL member access on a known query selection/value-table row that is not present in the query result schema;
   - no high-confidence diagnostic when the query schema is dynamic, unknown, or parse-failed.

For the motivating case, `ВыборкаПоДокументам.Период` is accepted because `Период` is present in the final query result schema. `ВыборкаПоДокументам.НесуществующееПоле` should produce a high-confidence diagnostic when the same schema is known.

### Caching and performance

Query parsing must be cached by:
- normalized query text hash;
- relevant metadata/config snapshot id;
- parser grammar version;
- query analysis settings.

Only query-candidate strings should be parsed. Random string literals must not pay query-parser cost. Query parse and schema derivation must be bounded and cancellable through the same v2 revision/cancellation discipline as other semantic artifacts.

### Interprocedural shape propagation

Direct same-scope selection access is the first acceptance gate. The next gate is local procedure propagation:

- when a local call passes a query selection argument, the callee parameter should receive the same structural members during bounded same-file analysis;
- recursion, cross-file calls, or large bodies may fail closed;
- caching should key by callee span and compact structural signature to avoid unbounded specialization.

This is required to remove analogous false diagnostics inside helpers like `ЗаполнитьДоговорКонтрагентаПоДокументу(ВыборкаПоДокументам)`.

## Alternatives Considered

- Embed query grammar into `tree-sitter-bsl`: rejected because it couples BSL parser recovery and query parser evolution, and query text lives inside strings with separate offset mapping concerns.
- Add permissive dynamic members to all `ВыборкаИзРезультатаЗапроса`: rejected because it hides real missing-field errors and makes diagnostics less useful.
- Keep a simple alias extractor only: rejected for this change because the requested direction is full query-language coverage and query-side field validation.

## Open Questions

- Which platform query dialect/version is the initial grammar baseline: 8.3.23 docs, latest installed platform docs, or repo-configured version?
- Should query diagnostics be enabled by default immediately, or gated behind a setting until the grammar corpus reaches an agreed coverage threshold?
- How aggressively should virtual table schemas be modeled in phase 1 versus relying on config metadata and weak unknowns?
