# Change: Уточнить контракт `FormModule.ЭтотОбъект` как runtime form context с контекстной доступностью members

## Why
Сейчас `FormModule.ЭтотОбъект/ЭтаФорма/Форма` в v2 фактически опираются на synthetic form type и не гарантируют проброс полного набора платформенных свойств/методов `ФормаКлиентскогоПриложения`.

Дополнительно отсутствует единый нормативный контракт на фильтрацию доступности members по текущему фасету использования (директива компиляции/контекст вызова), из-за чего возможны ложные подсказки и несогласованность между completion/hover/diagnostics/type-at-position.

## What Changes
- **MODIFIED**: требование `FormModule предоставляет фиксированный набор implicit symbols (MUST)` в `bsl-intellisense-v2`.
  - `ЭтотОбъект/ЭтаФорма/Форма` MUST трактоваться как runtime form context:
    - базовый платформенный тип `ФормаКлиентскогоПриложения`,
    - extension по типу главного реквизита,
    - локальный shape формы (реквизиты/элементы).
- **MODIFIED**: требование `Member completion для implicit symbols включает свойства и методы (MUST)` в `bsl-intellisense-v2`.
  - Completion для `ЭтотОбъект/ЭтаФорма/Форма` MUST применять контекстную фильтрацию доступности.
- **ADDED**: новое требование о context/facet-aware доступности members runtime form context для всех v2 consumer-ов (`completion`, `hover`, `diagnostics`, `type-at-position`).

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `analysis-v2/src/implicit_bindings.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `shared/src/domain/metadata_lookup/core.rs`
  - `shared/src/domain/runtime_context.rs`
  - `shared/src/domain/validators/type_validator.rs`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/helpers/hover_formatter/*`
  - `bsl-runtime/src/data/loaders/syntax_helper/*`

## Scope
- Изменение ограничено семантикой `FormModule.ЭтотОбъект/ЭтаФорма/Форма` и политикой доступности members по контексту.
- Семантика `FormModule.Объект` (strict form-data) не расширяется и не ослабляется этим change.

