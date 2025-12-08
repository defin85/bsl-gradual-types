# Тесты для проверки необъявленных переменных

## Обзор

Файл `undeclared_variable_test.rs` содержит комплексные тесты для проверки валидации необъявленных переменных в аргументах методов.

## Milestone

**Semantic Validation - Undeclared Variables**

Реализована базовая проверка необъявленных переменных в аргументах вызовов функций.

## Результаты тестирования

### ✅ Проходящие тесты (15/21)

#### Базовые кейсы
- ✅ `test_undeclared_variable_in_method_argument` - Обнаружение необъявленной переменной в аргументе
- ✅ `test_declared_variable_no_error` - Объявленная переменная не вызывает ошибку
- ✅ `test_literal_no_error` - Строковые литералы игнорируются
- ✅ `test_number_literal_no_error` - Числовые литералы игнорируются
- ✅ `test_boolean_literal_no_error` - Булевы литералы игнорируются

#### Edge cases
- ✅ `test_undefined_keyword_no_error` - Ключевое слово Неопределено игнорируется
- ✅ `test_null_keyword_no_error` - Ключевое слово Null игнорируется
- ✅ `test_multiple_undeclared_variables` - Множественные необъявленные переменные обнаруживаются
- ✅ `test_mixed_declared_and_undeclared` - Смешанные объявленные/необъявленные переменные

#### Глобальные коллекции метаданных
- ✅ `test_global_collection_catalogs_no_error` - Справочники игнорируется
- ✅ `test_global_collection_documents_no_error` - Документы игнорируется
- ✅ `test_global_collection_enums_no_error` - Перечисления игнорируется
- ✅ `test_global_collection_in_argument` - Глобальные коллекции в аргументах игнорируются

#### Scope и контекст
- ✅ `test_loop_counter_is_declared` - Счетчик цикла считается объявленным

#### Реальные сценарии
- ✅ `test_real_scenario_with_typo` - Обнаружение опечатки в имени переменной

---

### 🚧 Игнорируемые тесты (6/21) - Требуют улучшения scope tracking

#### 1. `test_function_parameter_is_declared`
**Причина:** Параметры функций не регистрируются в symbol_table при AST-to-IR конвертации

**Ожидаемое поведение:**
```1c
Процедура Тест(ПараметрВходной)
    М.НайтиПоКоду(ПараметрВходной);  // ПараметрВходной должен быть объявлен
КонецПроцедуры
```

**Текущее поведение:** Параметр помечается как необъявленная переменная

**Решение:** В `ast_to_ir.rs` при обработке `FunctionDeclaration`/`ProcedureDeclaration` нужно добавлять параметры в symbol_table текущего scope.

---

#### 2. `test_nested_method_calls_with_undeclared`
**Причина:** Member access на необъявленной переменной не отлавливается, т.к. property access обрабатывается иначе

**Ожидаемое поведение:**
```1c
М.Добавить(необъявленная.Свойство);  // необъявленная должна вызвать ошибку
```

**Текущее поведение:** Ошибка не генерируется для base объекта в property access

**Решение:** В `ast_to_ir.rs` в обработке `PropertyAccess` проверять, что base объект не является необъявленной переменной.

---

#### 3. `test_undeclared_in_chain_call`
**Причина:** Цепочки вызовов на необъявленных переменных не отлавливаются

**Ожидаемое поведение:**
```1c
Результат = необъявленная.Метод1().Метод2();  // необъявленная должна вызвать ошибку
```

**Текущее поведение:** Аналогично предыдущему - property/method access не проверяет base объект

---

#### 4. `test_variable_declared_in_if_branch`
**Причина:** BSL не имеет block scope, но текущая реализация использует вложенные scopes

**Ожидаемое поведение (правильное для BSL):**
```1c
Процедура Тест()
    Если Условие Тогда
        ЛокальнаяПеременная = "значение";
    КонецЕсли;

    М.Добавить(ЛокальнаяПеременная);  // Должна быть доступна
КонецПроцедуры
```

**Текущее поведение:** Переменная доступна только внутри if-блока

**Решение:** В BSL все переменные в пределах функции/процедуры имеют function scope, а не block scope. Нужно изменить стратегию создания scopes - использовать один scope для всей функции.

---

#### 5. `test_variable_declared_in_loop`
**Причина:** Аналогично - block scope вместо function scope

**Решение:** То же самое - function-level scope для всех переменных.

---

#### 6. `test_real_scenario_catalog_search`
**Причина:** Параметр КодПоиска не регистрируется в symbol_table

