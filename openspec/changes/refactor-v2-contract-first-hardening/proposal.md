# Change: Contract-first hardening для v2 observability и type-index cache semantics

## Why
После внедрения event-driven `type_index` и serve-only interactive path ключевые функции работают, но остаются источники semantic drift:
- stringly-typed normalize/projection mapping для observability;
- неоднозначность retention semantics артефактов (`N=2` в документах vs version-gap реализация);
- неунифицированный emission serve outcomes для всех interactive операций;
- хрупкая traceability perf artifacts из-за hardcoded `change_id`.

Нужен follow-up change уровня контракта, который делает эти классы регрессий невозможными по конструкции, а не через точечные патчи.

## Selected Approach
В этом change явно фиксируется подход:
- **Contract-first hardening** (рекомендуемый путь для текущего этапа);
- **registry-driven materialization** для canonical/legacy observability mapping;
- **fail-closed provenance** для perf evidence (`change_id` mismatch/absence => invalid evidence).

Полный rewrite observability/perf pipeline для этого change является **вне scope** и ведётся отдельным change.

## What Changes
- **MODIFIED**: observability requirements для канонического event model и dual-write проекций:
  - typed registry как единственный источник truth для stage/reason taxonomy;
  - детерминированная materialization canonical -> legacy;
  - отдельные обязательные проекции для `type_index_precompute` queue/exec/build без деградации в `other`.
- **ADDED**: deterministic count-based retention contract для `TypeIndexArtifact` (фиксированная семантика max versions + latest protection при global guard).
- **ADDED**: единый контракт emission `type_index` serve outcomes для всех interactive операций (`completion`, `hover`, `signatureHelp`, `definition`) в low-cardinality виде.
- **ADDED**: traceability contract для perf-gate/report artifacts (`change_id` должен быть привязан к запуску и проверяться на mismatch).
- **ADDED**: инвариантные contract/parity тесты, блокирующие drift до merge.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (planned):
  - `analysis-v2/src/derived_artifacts.rs`
  - `bsl-runtime/src/system/basic_observability.rs`
  - `backend/src/bin/lsp_server/server/language_server.rs`
  - `backend/src/bin/lsp_server/server/core.rs`
  - `contracts/observability-completion-v2/v1/*`

## Relation To Previous Change
Этот change является hardening-продолжением `refactor-v2-event-driven-type-index-cache` и закрывает обнаруженные drift-риски в контрактах/проекциях/валидации.
