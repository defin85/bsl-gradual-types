# Change: Унифицировать семантику scope для FormModule и applied modules (breaking)

## Why
Два отдельных change (`remove-form-module-dual-layer-semantics` и `update-applied-module-self-scope-and-predefined-members`) описывают один и тот же pipeline (`lookup/inference/completion/hover/diagnostics`) и создают риск конфликтующих правил.

Нужен один источник правды с явной матрицей по `ModuleType`, чтобы одновременно:
- закрепить strict form-data контракт для `FormModule.Объект`;
- добавить owner-member fallback только для applied object contexts;
- добавить predefined manager members без регрессии формы.

## What Changes
- **BREAKING**: `FormModule.Объект` MUST резолвиться как strict form-data (`ДанныеФормыСтруктура`) без object-facet fallback.
- **BREAKING**: `FormModule.ЭтотОбъект/ЭтаФорма/Форма` MUST резолвиться как контекст формы (`ФормаКлиентскогоПриложения` + extension главного реквизита + реквизиты формы).
- **BREAKING**: в `ObjectModule`/`RecordSetModule`/совместимых applied contexts bare identifier MUST проверяться через owner members перед `UndeclaredVariable`.
- **BREAKING**: manager-facet MUST включать readonly predefined members из `Predefined.xml`/`PredefinedDataName`.
- Зафиксировать контракт path-call для exported manager methods через `КоллекцияМетаданных.<Имя>.<Метод>(...)`.
- Зафиксировать единый source-of-truth lookup для `type-at-position`, `diagnostics`, `hover`, `completion`.
- Зафиксировать детерминированный алфавитный порядок members после merge provider-слоёв.
- Зафиксировать breaking-only adoption: без feature flags и compatibility fallback.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/implicit_bindings.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `shared/src/domain/metadata_lookup/core.rs`
  - `shared/src/domain/metadata_lookup/facets.rs`
  - `shared/src/domain/validators/type_validator.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/helpers/hover_formatter/*`
  - `bsl-runtime/src/data/loaders/config_metadata_parser/*`
  - `bsl-runtime/src/data/loaders/config_bsl_modules/indexing.rs`
  - `backend/tests/*implicit*`, `backend/tests/*hover*`, `backend/tests/*predefined*`

## Supersedes
- `remove-form-module-dual-layer-semantics`
- `update-applied-module-self-scope-and-predefined-members`

С этого момента оба change считаются superseded и не должны использоваться как отдельные источники требований.

## Non-Goals
- Сохранение dual-layer поведения `FormModule.Объект`.
- Compatibility toggle или staged rollout.
- Ослабление строгого form-contract ради applied fallback.