**Решение:** Исправится вместе с тестом #1.

---

## Архитектурные ограничения

### 1. Symbol Table и Scope Management

**Проблема:** Текущая реализация создает вложенные scopes для if/loop/etc, что не соответствует семантике BSL.

**BSL семантика:**
- Переменные видны во всей функции/процедуре (function scope)
- Нет block scope (как в JavaScript ES5)
- Параметры функции - часть function scope

**Текущая реализация:**
- Создает отдельные scopes для блоков
- Параметры НЕ добавляются в symbol_table

**Необходимые изменения:**
1. В `ast_to_ir.rs`:
   - При создании `FunctionDeclaration`/`ProcedureDeclaration` добавлять параметры в `body_scope`
   - Использовать единый scope для всего тела функции (не создавать вложенные scopes для if/loop)

2. В `symbol_table.rs` (если есть):
   - Обеспечить правильную видимость переменных в function scope

### 2. Property Access и проверка base объекта

**Проблема:** При обработке `object.property` или `object.method()` не проверяется, объявлен ли `object`.

**Решение:**
В `ast_to_ir.rs` в `infer_type_resolution` для `Expression::PropertyAccess`:
```rust
Expression::PropertyAccess { object, property, .. } => {
    let base = self.infer_type_resolution(object);

    // НОВОЕ: Проверяем, что base не является необъявленной переменной
    if base.is_undeclared_variable().is_some() {
        return base;  // Пробрасываем undeclared дальше
    }

    // Существующая логика...
}
```

---

## Запуск тестов

### Все тесты (включая ignored)
```bash
cargo test -p bsl-backend --test undeclared_variable_test -- --include-ignored
```

### Только активные (проходящие)
```bash
cargo test -p bsl-backend --test undeclared_variable_test
```

### Конкретный тест
```bash
cargo test -p bsl-backend --test undeclared_variable_test test_undeclared_variable_in_method_argument
```

### Игнорируемые тесты
```bash
cargo test -p bsl-backend --test undeclared_variable_test -- --ignored
```

---

## Метрики покрытия

| Категория | Всего | Проходит | Игнорируется | Покрытие |
|-----------|-------|----------|--------------|----------|
| Базовые кейсы | 5 | 4 | 1 | 80% |
| Edge cases | 4 | 4 | 0 | 100% |
| Глобальные коллекции | 4 | 4 | 0 | 100% |
| Вложенные вызовы | 2 | 0 | 2 | 0% |
| Scope и контекст | 3 | 1 | 2 | 33% |
| Реальные сценарии | 2 | 1 | 1 | 50% |
| **ИТОГО** | **21** | **15** | **6** | **71%** |

---

## Roadmap для полного покрытия

### Phase 1: Параметры функций (высокий приоритет)
- [ ] Регистрация параметров в symbol_table
- [ ] Тесты: `test_function_parameter_is_declared`, `test_real_scenario_catalog_search`

### Phase 2: Function Scope (средний приоритет)
- [ ] Единый scope для всей функции (не block scope)
- [ ] Тесты: `test_variable_declared_in_if_branch`, `test_variable_declared_in_loop`

### Phase 3: Property Access validation (средний приоритет)
- [ ] Проверка base объекта в property/method access
- [ ] Тесты: `test_nested_method_calls_with_undeclared`, `test_undeclared_in_chain_call`

---

## Примечания

### Успешная реализация

✅ **Что работает хорошо:**
- Обнаружение необъявленных переменных в аргументах функций
- Игнорирование литералов (строки, числа, булевы значения)
- Игнорирование ключевых слов (Неопределено, Null)
- Игнорирование глобальных коллекций метаданных (Справочники, Документы, etc)
- Множественные необъявленные переменные в одном вызове

### Известные ограничения

⚠️ **Что требует доработки:**
- Параметры функций считаются необъявленными
- Block scope вместо function scope
- Property/method access не проверяет base объект
- Счетчики циклов (Для Индекс = ...) требуют специальной обработки

---

## Связанные файлы

- **Реализация:** `backend/src/application/semantic_validation_visitor.rs`
- **AST-to-IR:** `backend/src/application/ast_to_ir.rs`
- **Валидаторы:** `shared/src/domain/validators.rs`
- **Типы:** `shared/src/domain/types.rs` (UncertaintyReason::UndeclaredVariable)

---

**Версия:** 1.0
**Дата создания:** 2025-12-08
**Автор:** BSL Gradual Types Team
