## ADDED Requirements

### Requirement: v2 SHALL load global-context properties from Syntax Helper

The system SHALL parse Syntax Helper `Global context/properties` pages into a data-driven global-context property index and SHALL use that index as the source of truth for platform global properties.

The system SHALL NOT require source-code changes in `analysis-v2` to add or update a global-context property that is present in the loaded Syntax Helper.

The system SHALL preserve Syntax Helper source provenance for global-context properties and SHALL NOT classify a property as global-context by short name alone.

#### Scenario: Metadata global property resolves from Syntax Helper

- **GIVEN** Syntax Helper contains `Global context.Metadata` with type `ОбъектМетаданныхКонфигурация`
- **WHEN** v2 analyzes `Метаданные`
- **THEN** `Метаданные` resolves as a global-context property of type `ОбъектМетаданныхКонфигурация`
- **AND** no `UndeclaredVariable` diagnostic is emitted for that identifier

#### Scenario: English global property name resolves through the same index

- **GIVEN** Syntax Helper contains bilingual global-context property data for `Метаданные` / `Metadata`
- **WHEN** v2 analyzes `Metadata`
- **THEN** the identifier resolves through the same global-context property entry
- **AND** the result does not depend on a separate hardcoded English-name table in inference

#### Scenario: Same short property name outside Global Context is not indexed as global

- **GIVEN** Syntax Helper contains a type property with the same short name as a global-context property
- **AND** that page is not under `Global context/properties`
- **WHEN** the global-context property index is built
- **THEN** the non-global property is not registered as a bare global identifier
- **AND** only pages with Global Context provenance can create global-context property bindings

### Requirement: Local symbols SHALL shadow Syntax Helper global-context properties

The system SHALL resolve local variables, parameters, module variables, and explicit declarations before global-context properties loaded from Syntax Helper.

#### Scenario: Local assignment shadows Metadata

- **GIVEN** a procedure contains `Метаданные = "local";`
- **WHEN** v2 analyzes `Метаданные` after the assignment
- **THEN** the identifier resolves to the local inferred value
- **AND** the global-context property binding is not used for that local reference

### Requirement: Metadata object chains SHALL prefer repository property data over source hardcodes

The system SHALL resolve chains starting from `Метаданные` through the loaded platform `TypeRepository` properties and metadata collection item information when that data is available.

The system SHALL NOT validate configuration object names inside metadata object collections as fixed platform properties.

The system SHALL attach metadata collection item types from the concrete source property that returned `КоллекцияОбъектовМетаданных`; it SHALL NOT globally assign a single item type to `КоллекцияОбъектовМетаданных`.

#### Scenario: Accumulation register dimension metadata chain resolves to String

- **GIVEN** Syntax Helper and platform type repository contain `ОбъектМетаданныхКонфигурация`, `КоллекцияОбъектовМетаданных`, `ОбъектМетаданных: РегистрНакопления`, and `ОбъектМетаданных: Поле`
- **WHEN** v2 analyzes `Метаданные.РегистрыНакопления.АвансовыеПлатежиИностранцевПоНДФЛ.Измерения.ГоловнаяОрганизация.Имя`
- **THEN** the final `.Имя` resolves to `Строка`
- **AND** no `UndeclaredVariable` or `NonExistentProperty` diagnostic is emitted for the valid metadata chain

#### Scenario: MetadataObjectCollection item type is instance-specific

- **GIVEN** Syntax Helper says `Метаданные.РегистрыНакопления` returns `КоллекцияОбъектовМетаданных` whose elements are `ОбъектМетаданных: РегистрНакопления`
- **AND** another metadata property returns `КоллекцияОбъектовМетаданных` with a different element type
- **WHEN** v2 analyzes both chains in the same file
- **THEN** each collection value keeps its own item type
- **AND** the reusable platform type `КоллекцияОбъектовМетаданных` is not globally mutated

#### Scenario: Missing Syntax Helper data fails closed

- **GIVEN** Syntax Helper global-context properties are unavailable
- **WHEN** v2 analyzes a bare identifier that only exists as a platform global-context property
- **THEN** the analyzer does not invent a precise platform type for it
- **AND** diagnostics may report missing documentation/configuration uncertainty rather than claiming the value is known `Неопределено`

### Requirement: Global metadata manager collection hardcodes SHALL be inventoried

The system SHALL inventory hardcoded global metadata manager collections such as `Справочники`, `Документы`, and `РегистрыНакопления` while adding the Syntax Helper global-context index.

For each hardcoded collection entry, the system SHALL either migrate it to a data-driven Syntax Helper/configuration index or record explicit fallback evidence and a follow-up plan.

#### Scenario: Remaining global collection hardcodes are explicit

- **GIVEN** implementation still needs a hardcoded global metadata manager collection entry after this change
- **WHEN** delivery evidence is produced
- **THEN** the entry is listed with the reason it remains hardcoded
- **AND** the entry is marked as degraded/bootstrap fallback or linked to a follow-up OpenSpec change
