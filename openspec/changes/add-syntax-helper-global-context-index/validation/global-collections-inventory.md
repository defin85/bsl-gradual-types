# GLOBAL_COLLECTIONS_INFO Inventory

Date: 2026-04-28
Change: `add-syntax-helper-global-context-index`

## Scope

This note inventories every current `analysis-v2/src/ast_to_ir/global_collections.rs`
`GLOBAL_COLLECTIONS_INFO` entry against checked-in Syntax Helper evidence under
`examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties`.

## Findings

All current entries have a corresponding global-context property page in the
checked-in Syntax Helper sample. These pages are valid data-driven candidates for
the global-context index.

Some hardcoded manager type names do not exactly match the direct Syntax Helper
property type. For example, the hardcoded register/plan entries often use
singular item-manager collection types such as `РегистрНакопленияМенеджерКоллекция`,
while the global-context property page returns the plural manager type such as
`РегистрыНакопленияМенеджер`. Migration must preserve existing inference behavior
only when repository/platform type data proves the replacement shape.

## Migration Decision

All 14 `GLOBAL_COLLECTIONS_INFO` entries are data-derivable from checked-in
Syntax Helper global-context property pages and are migrated into the
`GlobalContextIndex` input surface when Syntax Helper data is loaded. No entry is
documented as a permanent hardcoded source of platform truth.

The existing `GLOBAL_COLLECTIONS_INFO` table remains only as a temporary
degraded/bootstrap fallback candidate until the inference tasks prove index-first
resolution and preserve existing manager-collection behavior. The blocker is
type-shape parity, not missing evidence: several Syntax Helper properties return
plural manager types while the legacy table names singular item-manager
collection types. Follow-up enforcement is tracked inside this same change by
tasks 3.6, 3.7, 7.3, 7.4, and 7.5.

## Inventory

| Entry | English | Hardcoded collection manager | Syntax Helper property type | Evidence |
|---|---|---|---|---|
| `Справочники` | `Catalogs` | `СправочникиМенеджер` | `СправочникиМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Catalogs336.html` |
| `Документы` | `Documents` | `ДокументыМенеджер` | `ДокументыМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Documents900.html` |
| `РегистрыСведений` | `InformationRegisters` | `РегистрСведенийМенеджерКоллекция` | `РегистрыСведенийМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/InformationRegisters901.html` |
| `РегистрыНакопления` | `AccumulationRegisters` | `РегистрНакопленияМенеджерКоллекция` | `РегистрыНакопленияМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/AccumulationRegisters1036.html` |
| `РегистрыБухгалтерии` | `AccountingRegisters` | `РегистрБухгалтерииМенеджерКоллекция` | `РегистрыБухгалтерииМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/AccountingRegisters2504.html` |
| `РегистрыРасчета` | `CalculationRegisters` | `РегистрРасчетаМенеджерКоллекция` | `РегистрыРасчетаМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/CalculationRegisters2043.html` |
| `Перечисления` | `Enums` | `ПеречисленияМенеджер` | `ПеречисленияМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Enums898.html` |
| `Константы` | `Constants` | `КонстантыМенеджер` | `КонстантыМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Constants335.html` |
| `ПланыОбмена` | `ExchangePlans` | `ПланОбменаМенеджерКоллекция` | `ПланыОбменаМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/ExchangePlans3006.html` |
| `ПланыВидовХарактеристик` | `ChartsOfCharacteristicTypes` | `ПланВидовХарактеристикМенеджерКоллекция` | `ПланыВидовХарактеристикМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/ChartsOfCharacteristicTypes2502.html` |
| `ПланыСчетов` | `ChartsOfAccounts` | `ПланСчетовМенеджерКоллекция` | `ПланыСчетовМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/ChartsOfAccounts2503.html` |
| `ПланыВидовРасчета` | `ChartsOfCalculationTypes` | `ПланВидовРасчетаМенеджерКоллекция` | `ПланыВидовРасчетаМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/ChartsOfCalculationTypes2039.html` |
| `БизнесПроцессы` | `BusinessProcesses` | `БизнесПроцессМенеджерКоллекция` | `БизнесПроцессыМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/BusinessProcesses3183.html` |
| `Задачи` | `Tasks` | `ЗадачаМенеджерКоллекция` | `ЗадачиМенеджер` | `examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Tasks3184.html` |

## Follow-up

`GLOBAL_COLLECTIONS_INFO` should remain a degraded/bootstrap fallback candidate
until the inference layer enforces loaded-index precedence and at least one
global manager collection is proven to resolve from loaded Syntax Helper data
without the direct inference-table dependency.

## 3.6 Demotion Evidence

`GLOBAL_CONTEXT_PROPERTIES_INFO` and its lookup were removed from the
production analysis path; loaded global-context properties are resolved through
`GlobalContextIndex`.

The metadata-object collection table was renamed to
`LEGACY_METADATA_OBJECT_COLLECTION_FALLBACKS` and is reached only after
repository/Syntax Helper properties for `ОбъектМетаданныхКонфигурация` or a
metadata object type fail to provide a collection item type.

## 7.4 Remaining Hardcode Register

Owner: `add-syntax-helper-global-context-index` until archive. If archive review
decides these fallbacks should be removed rather than retained for degraded
docs/bootstrap mode, open a narrow follow-up change from this register.

Remaining `GLOBAL_COLLECTIONS_INFO` entries:

- `Справочники` / `Catalogs`
- `Документы` / `Documents`
- `РегистрыСведений` / `InformationRegisters`
- `РегистрыНакопления` / `AccumulationRegisters`
- `РегистрыБухгалтерии` / `AccountingRegisters`
- `РегистрыРасчета` / `CalculationRegisters`
- `Перечисления` / `Enums`
- `Константы` / `Constants`
- `ПланыОбмена` / `ExchangePlans`
- `ПланыВидовХарактеристик` / `ChartsOfCharacteristicTypes`
- `ПланыСчетов` / `ChartsOfAccounts`
- `ПланыВидовРасчета` / `ChartsOfCalculationTypes`
- `БизнесПроцессы` / `BusinessProcesses`
- `Задачи` / `Tasks`

Reason: degraded/bootstrap fallback for global manager collections when
Syntax Helper global-context data is absent or unavailable. Loaded
`GlobalContextIndex` entries are authoritative and have precedence.

Remaining `LEGACY_METADATA_OBJECT_COLLECTION_FALLBACKS` entries:

- `Справочники` / `Catalogs`
- `Документы` / `Documents`
- `РегистрыСведений` / `InformationRegisters`
- `РегистрыНакопления` / `AccumulationRegisters`
- `РегистрыБухгалтерии` / `AccountingRegisters`
- `РегистрыРасчета` / `CalculationRegisters`
- `Перечисления` / `Enums`
- `Константы` / `Constants`
- `ПланыОбмена` / `ExchangePlans`
- `ПланыВидовХарактеристик` / `ChartsOfCharacteristicTypes`
- `ПланыСчетов` / `ChartsOfAccounts`
- `ПланыВидовРасчета` / `ChartsOfCalculationTypes`
- `БизнесПроцессы` / `BusinessProcesses`
- `Задачи` / `Tasks`

Reason: degraded/bootstrap fallback for `Метаданные.<collection>` item type
when repository/Syntax Helper property-level `collection_item_type` evidence is
absent.

Remaining nested metadata collection fallback entries:

- `Измерения` / `Dimensions`
- `Реквизиты` / `Attributes`
- `Ресурсы` / `Resources`

Reason: degraded/bootstrap fallback for metadata object field collections when
repository/Syntax Helper property-level `collection_item_type` evidence is
absent.
