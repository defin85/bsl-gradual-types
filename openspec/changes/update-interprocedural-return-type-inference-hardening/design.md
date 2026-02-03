# Дизайн: hardening межпроцедурного вывода return‑типа (persistent + overlay)

## Контекст
Система уже имеет “двухконтурный” механизм:

- Контур A (persistent): индексация конфигурационных модулей с DiskCache и записью в `SignatureIndex`.
- Контур B (overlay): вычисление return‑summary по open files в `analysis-v2` (salsa), затем использование в v2 inference.

Ревью текущего состояния выявило 3 критичных разрыва:
1) overlay не покрывает ObjectModule/RecordSetModule (хотя они индексируются в `SignatureIndex`);
2) weak/dynamic не сохраняется в persistent‑контуре и теряется на call-site резолве;
3) AST fallback выключает interprocedural inference, т.к. не даёт `return_facts`.

Этот change доводит контракт до рабочего и предсказуемого состояния без смены базовой архитектуры.

## Цели
- Типизация вызовов пользовательских экспортов использует return‑summary:
  - сохраняет известный union типов;
  - при неопределённости помечает результат как weak/dynamic (через `Certainty::InferredWeak`), не затирая union.
- Overlay покрывает **все типы модулей, которые индексируются в `SignatureIndex`**: common/manager/object/recordset.
- Overlay использует `SignatureIndex` как внешний источник summary (без I/O), чтобы open‑файлы корректно типизировались даже при вызовах в закрытые модули.
- Fallback‑парсинг не “обнуляет” interprocedural inference.

## Non‑goals
- Overlay для модулей, не индексируемых в `SignatureIndex` (например, FormModule).
- Полный CFG‑based interprocedural analysis.

## Контракт данных: ReturnDomain/ReturnSummary
Единый домен для обоих контуров:

- `known: Set<String>` — множество строковых типов (варианты union).
- `has_dynamic: bool` — встречалась неопределённость (динамика/неразрешимый вызов/циклическая зависимость переменной).

Правила:
- join: `known ∪=` и `has_dynamic |=`.
- финализация в строку: `known` сортируется/нормализуется и выдаётся как `"A | B | C"`.
- weak/dynamic: если `has_dynamic==true`, то на уровне `TypeResolution` понижаем `Certainty` до `InferredWeak`, но тип остаётся union известных вариантов.

## Persistent‑контур: хранение weak/dynamic в SignatureIndex
Проблема: `SignatureIndex` сейчас хранит только `return_type: Option<String>`, этого недостаточно для UX “сохранить known, но пометить weak”.

Решение:
- расширить сигнатуру метода (например, `MethodSignature`) дополнительным полем `return_is_weak: bool` (serde‑совместимо через `#[serde(default)]`).
- при индексации конфигурации:
  - `return_type` берём из inferred union (если есть), иначе из локального inferred тела;
  - `return_is_weak` = `domain.has_dynamic` для inferred результата.
- при резолве return‑типа через `SignatureIndex` в v2 inference:
  - строим `TypeResolution` по `return_type`;
  - если `return_is_weak`, то `Certainty::InferredWeak` + `UncertaintyReason`.

Это выравнивает поведение с overlay‑контуром (где weak уже умеем выражать через `Certainty::InferredWeak`).

## Overlay: канонические owner_type ключи для module types
Overlay должен использовать те же owner_type key, что и `SignatureIndex`:

### CommonModule
- `ОбщиеМодули.<ИмяОбщегоМодуля>`

### ManagerModule
- `<BaseManagerType>.<Имя>`
  - Пример: `Catalog.Контрагенты` → `СправочникМенеджер.Контрагенты`

### ObjectModule
- `<BaseObjectType>.<Имя>`
  - Пример: `Catalog.Контрагенты` → `СправочникОбъект.Контрагенты`

### RecordSetModule
- `<BaseRecordSetType>.<Имя>`
  - Пример: `AccumulationRegister.<Регистр>` → `РегистрНакопленияНаборЗаписей.<Регистр>`

Источник truth для маппинга:
- `CodeLocation` даёт `ModuleType::{ObjectModule,RecordSetModule,ManagerModule}` с `owner_type` вида `<XmlKind>.<ObjectName>`.
- `XmlKind` конвертируется в `MetadataKind` (например, `Catalog` → `Catalog`, `InformationRegister` → `InformationRegister`).
- `MetadataKind` маппится в “базовое имя типа”:
  - ObjectModule: `Catalog` → `СправочникОбъект`, `Document` → `ДокументОбъект`, …
  - RecordSetModule: `AccumulationRegister` → `РегистрНакопленияНаборЗаписей`, `InformationRegister` → `РегистрСведенийНаборЗаписей`, …

## Overlay fixed‑point: open files ∪ SignatureIndex
Требование: open file может вызывать экспорт из неоткрытого модуля; overlay должен учитывать это, не делая I/O.

Решение:
- при вычислении домена для `Atom::Call(callee)`:
  1) если `callee` есть в `facts_by_fn` (open set) → берём домен из overlay‑таблицы;
  2) иначе пробуем взять return‑summary из `SignatureIndex` по `(owner_type, method_name)`:
     - извлекаем union из `return_type`;
     - `has_dynamic` берём из `return_is_weak`.
  3) если и это недоступно → `has_dynamic=true` (консервативно).

Важно: overlay query должен зависеть от актуального снапшота deps/signature index, чтобы пересчитываться при переиндексации конфигурации (без чтения файлов).

## AST fallback: минимум “консервативных фактов”
Fallback не должен выключать interprocedural inference. Если точные `ReturnFacts` извлечь нельзя, допускается:
- `returns = [Unknown]`, `has_dynamic = true`, `has_return_without_value = false`, `vars = {}`.

Эта деградация даёт:
- отсутствие ложной точности;
- наличие weak‑сигнала для UX;
- отсутствие “провала в пустоту”.

