# Change: Обновить self-scope прикладных модулей и резолв predefined members (breaking)

## Why
В v2 есть системный разрыв между 1С runtime-контекстом и статическим резолвом в прикладных модулях:
- в `ObjectModule`/`RecordSetModule` прямые обращения к реквизитам/системным свойствам объекта (`ДоговорКонтрагента`, `ОбменДанными`, `ДополнительныеСвойства`) ошибочно уходят в `Необъявленная переменная`;
- не зафиксирован обязательный контракт доступа к exported manager-методам через `КоллекцияМетаданных.<Имя>.Метод(...)`;
- `Predefined.xml` не участвует в построении member-модели manager-фасета, из-за чего выражения вида `ПланыСчетов.Хозрасчетный.ГотоваяПродукция` не резолвятся.

Нужен архитектурный переход на runtime-совместимую модель без compatibility-слоя.

## What Changes
- **BREAKING**: изменить семантику `bare identifier` в `ObjectModule`/`RecordSetModule`/аналогичных applied object modules: перед `undeclared` система MUST проверять members implicit owner (`ЭтотОбъект`/`Объект`) по объектному контексту.
- **BREAKING**: зафиксировать, что `DataExchange`, `AdditionalProperties` и прочие системные/object members резолвятся как members metadata object-контекста, даже при прямом обращении без `ЭтотОбъект.`.
- **BREAKING**: manager-facet member model MUST включать readonly predefined items (из `Predefined.xml`/PredefinedDataName) как свойства manager-объекта для поддерживаемых metadata kinds.
- Зафиксировать контракт: exported методы из manager module доступны через metadata collection path (`РегистрыСведений.<Имя>.<ExportMethod>` и аналоги).
- Зафиксировать единый детерминированный алфавитный порядок в `hover/completion` для properties/methods/predefined members после merge из всех provider-источников.
- Прямо исключить safe migration: без feature-flag, без compatibility fallback path.

## Coordination Boundaries
- Этот change MUST применяться только к applied object contexts (`ObjectModule`, `RecordSetModule`, manager path resolution, manager predefined members).
- Этот change MUST NOT менять `FormModule.Объект` semantics; поведение формы задаётся change `remove-form-module-dual-layer-semantics`.
- При совместном внедрении owner-member fallback MUST оставаться выключенным для `FormModule`, чтобы не вернуть dual-layer поведение.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/type_inference_v2.rs`
  - `analysis-v2/src/implicit_bindings.rs`
  - `semantic-diagnostics/src/visitor.rs`
  - `shared/src/domain/metadata_lookup/core.rs`
  - `shared/src/domain/metadata_lookup/facets.rs`
  - `bsl-runtime/src/data/loaders/config_metadata_parser/*`
  - `bsl-runtime/src/data/loaders/config_bsl_modules/indexing.rs`
  - `bsl-runtime/src/helpers/hover_formatter/*`
  - `backend/tests/*implicit*`, `backend/tests/*hover*`, `backend/tests/*predefined*`

## Non-Goals
- Поддержка dual-layer/safe-migration режима для старой семантики.
- Частичный rollout по отдельным metadata kinds через runtime toggle.
- Изменение протокольного формата LSP beyond текущих полей.
