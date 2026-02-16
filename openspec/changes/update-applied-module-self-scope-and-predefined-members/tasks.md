## 1. Specification
- [ ] 1.1 Обновить `bsl-intellisense-v2`: зафиксировать owner-member fallback для bare identifier в applied object modules.
- [ ] 1.2 Зафиксировать, что системные members (`DataExchange`, `AdditionalProperties` и т.п.) резолвятся через metadata object контекст.
- [ ] 1.3 Добавить requirement на manager path-call (`Коллекция.<Имя>.ExportMethod`) с обязательной резолюцией exported manager methods.
- [ ] 1.4 Добавить requirement на predefined members manager-фасета (включая `ПланыСчетов.*.<Предопределенный>`).
- [ ] 1.5 Зафиксировать breaking-only policy (без safe migration и compatibility toggle).

## 2. Design
- [ ] 2.1 Описать precedence для identifier resolution: local/module vars -> params -> implicit owner members -> undeclared.
- [ ] 2.2 Описать unified source-of-truth для owner-member lookup между diagnostics/hover/completion/type-at-position.
- [ ] 2.3 Описать модель данных predefined items: parser (`Predefined.xml`) -> repository -> manager properties.
- [ ] 2.4 Описать dedupe/precedence при merge manager properties: platform -> metadata attrs -> predefined.
- [ ] 2.5 Описать deterministic alphabetical ordering policy для hover/completion после merge.

## 3. Implementation (follow-up)
- [ ] 3.1 Реализовать fallback `infer_identifier` к implicit owner members для applied object modules.
- [ ] 3.2 Синхронизировать semantic diagnostics с новой моделью, чтобы не генерировать ложные `UndeclaredVariable` в параметрах вызовов.
- [ ] 3.3 Добавить парсинг `Predefined.xml` в config metadata parser и перенос в `RawTypeData`.
- [ ] 3.4 Добавить выдачу predefined members в `TypeMetadataLookup` для manager-фасета поддерживаемых metadata kinds.
- [ ] 3.5 Зафиксировать/дотянуть path resolution для exported manager methods в manager modules на уровне lookup/index контракта.
- [ ] 3.6 Обновить hover/completion сортировку и тесты на алфавитный детерминизм с учетом новых members.
- [ ] 3.7 Добавить guard по `ModuleType`: owner-member fallback не применяется к `FormModule`.

## 4. Validation
- [ ] 4.1 `openspec validate update-applied-module-self-scope-and-predefined-members --strict --no-interactive`
- [ ] 4.2 Прогнать regression-тесты на кейсах:
- [ ] 4.3 `ObjectModule`: `ЗначениеЗаполнено(ДоговорКонтрагента)`, `ОбменДанными.Загрузка`, `ДополнительныеСвойства`.
- [ ] 4.4 `RecordSetModule`: вызов manager export `РегистрыСведений.<Имя>.ВладелецБезопасногоХранилища(...)`.
- [ ] 4.5 `ManagerModule`: `ПланыСчетов.<Имя>.<Предопределенный>` и hover/completion сортировка.
- [ ] 4.6 Кросс-regression с `remove-form-module-dual-layer-semantics`: applied fallback/predefined members не изменяют `FormModule.Объект` strict form-data semantics.
