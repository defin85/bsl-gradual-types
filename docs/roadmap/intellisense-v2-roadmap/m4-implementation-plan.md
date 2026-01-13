# План реализации M4: Типизация выражений для completion (call/index/ternary/paren)

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** вычислять тип receiver‑выражения из M3 для реальных выражений BSL: вызовы методов/функций, индексаторы, скобки, `?()` и `Выбор`, включая интеграцию stdlib + metadata и фасеты.

**Примечание:** базовые данные доступны через v2 engine:
- `SemanticProgram` из `AnalysisV2::ir(FileId)` (зависит от `deps_id`),
- “deps” (`TypeRepository/Resolver/SignatureIndex`) из deps snapshot (часть `DepsBundleV2`),
- `IndexSnapshot` (metadata индексация) берём из `snapshot_with_deps()` на LSP уровне.

---

## Область работ

- Типизация сегментов цепочки:
  - property access → type of property
  - method call → return type (по сигнатурам)
  - index access → element type (для коллекций)
  - parentheses → type passthrough
  - `?(cond, a, b)` → union type
  - `Выбор ... Конец` → union type
- Типизация должна быть устойчивой к Unknown и давать best‑effort.

**Ограничение текущего синтаксического AST:** в `bsl_syntax::ast::Expression` нет узла для `Выбор ... Конец`.
Для completion `Выбор` поддержан best‑effort: ветви (`Тогда`/`Иначе`) извлекаются из текста receiver’а в `CompletionTarget` и типизируются как union.

---

## Пошаговый план

### Шаг 1: Return type для вызовов (stdlib + metadata)
- Использовать SignatureIndex/TypeRepository как первичный источник.
- Fallback: syntax_helper metadata (если сигнатура отсутствует).

**Выход:** тип `obj.Метод()` вычисляется предсказуемо.

---

### Шаг 2: Типы свойств (property access)
- Для stdlib: типы свойств из syntax_helper.
- Для metadata: типы реквизитов/табличных частей/коллекций.

**Выход:** тип `obj.Свойство` корректен и участвует в дальнейшей цепочке.

---

### Шаг 3: Индексаторы `[]`
- Ввести модель element‑типа для ключевых коллекций:
  - `Массив[T]`, `Соответствие[K,V]` (если есть generics)
  - платформенные специализированные коллекции (ValueTable.Columns и т.п.)
- Fallback: Unknown при невозможности вычисления.

**Выход:** тип `Коллекция[i]` даёт элементы с корректными members.

---

### Шаг 4: Условные выражения → union type
- `?(cond, a, b)` → `type(a) | type(b)`
- `Выбор ... Конец` → union по веткам

**Выход:** completion по `?(...).` и `Выбор...Конец.` работает best‑effort.

---

## Критерии завершения

- Тип receiver’а вычисляется для перечисленных конструкций.
- Unknown не “ломает” pipeline: есть fallback‑поведение.
- Добавлены unit/golden тесты на типизацию выражений.

---

## Задачи (тикеты) по M4

### T1: Типизация вызовов по сигнатурам ✅
**DoD:**
- return type для методов/функций извлекается по индексу;
- покрыто тестами.

### T2: Типизация свойств ✅
**DoD:**
- stdlib свойства типизируются из docs;
- metadata свойства типизируются из конфигурации;
- тесты на цепочки `a.prop.b.`.

### T3: Индексаторы `[]` ✅
**DoD:**
- элементные типы определены для ключевых коллекций;
- тесты на `arr[0].` и `map["k"].`.

### T4: `?()` и `Выбор` ✅
**DoD:**
- union type вычисляется;
- completion не деградирует на этих выражениях.

---

## Прогресс (факты по коду)

- Type inference receiver‑выражений для completion: `backend/src/application/type_system/services/completion_service.rs` (`resolve_receiver_types_from_expression` и helpers).
- Метод‑call return type берётся из `SignatureIndex` через `TypeMetadataLookup` + применяется подстановка имени метаданных для фасетных типов: `backend/src/application/type_system/services/completion_service.rs` (`substitute_type_name_if_needed`).
- Индексаторы `[]`: element type определяется через generics, а при `Массив<Неопределено>` есть fallback на `RawTypeData.collection_item_type`: `backend/src/application/type_system/services/completion_service.rs`.
- `?()` (ternary_expression) даёт best‑effort union (как набор типов): `backend/src/application/type_system/services/completion_service.rs`.
- `Выбор ... Конец` даёт best‑effort union по веткам (`Тогда`/`Иначе`) через разбор текста receiver’а: `backend/src/application/type_system/services/completion_target.rs` + `backend/src/application/type_system/services/completion_service.rs`.
- Index access корректно конвертируется в `Expression::IndexAccess` в синтаксическом слое: `syntax/src/tree_sitter_adapter/expression_converter.rs`.

## Проверка

- `cargo test -p bsl-backend completion_supports_member_access_after_method_call`
- `cargo test -p bsl-backend completion_supports_member_access_after_index_access`
- `cargo test -p bsl-backend completion_supports_member_access_after_map_index_access`
- `cargo test -p bsl-backend completion_supports_member_access_after_ternary_expression`
- `cargo test -p bsl-backend completion_supports_member_access_after_choice_expression`
- `cargo test -p bsl-backend completion_substitutes_faceted_metadata_name_in_return_type`
