## 1. Design & Contracts
- [ ] 1.1 Зафиксировать структуру descriptor-based модели для implicit symbols (`ContextualTypeDescriptor` и подтипы) в `bsl-types`.
- [ ] 1.2 Зафиксировать контракт преобразования descriptor -> `TypeResolution` (включая `active_facet`, `certainty`, деградации).
- [ ] 1.3 Зафиксировать dual-layer контракт для `FormModule.Объект` в спецификации: canonical form-data semantics + user-facing owner facet label (с form-data пометкой в detailed представлении).

## 2. Resolver Integration
- [ ] 2.1 Перевести `analysis-v2/src/implicit_bindings.rs` на возврат дескрипторов вместо string `type_name`.
- [ ] 2.2 Перевести seed implicit symbols в `analysis-v2/src/ast_to_ir/converter.rs` на descriptor-based контракт.
- [ ] 2.3 Перевести seed implicit symbols в `analysis-v2/src/type_inference_v2.rs` на descriptor-based контракт без string fallback для implicit paths.
- [ ] 2.4 Сохранить текущее поведение `*БезКонтекста` для context-bound symbols.

## 3. Member Resolution
- [ ] 3.1 Добавить descriptor-aware провайдер для form-data member-resolution в `TypeMetadataLookup`.
- [ ] 3.2 Зафиксировать порядок разрешения members: form shape -> guaranteed applied-object members -> applied facet fallback.
- [ ] 3.3 Обеспечить, что `ManagerModule`/`ObjectModule`/`RecordSetModule` используют `ConfigurationFacet` descriptors без псевдотипов.

## 4. UX Compatibility
- [ ] 4.1 Оставить `ДанныеФормыОбъект.*` только как migration compatibility alias (внутренне), без участия в новом semantic contract.
- [ ] 4.2 Обеспечить user-facing отображение `FormModule.Объект` по правилу owner facet (compact/standard) и owner facet + form-data пометка (detailed).
- [ ] 4.3 Обеспечить отсутствие legacy alias и внутренних descriptor имен в user-facing outputs.

## 5. Validation
- [ ] 5.1 Добавить unit-тесты на descriptor->TypeResolution mapping (facet/certainty invariants).
- [ ] 5.2 Добавить интеграционную матрицу `ModuleType x Symbol` (`Объект`, `ЭтотОбъект`, формы/менеджеры/объекты/наборы записей).
- [ ] 5.3 Добавить регрессии на `Объект.Ссылка` в документной форме.
- [ ] 5.4 Добавить кейсы `FormAttributeToValue("Объект")`, `&НаСервереБезКонтекста`, сценарии регистров/иерархических справочников.
- [ ] 5.5 Добавить regression-тесты на user-facing labels для `FormModule.Объект` (compact/standard/detailed).
- [ ] 5.6 `openspec validate add-descriptor-based-contextual-types --strict --no-interactive`.
