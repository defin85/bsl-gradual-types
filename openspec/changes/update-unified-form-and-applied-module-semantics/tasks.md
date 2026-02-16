## 1. Specification
- [x] 1.1 Объединить form/applied требования в едином `bsl-intellisense-v2` delta-контракте.
- [x] 1.2 Зафиксировать matrix policy по `ModuleType` (FormModule strict, applied owner fallback).
- [x] 1.3 Зафиксировать manager path-call контракт для exported manager methods.
- [x] 1.4 Зафиксировать predefined manager members (`Predefined.xml`/`PredefinedDataName`) как readonly properties.
- [x] 1.5 Зафиксировать deterministic ordering policy в hover/completion.
- [x] 1.6 Зафиксировать breaking-only policy (без safe migration/compatibility toggle).

## 2. Design
- [x] 2.1 Описать общий identifier pipeline: `local -> global -> explicit common module -> owner members -> undeclared`.
- [x] 2.2 Зафиксировать guard: owner fallback применяется только в applied object contexts.
- [x] 2.3 Описать unified source-of-truth между diagnostics/hover/completion/type-at-position.
- [x] 2.4 Описать parser/model projection для predefined members.
- [x] 2.5 Описать merge precedence и dedupe policy для manager properties.

## 3. Implementation (follow-up)
- [x] 3.1 Убрать object-facet fallback для `FormModule.Объект` в metadata lookup chain.
- [x] 3.2 Добавить owner-member fallback в `infer_identifier` для applied object contexts.
- [x] 3.3 Добавить guard по `ModuleType`, исключающий fallback в `FormModule`.
- [x] 3.4 Добавить парсинг `Predefined.xml`/`PredefinedDataName` и перенос в metadata model.
- [x] 3.5 Добавить predefined members в manager-facet lookup как readonly properties.
- [x] 3.6 Обновить user-facing type labels (hover/diagnostics/completion) согласно strict form-data semantics.
- [x] 3.7 Обновить сортировку members в hover/completion до детерминированного алфавитного порядка.
- [x] 3.8 Переписать dual-layer contract tests и добавить cross-regression тесты form vs applied.

## 4. Validation
- [x] 4.1 `openspec validate update-unified-form-and-applied-module-semantics --strict --no-interactive`
- [x] 4.2 `FormModule`: `ЭтотОбъект` vs `Объект` и отсутствие applied object-facet members на `Объект.`.
- [x] 4.3 `ObjectModule`: bare identifier реквизита (`ЗначениеЗаполнено(ДоговорКонтрагента)`) без `UndeclaredVariable`.
- [x] 4.4 `RecordSetModule`: `ОбменДанными`/`ДополнительныеСвойства` и manager path-call экспортного метода.
- [x] 4.5 `ManagerModule`: `ПланыСчетов.<Имя>.<Предопределенный>` резолвится и попадает в hover/completion.
- [x] 4.6 Подтвердить отсутствие compatibility switch и legacy fallback path.
