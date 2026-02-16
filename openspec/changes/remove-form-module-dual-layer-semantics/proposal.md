# Change: Remove dual-layer semantics for FormModule implicit symbols (breaking)

## Why
Текущий контракт dual-layer для `FormModule.Объект` (shape + intrinsic + object facet) не соответствует фактическому runtime-поведению форм 1С и приводит к ложным/избыточным members в `hover/completion`.

Нужен жёсткий переход на runtime-совместимую модель без режима совместимости, чтобы убрать смешение форм-данных и фасета прикладного объекта.

## What Changes
- **BREAKING**: убрать dual-layer семантику `FormModule.Объект` (shape/intrinsic/facet chain) как source of truth.
- **BREAKING**: `FormModule.Объект` резолвится как strict form-data (`ДанныеФормыСтруктура`) по данным формы, без автоматического подмешивания `DocumentObject/CatalogObject` members.
- **BREAKING**: user-facing label для `FormModule.Объект` больше не отображается как owner object facet (`ДокументОбъект.X`) в compact/full режимах.
- **BREAKING**: `FormModule.ЭтотОбъект/ЭтаФорма/Форма` резолвятся как контекст формы (`ФормаКлиентскогоПриложения` + extension главного реквизита + реквизиты формы), а не как сокращённая/смешанная модель.
- Удалить requirement-уровневую политику safe migration/fallback для этой архитектуры (без feature flag и без compatibility alias path для поведения).
- Зафиксировать единое поведение для `hover`, `completion`, `type-at-position`, `diagnostics` на новой модели.

## Coordination Boundaries
- Этот change ограничен `FormModule` и MUST NOT менять правила bare-identifier fallback в `ObjectModule`/`RecordSetModule`/`ManagerModule`.
- Изменения applied module self-scope, manager path-calls и predefined members находятся в `update-applied-module-self-scope-and-predefined-members`.
- При совместном внедрении оба change MUST сохранять invariant: form strict semantics не размывается fallback-логикой applied modules.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/implicit_bindings.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `shared/src/domain/metadata_lookup/core.rs`
  - `shared/src/domain/validators/type_validator.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/helpers/hover_formatter/*`
  - `backend/tests/*form*implicit*` и `backend/tests/*completion*`

## Non-Goals
- Сохранение текущего dual-layer поведения в любом runtime режиме.
- Временный compatibility toggle или phased rollout.
- Обратная совместимость с ожидаемыми `DocumentObject.*` members на `FormModule.Объект`.
