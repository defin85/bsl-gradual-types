## ADDED Requirements

### Requirement: v2 SHALL parse embedded 1C query text with a dedicated query-language tree-sitter grammar

The system SHALL provide a dedicated tree-sitter grammar for the 1C query language and SHALL use it as the parser for static query texts embedded in BSL strings.

The query parser SHALL be separate from the BSL tree-sitter grammar. BSL parsing SHALL remain responsible for BSL syntax and embedded string extraction only.

The parser integration SHALL preserve a source map from normalized query text byte ranges back to the original BSL source byte ranges so diagnostics and IDE features can point at the user's `.bsl` file.

#### Scenario: Multiline query text is parsed as an embedded query

- **GIVEN** a BSL module assigns a multiline string to `Запрос.Текст`
- **WHEN** v2 semantic analysis processes the current revision
- **THEN** the embedded query text is normalized and parsed by the dedicated query-language parser
- **AND** query parser diagnostics, if any, map back to ranges in the original BSL string literal

#### Scenario: BSL parser does not own query grammar recovery

- **GIVEN** embedded query text contains a query-language syntax error
- **WHEN** the BSL file is parsed
- **THEN** the BSL parse remains governed by the BSL grammar
- **AND** the query-language syntax error is reported by the query parser integration, not by broadening BSL grammar recovery

### Requirement: v2 SHALL derive query result schemas from static query texts

The system SHALL derive a `QuerySchema` for statically known query texts. The schema SHALL include result field names, field source spans, temporary table schemas created by `ПОМЕСТИТЬ`, and confidence/type information when available.

The schema derivation SHALL process query packages in statement order so temporary tables created by earlier statements can be used by later statements.

When query text is dynamic, unparsable, or depends on unknown sources, the system SHALL fail closed by marking the schema unknown instead of inventing fields.

#### Scenario: Final SELECT aliases become result fields

- **GIVEN** a static query text whose final result statement selects `Источник.Период КАК Период`
- **WHEN** v2 derives the query result schema
- **THEN** the schema contains result field `Период`
- **AND** the field records the source span of the select item

#### Scenario: Temporary table schema feeds subsequent statements

- **GIVEN** a query package creates `ПОМЕСТИТЬ ВТ_Данные` from a select list
- **AND** a later select statement reads fields from `ВТ_Данные`
- **WHEN** v2 derives the query package schema
- **THEN** the later statement can resolve fields exposed by `ВТ_Данные`

#### Scenario: Dynamic query text does not invent result fields

- **GIVEN** `Запрос.Текст` is built from a non-static expression
- **WHEN** v2 analysis reaches `РезультатЗапроса.Выбрать()`
- **THEN** the selection keeps the platform type without query-derived structural fields
- **AND** the system does not emit high-confidence query-field diagnostics based on guessed schema

### Requirement: v2 SHALL propagate query schemas through BSL query result APIs

The system SHALL attach query-derived schemas to query objects and propagate them through platform query result APIs:

- `Запрос.Выполнить()` SHALL return `РезультатЗапроса` with the derived query schema when available.
- `РезультатЗапроса.Выбрать()` SHALL return `ВыборкаИзРезультатаЗапроса` with instance-specific structural members for the schema fields.
- `РезультатЗапроса.Выгрузить()` SHALL return `ТаблицаЗначений` with query-derived columns when available.

The system SHALL NOT add query result fields globally to all values of platform type `ВыборкаИзРезультатаЗапроса`.

#### Scenario: Selection row fields are available to BSL member access

- **GIVEN** a static query result schema contains fields `Период` and `Регистратор`
- **AND** `Выборка = РезультатЗапроса.Выбрать()` is produced from that query result
- **WHEN** diagnostics validate `Выборка.Период` and `Выборка.Регистратор`
- **THEN** no missing-property diagnostic is emitted for those fields
- **AND** hover/completion can surface those query-derived fields for that selection instance

#### Scenario: Query fields do not leak between selections

- **GIVEN** two different query results produce different field schemas
- **WHEN** each result is converted to a selection
- **THEN** each selection exposes only its own query-derived fields
- **AND** no field from one query result is treated as present on the other selection

### Requirement: v2 SHALL diagnose missing query fields when source schemas are known

The system SHALL validate field references inside static query text when the referenced source schema is known from metadata, virtual-table models, or temporary table schemas.

The system SHALL emit diagnostics for:

- unknown source aliases when no matching source is declared;
- missing fields on known sources;
- ambiguous unqualified field references;
- duplicate output aliases that make result schema ambiguous.

The system SHALL NOT emit high-confidence missing-field diagnostics when the relevant source schema is unknown.

#### Scenario: Typo in a known query source field is diagnosed

- **GIVEN** a static query selects `Контрагенты.НесуществующееПоле`
- **AND** the schema for source alias `Контрагенты` is known and does not contain `НесуществующееПоле`
- **WHEN** v2 query diagnostics run
- **THEN** the system emits a query-field diagnostic mapped to the field reference inside the BSL query string

#### Scenario: Unknown source schema suppresses high-confidence field errors

- **GIVEN** a static query references `ВнешнийИсточник.Поле`
- **AND** v2 cannot determine the schema for `ВнешнийИсточник`
- **WHEN** v2 query diagnostics run
- **THEN** the system does not emit a high-confidence missing-field diagnostic for `Поле`
- **AND** the query-dependent type facts are marked as unknown or weak rather than known

### Requirement: v2 SHALL diagnose BSL access to fields absent from a known query result schema

When BSL code accesses a member on a query selection or query-produced value table row and the query result schema is known, the system SHALL validate the member name against that schema.

If the member is absent from the known schema, the system SHALL emit a missing query-result-field diagnostic. If the schema is unknown, dynamic, or parse-failed, the system SHALL use existing platform-type behavior and SHALL NOT invent a high-confidence query-result-field error.

#### Scenario: Missing BSL selection field is diagnosed against known query schema

- **GIVEN** a static query result schema contains `Период`
- **AND** does not contain `НесуществующееПоле`
- **WHEN** BSL diagnostics validate `Выборка.НесуществующееПоле` for a selection produced from that query result
- **THEN** the system emits a missing query-result-field diagnostic
- **AND** the diagnostic explains that the field is absent from the known query result schema

#### Scenario: Known query schema accepts fields from real conf_big module

- **GIVEN** `КнигаУчетаДоходовПатент/Ext/ManagerModule.bsl` contains a query whose final select list declares `Период`, `Регистратор`, `Покупатель`, `ДоговорСПокупателем`, `ДругойКонтрагент`, `ДоговорДругогоКонтрагента`, and `КоличествоДоговоров`
- **WHEN** v2 diagnostics analyze accesses on `ВыборкаПоДокументам`
- **THEN** no missing-property diagnostic is emitted for those declared query result fields
