## 1. Specification
- [ ] 1.1 Добавить spec delta в `bsl-intellisense-v2` для completion по implicit symbols и их members.
- [ ] 1.2 Зафиксировать контракт provider chain для `FormModule.Объект` (shape -> intrinsic supplement -> facet lookup).
- [ ] 1.3 Зафиксировать поведение completion в контекстах `*БезКонтекста` для context-bound implicit symbols.

## 2. Design
- [ ] 2.1 Описать единый source-of-truth для member-completion implicit symbols через descriptor/facet-aware lookup.
- [ ] 2.2 Зафиксировать политику intrinsic supplement: whitelist, additive-only, без override facet metadata.
- [ ] 2.3 Зафиксировать правила нормализации/дедупликации properties+methods в completion output.

## 3. Implementation (follow-up)
- [ ] 3.1 Реализовать member-completion implicit symbols в `completion_service` через v2 snapshot и descriptor-aware owner resolution.
- [ ] 3.2 Подключить facet-aware lookup свойств и методов для `FormModule`/`ManagerModule`/`ObjectModule`/`RecordSetModule`.
- [ ] 3.3 Добавить regression-тесты completion на кейсы `ЭтотОбъект.`/`Объект.` и shape/facet members.

## 4. Validation
- [ ] 4.1 `openspec validate add-implicit-symbol-member-completion --strict --no-interactive`
- [ ] 4.2 Review change с владельцами `analysis-v2` и `completion_service`.
