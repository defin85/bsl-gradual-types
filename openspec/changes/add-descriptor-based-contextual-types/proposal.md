# Change: Ввести descriptor-based модель контекстных типов для implicit symbols в v2

## Why
Текущий v2 путь типизации implicit symbols опирается на string-based `type_name`, из-за чего теряется часть семантического контекста (особенно facet context), а member-resolution вынужден чаще уходить в fallback.

Это создаёт архитектурный разрыв между:
- implicit binding (строковые имена),
- `TypeResolution`/`TypeMetadataLookup` (структурная фасетная модель),
- form-data сценариями (`FormModule.Объект`).

Нужна единая descriptor-based модель, чтобы implicit symbols резолвились структурно и предсказуемо на всех этапах v2 pipeline.

## What Changes
- Ввести descriptor-based представление контекстных implicit типов (вместо string-only binding) как архитектурный контракт v2.
- Перевести `ImplicitBindingResolver` на возврат семантических дескрипторов, а не строковых псевдотипов.
- Определить и зафиксировать правила преобразования descriptor -> `TypeResolution` с сохранением `active_facet`.
- Добавить отдельный descriptor-aware путь member-resolution для form-data контекста (`FormModule.Объект`) с детерминированным порядком провайдеров.
- Зафиксировать dual-layer контракт для `FormModule.Объект`:
  - canonical semantic layer: form-data descriptor (`ДанныеФормыСтруктура` semantics);
  - user-facing layer: owner object facet label (`ДокументОбъект.X` и аналоги), с явной пометкой form-data в detailed представлении.
- Оставить legacy alias `ДанныеФормыОбъект.*` только как совместимость на входе/миграции, без попадания в user-facing output.
- Добавить регрессионный тестовый контракт для матрицы `ModuleType x Symbol` и form-data кейсов.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `bsl-types/src/types/*`
  - `analysis-v2/src/implicit_bindings.rs`
  - `analysis-v2/src/ast_to_ir/converter.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `shared/src/domain/metadata_lookup/*`
  - `shared/src/domain/resolver/*`
  - `backend/tests/*`
