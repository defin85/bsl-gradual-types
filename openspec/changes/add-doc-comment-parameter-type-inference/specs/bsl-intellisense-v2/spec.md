## ADDED Requirements

### Requirement: v2 SHALL extract routine parameter type hints from standard leading doc comments

The system SHALL recognize adjacent standard BSL documentation comments before procedure/function declarations and extract `Параметры:` / `Parameters:` entries as structured routine parameter type hints.

The extraction SHALL support one or more type lines per parameter, including continuation lines where the parameter name is omitted and only an additional type entry is present. The extraction SHALL only bind entries whose parameter name matches a parameter in the actual routine declaration.

The syntax and IR layers SHALL preserve the extracted raw type names, source range, and source kind, but SHALL NOT resolve the type names or perform type inference.

#### Scenario: Multi-line parameter type list is extracted

- **GIVEN** a routine has an adjacent leading comment with `Параметры:`
- **AND** parameter `ТекущийОбъект` is documented with several continuation lines such as `- СправочникОбъект`, `- ДокументОбъект`, and `- РегистрСведенийНаборЗаписей`
- **WHEN** v2 builds syntax/IR for `Процедура ПриЧтенииНаСервере(Форма, ТекущийОбъект) Экспорт`
- **THEN** the IR parameter `ТекущийОбъект` contains doc-derived raw type names for all recognized type lines
- **AND** the IR parameter `Форма` contains the doc-derived raw type name `ФормаКлиентскогоПриложения`
- **AND** no type resolution is performed by the syntax layer itself

#### Scenario: Non-adjacent stale comments are not bound to a routine

- **GIVEN** a `Параметры:` comment block is separated from a routine declaration by a non-comment, non-directive statement
- **WHEN** v2 builds syntax/IR for that routine
- **THEN** the separated comment block is not associated with the routine parameters
- **AND** the routine parameters do not receive doc-derived hints from the stale block

#### Scenario: Mismatched parameter names are ignored

- **GIVEN** a leading routine comment documents parameter `СтароеИмя`
- **AND** the actual routine declaration contains parameter `НовоеИмя`
- **WHEN** v2 extracts doc-comment parameter hints
- **THEN** `СтароеИмя` is not attached to `НовоеИмя`
- **AND** analysis continues without parse or lowering failure

### Requirement: v2 SHALL resolve doc-derived parameter hints as scoped parameter type evidence

The system SHALL resolve doc-derived routine parameter type names in `analysis-v2` and SHALL seed the corresponding routine body parameter symbols before body inference runs.

Resolved doc-derived parameter facts SHALL be available through the shared v2 semantic facts used by diagnostics, hover, type-at-position, completion, signature help, CLI, Web, and `bsl-agent`. Adapter layers SHALL NOT implement separate doc-comment parsing or separate parameter type inference.

Doc-derived evidence SHALL fail closed: unresolved or malformed type hints SHALL NOT invent concrete owners, SHALL NOT stop analysis, and SHALL NOT create high-confidence diagnostics unless a concrete/family type was resolved.

#### Scenario: Routine body inference uses a doc-derived parameter type

- **GIVEN** parameter `Форма` is documented as `ФормаКлиентскогоПриложения`
- **WHEN** v2 analyzes the body of `ПриЧтенииНаСервере`
- **THEN** the symbol `Форма` is seeded with the resolved doc-derived type before body inference
- **AND** hover/type-at-position for `Форма` uses the shared v2 semantic fact

#### Scenario: Parameter shadowing remains stronger than global context

- **GIVEN** a routine parameter has the same name as a global context property or common module
- **AND** the parameter has a doc-derived type hint
- **WHEN** v2 resolves the identifier inside the routine body
- **THEN** the local parameter symbol wins over global context, common modules, and owner-member fallback
- **AND** the parameter's doc-derived type is used as the local symbol type evidence

#### Scenario: bsl-agent observes the same doc-derived type fact

- **GIVEN** v2 has extracted and resolved doc-derived parameter type hints for a routine
- **WHEN** `bsl-agent` serves a type-aware operation for a position inside that routine
- **THEN** the agent result uses the same v2 semantic fact as LSP/CLI/Web
- **AND** the agent does not reparse comments or drop the doc-derived hint at its adapter boundary

### Requirement: v2 SHALL represent broad 1C metadata family parameter types from doc comments

The system SHALL recognize broad 1C metadata family type names in doc-derived parameter hints, including object families such as `СправочникОбъект`, `ДокументОбъект`, `ПланВидовХарактеристикОбъект`, `ПланСчетовОбъект`, `ПланВидовРасчетаОбъект`, `БизнесПроцессОбъект`, `ЗадачаОбъект`, `ПланОбменаОбъект`, and register record/record-set families such as `РегистрСведенийМенеджерЗаписи`, `РегистрСведенийНаборЗаписей`, `РегистрНакопленияНаборЗаписей`, `РегистрБухгалтерииНаборЗаписей`, and `РегистрРасчетаНаборЗаписей`.

These names SHALL resolve to coarse family/facet semantic types when no concrete metadata object name is present. Multiple recognized family names for one parameter SHALL produce a union-like resolution that preserves the family entries instead of collapsing to `Unknown` or to a display-only string.

#### Scenario: BSP object/register parameter resolves to a family union

- **GIVEN** parameter `ТекущийОбъект` is documented with object and register family type lines
- **WHEN** v2 resolves doc-derived parameter hints for the routine
- **THEN** `ТекущийОбъект` receives a union-like resolution containing the recognized object/register family types
- **AND** the result is not collapsed to `Unknown`
- **AND** consumers can distinguish the doc-derived family union from a concrete single metadata object type

#### Scenario: Unknown doc type name fails closed

- **GIVEN** a doc-comment parameter entry names `НесуществующийТипДокументации`
- **WHEN** v2 resolves doc-derived parameter hints
- **THEN** the unknown type name does not produce a concrete owner/member contract
- **AND** analysis continues without parse, IR, or semantic failure
- **AND** high-confidence member diagnostics are not emitted solely from the unknown raw type name
