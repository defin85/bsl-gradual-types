# Roadmap: Undeclared Variables - Phase 2

Расширение функционала проверки необъявленных переменных.

## Статус Phase 1 (завершён)

- [x] Базовая инфраструктура (`UncertaintyReason::UndeclaredVariable`)
- [x] Проверка простых идентификаторов в аргументах методов
- [x] Корректная обработка литералов, `Неопределено`, `Null`
- [x] Корректная обработка глобальных коллекций (Справочники, Документы и т.д.)
- [x] Тестовое покрытие: 17 тестов

## Phase 2: Параметры функций ✅ ЗАВЕРШЁН

**Статус:** ✅ Завершён (2025-12-09, в рамках Milestone 3.12)
**Приоритет:** Высокий
**Сложность:** Низкая

### Реализация

Параметры функций и процедур теперь регистрируются в `symbol_table` при AST-to-IR конвертации.

### Изменённые файлы

| Файл | Изменение |
|------|-----------|
| `backend/src/application/ast_to_ir.rs` | Регистрация параметров в `symbol_table` |
| `shared/src/ir/mod.rs` | `VariableState` структура для отслеживания состояния |

### Тесты (активированы)

- [x] `test_function_parameter_is_declared` — параметр функции считается объявленным
- [x] `test_real_scenario_catalog_search` — реальный сценарий с параметром

---

## Phase 3: BSL Function Scope ✅ ЗАВЕРШЁН

**Статус:** ✅ Завершён (2025-12-10)
**Приоритет:** Средний
**Сложность:** Средняя

### Проблема (решена)

В BSL переменные видны во всей функции (function scope), независимо от места объявления.

### Решение: ScopeKind

Добавлен enum `ScopeKind` для различения типов scope:

```rust
pub enum ScopeKind {
    Global,     // root scope
    Function,   // body функции/процедуры
    Block,      // if/while/for блоки
}
```

Переменные теперь регистрируются в ближайшем function scope через метод `register_variable_in_function_scope()`.

### Изменённые файлы

| Файл | Изменение |
|------|-----------|
| `shared/src/ir/mod.rs` | `ScopeKind`, `find_enclosing_function_scope()`, `register_variable_in_function_scope()` |
| `backend/src/application/ast_to_ir.rs` | Использование `ScopeKind::Function` для функций, `ScopeKind::Block` для блоков |

### Тесты (активированы)

- [x] `test_variable_declared_in_if_branch` — переменная из if-блока видна в функции
- [x] `test_variable_declared_in_loop` — переменная из цикла видна в функции

---

## Phase 4: Property Access и Method Chains

**Приоритет:** Средний
**Сложность:** Средняя
**Оценка:** 3-4 часа

### Проблема

Проверка необъявленных переменных работает только для простых идентификаторов. Property access и method chains на необъявленных переменных не детектируются.

### Затрагиваемые файлы

| Файл | Изменение |
|------|-----------|
| `backend/src/application/ast_to_ir.rs` | Проверка base в `PropertyAccess` |
| `backend/src/application/semantic_validation_visitor.rs` | Возможно дополнительная валидация |

### План реализации

1. В `infer_type_resolution` для `Expression::PropertyAccess` проверять base expression
2. Если base — идентификатор и не найден в `symbol_table`, пометить как undeclared
3. Рекурсивно проверять вложенные property access
4. Передать информацию в валидатор

### Тесты для активации

- `test_property_access_on_undeclared_variable`
- `test_method_chain_on_undeclared_variable`

---

## Phase 5: Глобальные функции без receiver

**Приоритет:** Низкий
**Сложность:** Низкая
**Оценка:** 1-2 часа

### Проблема

Проверка выполняется только для method calls с объектом. Глобальные функции не проверяются.

### Затрагиваемые файлы

| Файл | Изменение |
|------|-----------|
| `backend/src/application/semantic_validation_visitor.rs` | Добавить case для `object_type: None` |

### План реализации

1. Добавить match case в `visit_node` для `FunctionCall` с `object_type: None`
2. Применить ту же логику проверки `arg_types` на undeclared variables

---

## Phase 6: Расширенные контексты

**Приоритет:** Низкий
**Сложность:** Низкая
**Оценка:** 2-3 часа

### Проблема

Проверка выполняется только в аргументах методов. Другие контексты не проверяются.

### Дополнительные контексты

- Правая часть присваивания: `а = б + в`
- Return statements: `Возврат х`
- Условия: `Если х Тогда`
- Индексы массивов: `Массив[индекс]`

### Затрагиваемые файлы

| Файл | Изменение |
|------|-----------|
| `backend/src/application/semantic_validation_visitor.rs` | Обработка дополнительных `SemanticNodeKind` |

### План реализации

1. Добавить проверку в `SemanticNodeKind::Assignment`
2. Добавить проверку в `SemanticNodeKind::Return`
3. Добавить проверку в `SemanticNodeKind::IfStatement`

---

## Метрики успеха

| Phase | Тесты до | Тесты после | Покрытие |
|-------|----------|-------------|----------|
| 1 (done) | 0 | 15 | 71% |
| 2 | 15 | 17 | 81% |
| 3 | 17 | 19 | 90% |
| 4 | 19 | 21 | 100% |
| 5 | - | +2 | - |
| 6 | - | +4 | - |

---

## Связанные файлы

- Тесты: `backend/tests/undeclared_variable_test.rs`
- README тестов: `backend/tests/undeclared_variable_test_README.md`
- Основная реализация: `backend/src/application/semantic_validation_visitor.rs`
- Инфраструктура: `shared/src/domain/types.rs` (`UncertaintyReason::UndeclaredVariable`)

---

**Создан:** 2025-12-08
**Версия:** 1.0
