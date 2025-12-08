# Roadmap: Undeclared Variables - Phase 2

Расширение функционала проверки необъявленных переменных.

## Статус Phase 1 (завершён)

- [x] Базовая инфраструктура (`UncertaintyReason::UndeclaredVariable`)
- [x] Проверка простых идентификаторов в аргументах методов
- [x] Корректная обработка литералов, `Неопределено`, `Null`
- [x] Корректная обработка глобальных коллекций (Справочники, Документы и т.д.)
- [x] Тестовое покрытие: 15 тестов

## Phase 2: Параметры функций

**Приоритет:** Высокий
**Сложность:** Низкая
**Оценка:** 2-3 часа

### Проблема

Параметры функций и процедур не регистрируются в `symbol_table`, что приводит к ложноположительным ошибкам.

### Затрагиваемые файлы

| Файл | Изменение |
|------|-----------|
| `backend/src/application/ast_to_ir.rs` | Регистрация параметров в `symbol_table` |
| `shared/src/ir/mod.rs` | Возможно расширение `Scope` структуры |

### План реализации

1. Найти обработку `FunctionDeclaration` / `ProcedureDeclaration` в `ast_to_ir.rs`
2. Извлечь список параметров из AST узла
3. Перед обходом тела функции зарегистрировать каждый параметр в `symbol_table`
4. Тип параметра: `TypeResolution::unknown()` (gradual typing) или из аннотации если есть

### Тесты для активации

- `test_function_parameter_is_declared`
- `test_procedure_parameter_is_declared`

---

## Phase 3: BSL Function Scope

**Приоритет:** Средний
**Сложность:** Средняя
**Оценка:** 4-6 часов

### Проблема

В BSL переменные видны во всей функции (function scope), независимо от места объявления. Текущая реализация использует block scope.

### Семантика BSL

- Переменная, объявленная внутри `Если`/`Для`/`Пока`, видна после выхода из блока
- Переменная, объявленная в любом месте функции, видна везде в этой функции
- Это отличается от большинства современных языков (JavaScript var vs let)

### Затрагиваемые файлы

| Файл | Изменение |
|------|-----------|
| `shared/src/ir/mod.rs` | Изменение логики `SymbolTable` |
| `backend/src/application/ast_to_ir.rs` | Изменение управления scope |

### План реализации

1. Изменить `SymbolTable.add_variable()` — регистрировать в function scope, не в текущем block scope
2. Альтернатива: двухпроходный анализ — сначала собрать все переменные, потом проверять
3. Учесть особенности: переменные модуля, экспортируемые переменные

### Тесты для активации

- `test_variable_declared_in_if_branch`
- `test_variable_declared_after_usage_in_same_function`

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
