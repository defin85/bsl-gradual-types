## 1. Specification
- [x] 1.1 Добавить spec delta в `bsl-intellisense-v2` для completion по implicit symbols и их members.
- [x] 1.2 Зафиксировать контракт provider chain для `FormModule.Объект` (shape -> intrinsic supplement -> facet lookup).
- [x] 1.3 Зафиксировать поведение completion в контекстах `*БезКонтекста` для context-bound implicit symbols.
- [x] 1.4 Развести в spec `collection order` и `precedence policy` и добавить scenario на конфликт intrinsic vs repository/facet member.
- [x] 1.5 Зафиксировать bounded output контракт (`limit` + `isIncomplete`) и детерминированный порядок выдачи.

## 2. Design
- [x] 2.1 Описать единый source-of-truth для member-completion implicit symbols через descriptor/facet-aware lookup.
- [x] 2.2 Зафиксировать политику intrinsic supplement: whitelist, additive-only, без override facet metadata.
- [x] 2.3 Зафиксировать правила нормализации/дедупликации properties+methods в completion output.
- [x] 2.4 Зафиксировать canonical dedupe key с owner-sensitive семантикой для union/chain owner-кейсов.
- [x] 2.5 Зафиксировать NFR: latency budget, stage telemetry (`resolve/collect/rank/format`), bounded output.
- [x] 2.6 Зафиксировать rollout/rollback strategy через feature flag и canary-проверку.

## 3. Implementation (follow-up)
- [x] 3.1 Реализовать единый owner-resolution path для completion/hover/type-at-position/diagnostics (shared source-of-truth).
- [x] 3.2 Реализовать provider merge с раздельными правилами collection order и precedence (intrinsic never overrides repository/facet).
- [x] 3.3 Подключить facet-aware lookup свойств и методов для `FormModule`/`ManagerModule`/`ObjectModule`/`RecordSetModule`.
- [x] 3.4 Доработать ranking/dedup для owner-sensitive кейсов и стабильной классификации kind.
- [x] 3.5 Добавить regression/e2e тесты: `*БезКонтекста`, tri-layer `shape+intrinsic+facet`, provider conflicts, false `NonExistentProperty` guard.
- [x] 3.6 Обеспечить bounded output (`limit` + `isIncomplete`) и стабильный `sortText` в LSP-выдаче.

## 4. Validation
- [x] 4.1 `openspec validate add-implicit-symbol-member-completion --strict --no-interactive`
- [ ] 4.2 Review change с владельцами `analysis-v2` и `completion_service`.
- [ ] 4.3 Проверить regression matrix и зафиксировать baseline/after latency метрики для интерактивных completion-сценариев.
