# Change: Добавить facet-aware code completion для implicit symbols и их members

## Why
В текущем состоянии v2 implicit symbols (`ЭтотОбъект`, `Объект`, `Форма`, `ЭтаФорма`, `Элементы`, `Параметры` и аналоги) уже типизируются контекстно, но контракт member-completion по этим символам (свойства + методы) не зафиксирован как отдельное обязательство.

На практике это приводит к разрыву UX:
- symbol может считаться объявленным, но completion на `Объект.`/`ЭтотОбъект.` неполный или нестабильный;
- свойства и методы могут резолвиться разными путями в completion vs hover/diagnostics;
- неочевидно, какие members гарантированы для form-data контекста.

## What Changes
- Добавить в `bsl-intellisense-v2` отдельные требования на code completion для context implicit symbols.
- Зафиксировать, что member-completion для implicit symbols обязан возвращать и свойства, и методы на основе descriptor/facet-aware lookup.
- Зафиксировать детерминированный provider chain (collection order) для `FormModule.Объект`:
  - форма (реквизиты/элементы/табличные части по shape),
  - intrinsic supplement (минимальный whitelist гарантированных свойств),
  - applied object facet (свойства + методы),
  - fallback-провайдеры (если нужно для деградации без загруженной конфигурации).
- Развести два контракта:
  - collection order (порядок формирования выдачи),
  - precedence policy (кто побеждает при конфликте имени/member key).
- Зафиксировать, что intrinsic-слой только дополняет lookup (additive-only), а при конфликте с repository/facet members выигрывает repository/facet.
- Зафиксировать canonical дедуп-правила для member completion, включая owner-sensitive кейсы.
- Зафиксировать bounded output для интерактивного completion (limit + `isIncomplete`) и детерминированный `sort` в рамках одного snapshot.
- Зафиксировать, что в контекстах `*БезКонтекста` context-bound implicit symbols не попадают в non-member completion.
- Добавить интеграционные regression-сценарии completion для `FormModule`, `ManagerModule`, `ObjectModule`, `RecordSetModule`.
- Добавить NFR и observability-критерии для интерактивной задержки completion и rollout/rollback через feature flag.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (implementation follow-up):
  - `analysis-v2/src/implicit_bindings.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `shared/src/domain/metadata_lookup/*`
  - `bsl-runtime/src/application/type_system/services/completion_service.rs`
  - `bsl-runtime/src/application/type_system/services/completion_ranking.rs`
  - `backend/src/bin/lsp_server/handlers/completion.rs`
  - `backend/tests/*completion*`

## Non-Goals
- Полное моделирование runtime-поведения форм сверх статического metadata/facet контракта.
- Внедрение новых пользовательских источников типов вне v2 pipeline.
- Изменение LSP-протокола completion beyond current item contract.
- Расширение intrinsic whitelist без платформенного/продуктового обоснования.
