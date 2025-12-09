# Milestone 3.12: Bugfixes - Property Resolution & Variable Declaration

**Статус:** ✅ Завершён
**Приоритет:** Высокий
**Зависимости:** Milestone 3.11 (Context-Aware Facet Selection)
**Дата завершения:** 2025-12-09

## Обзор

Исправление трёх багов, обнаруженных при тестировании фасетной системы типов на файле `examples/bsl/test_milestone_3_11_facets.bsl`.

---

## ✅ Баг 1: Табличные части не видны при валидации свойств

**Файл теста:** `examples/bsl/test_milestone_3_11_facets.bsl:139`

### Симптомы
```bsl
ДокЗаказНарядыСсылка = Документы.ЗаказНаряды.НайтиПоНомеру("ввв");
ДокЗаказНарядыСсылка.Работы.Выгрузить();  // ❌ Ошибка: Свойство 'Работы' не существует
```

### Решение

Добавлены табличные части в `get_facet_properties()` для фасетов Object/Reference.

### Выполненные задачи

- [x] **3.12.1** Модифицирован `get_facet_properties()` в `shared/src/domain/metadata_lookup.rs`
  - Табличные части добавляются как свойства с типом `ТабличнаяЧасть<ИмяТЧ>`
  - `is_readonly: true` для табличных частей

- [x] **3.12.2** Добавлены тесты в `shared/tests/metadata_lookup_tabular_sections_properties_tests.rs`
  - 12 тестов покрывают все фасеты и edge cases

### Изменённые файлы

| Файл | Изменение |
|------|-----------|
| `shared/src/domain/metadata_lookup.rs` | +15 строк в `get_facet_properties()` |
| `shared/tests/metadata_lookup_tabular_sections_properties_tests.rs` | Новый файл, 12 тестов |

---

## ✅ Баг 2: `Перем` не в начале функции не вызывает ошибку

**Файл теста:** `examples/bsl/test_milestone_3_11_facets.bsl:124`

### Симптомы
```bsl
Функция ТестДокументы()
    ДокМенеджер = Документы.ЗаказКлиента;
    Перем ДатаДок;  // ❌ Должна быть ошибка
КонецФункции
```

### Решение

Добавлена проверка позиции `Перем` в semantic validation visitor.

### Выполненные задачи

- [x] **3.12.4** Добавлен метод `validate_var_declaration_position()` в `semantic_validation_visitor.rs`
- [x] **3.12.5** Добавлен `TypeErrorKind::VarDeclarationAfterExecutable`
- [x] **3.12.6** Добавлены тесты (6 тестов)

### Изменённые файлы

| Файл | Изменение |
|------|-----------|
| `shared/src/domain/validators.rs` | +30 строк (новый TypeErrorKind) |
| `backend/src/application/semantic_validation_visitor.rs` | +35 строк (валидация) |
| `backend/tests/semantic_diagnostics_lsp_test.rs` | +6 тестов |

---

## ✅ Баг 3: Неинициализированная переменная не вызывает предупреждение

**Файл теста:** `examples/bsl/test_milestone_3_11_facets.bsl:138`

### Симптомы
```bsl
Перем ДатаДок;  // Объявлена, но не инициализирована
Результат = НайтиПоНомеру("ввв", ДатаДок);  // ❌ Нет предупреждения
```

### Решение (Вариант B2 — унификация)

Реализована структура `VariableState` для унифицированного отслеживания состояния переменных.

### Архитектурные изменения

**Новая структура `VariableState`:**
```rust
pub struct VariableState {
    pub resolution: TypeResolution,
    pub initialized: bool,
    pub declaration_span: Span,
}
```

**Унификация хранилищ:**
- `Scope.variables` теперь использует `HashMap<String, VariableState>`
- `FlowContext.variable_states` теперь использует `HashMap<String, VariableState>`
- Backward-compatible API сохранён

### Выполненные задачи

- [x] **3.12.7** Добавлена структура `VariableState` в `shared/src/ir/mod.rs`
- [x] **3.12.8** Добавлен `TypeErrorKind::UninitializedVariableUsage` (Warning)
- [x] **3.12.9** Реализован data-flow tracking в FlowContext и visitor
- [x] **3.12.10** Добавлены тесты (6 тестов)
- [x] **3.12.11** (бонус) Исправлена регистрация параметров функций в symbol_table

### Изменённые файлы

| Файл | Изменение |
|------|-----------|
| `shared/src/ir/mod.rs` | +50 строк (VariableState, методы SymbolTable) |
| `shared/src/ir/visitor.rs` | +30 строк (FlowContext с VariableState) |
| `shared/src/domain/validators.rs` | +20 строк (TypeErrorKind) |
| `backend/src/application/ast_to_ir.rs` | +15 строк (register_variable_declared, параметры) |
| `backend/src/application/semantic_validation_visitor.rs` | +25 строк (валидация) |
| `backend/tests/semantic_diagnostics_lsp_test.rs` | +6 тестов |

---

## Критерии приёмки

### Баг 1: Табличные части
- [x] `ДокСсылка.ТабличнаяЧасть` не вызывает ошибку
- [x] `ДокОбъект.ТабличнаяЧасть` не вызывает ошибку
- [x] Hover показывает корректный тип табличной части
- [x] Тесты проходят (12 тестов)

### Баг 2: Перем позиция
- [x] `Перем` после исполняемого кода → ошибка
- [x] `Перем` в начале → OK
- [x] Несколько `Перем` подряд в начале → OK
- [x] Тесты проходят (6 тестов)

### Баг 3: Инициализация
- [x] Использование `Перем X` без инициализации → Warning
- [x] `Перем X; X = 1; X.Метод()` → OK
- [x] Параметры функций → всегда initialized
- [x] Тесты проходят (6 тестов)

---

## Итоги

| Баг | Сложность | Тестов | Статус |
|-----|-----------|--------|--------|
| 1. Табличные части | 🟢 Низкая | 12 | ✅ |
| 2. Перем позиция | 🟡 Средняя | 6 | ✅ |
| 3. Инициализация | 🔴 Высокая | 6 | ✅ |

**Всего:** 24 новых теста, ~200 строк кода

### Бонусы

- Унифицированная структура `VariableState` улучшает архитектуру
- Параметры функций теперь регистрируются в symbol_table
- Backward-compatible API
