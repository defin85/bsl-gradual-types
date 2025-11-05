# Отчёт о тестировании SignatureHelp для BSL LSP Сервера

**Дата:** 2025-11-05
**Версия:** 0.4.0
**Компонент:** SignatureHelp (Milestone 2.20)

---

## Резюме

✅ **Компиляция:** Успешно
✅ **Базовая функциональность:** Реализована
⚠️ **Unit тесты:** 10 passed, 15 failed (60% успешных)

---

## Результаты компиляции

```
Compiling bsl-backend v0.4.2
Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.25s
```

**Вывод:** Код компилируется без ошибок.

---

## Результаты Unit Тестов

### Тесты SignatureIndex (10 passed)

Все тесты работают корректно:

✅ `test_signature_index_add_and_find` - Поиск методов в типах работает
✅ `test_signature_index_case_insensitive_search` - Case-insensitive поиск работает для кириллицы
✅ `test_signature_index_global_function` - Поиск глобальных функций работает
✅ `test_signature_index_global_function_case_insensitive` - Case-insensitive поиск глобальных функций работает
✅ `test_extract_function_name_global_function` - Извлечение имён глобальных функций
✅ `test_extract_function_name_method_call` - Извлечение имён методов
✅ `test_extract_function_name_with_spaces` - Обработка пробелов
✅ `test_extract_function_name_case_insensitive` - Сохранение регистра при извлечении
✅ `test_edge_case_empty_code` - Обработка пустого кода
✅ `test_find_call_context_no_call` - Корректное возвращение None когда вызова нет

**Вывод:** Компонент SignatureIndex полностью функционален.

### Критические баги в find_call_context и calculate_active_parameter (15 failed)

#### 🔴 Проблема 1: Неправильная работа с UTF-8 на кириллице

**Сигнатура проблемы:**
```
byte index 7 is not a char boundary; it is inside 'к' (bytes 6..8) of `Функция(`
```

**Причина:**
В `tower_lsp`, Position.character использует **byte индексы**, не character индексы.
Кириллические символы занимают 2+ байта в UTF-8, поэтому простое преобразование `code.len()` as u32 создаёт неправильный byte index.

**Примеры:**
- `"Сообщить("` = 9 bytes, но только 9 characters... но "Сообщ" уже 5 символов × 2 байта = 10 bytes
- Функция преобразует character index в byte index неправильно

**Затронутые тесты:**
- `test_find_call_context_simple_function` - ожидал "Сообщить", получил "Сооб"
- `test_find_call_context_with_parameters` - ожидал "Сообщить", получил "Сооб"
- `test_find_call_context_method_call` - byte boundary error
- `test_find_call_context_nested_calls` - byte boundary error
- `test_edge_case_unicode_function_names` - ожидал "МойМетод", получил "МойМ"
- `test_edge_case_method_with_underscores` - byte boundary error
- Все остальные тесты с calculate_active_parameter

#### 🔴 Проблема 2: Неправильное преобразование character ↔ byte индексов

Для работы с tower_lsp требуется преобразование:
- **character index** (позиция в графемах/символах) → **byte index** (позиция в байтах)
- **byte index** → **character index** (обратное преобразование)

**Текущая реализация:** Использует индексы напрямую без преобразования.

**Пример ошибки:**
```rust
let code = "Функция(";  // 4 символа × 2 байта = 8 байт + 1 байт ( = 9 bytes
let position = Position {
    line: 0,
    character: code.len() as u32,  // 9 bytes, но это не правильный character index!
};
```

#### 🔴 Проблема 3: Multiline test fails

Тест `test_find_call_context_multiline` не находит контекст вызова, потому что:
- Парсер не правильно работает с многострочными вызовами
- Или line_idx выходит за границы массива lines при неправильном line count

---

## Анализ реализованной функциональности

### ✅ Что работает

1. **SignatureIndex (signature_index.rs)**
   - Хранение и поиск сигнатур (case-insensitive, кириллица)
   - Поддержка платформенных и конфигурационных методов
   - Поддержка глобальных функций

2. **LSP Capability**
   - `signature_help_provider` правильно зарегистрирован в ServerCapabilities
   - Trigger characters ("(", ",") установлены
   - Retrigger characters (")") установлены

3. **Extract function name**
   - Правильно извлекает имена глобальных функций
   - Правильно извлекает имена методов (из "Объект.Метод")
   - Сохраняет правильный регистр

### ⚠️ Что нужно исправить

1. **find_call_context** - Работает с byte индексами неправильно
2. **calculate_active_parameter** - Использует неправильные индексы
3. **Преобразование character ↔ byte индексов** - Нужна вспомогательная функция

### ❓ Что нужно доработать

1. **receiver_type determination** - Сейчас всегда None
   - Нужна type inference через TypeSystemService
   - Требуется определить тип переменной/выражения перед точкой

2. **Документация параметров** - Не заполняется
   - SignatureInformation может содержать documentation
   - ParameterInformation может содержать documentation (описание каждого параметра)

3. **Граничные случаи**
   - Строки с escaped quotes (""")
   - Многострочные вызовы
   - Комментарии в коде

---

## Рекомендации по исправлению

### Приоритет 1: Исправить работу с кириллицей

Создать вспомогательные функции для преобразования индексов:

```rust
/// Преобразовать character index в byte index
fn char_to_byte_index(text: &str, char_idx: u32) -> usize {
    text.chars()
        .take(char_idx as usize)
        .map(|c| c.len_utf8())
        .sum()
}

/// Преобразовать byte index в character index
fn byte_to_char_index(text: &str, byte_idx: usize) -> u32 {
    text[..byte_idx]
        .chars()
        .count() as u32
}
```

### Приоритет 2: Добавить type inference для receiver_type

```rust
fn get_receiver_type(&self, expr: &str) -> Option<String> {
    // Попросить type inference у analysis engine
    // Вернуть тип выражения перед точкой
}
```

### Приоритет 3: Добавить документацию параметров

```rust
fn get_param_documentation(&self, method_name: &str, param_idx: usize) -> Option<String> {
    // Загрузить описание параметра из syntax_helper или конфигурации
}
```

---

## Заключение по готовности к Production

### ❌ НЕ готово к Production

**Причины:**
1. **Critical bug с кириллицей** - функция не работает с русскими именами функций
2. **15 из 25 unit тестов fail** - 60% успешных тестов недостаточно
3. **Отсутствует type inference** - receiver_type всегда None

### ✅ Готово для дальнейшей разработки

Архитектура правильная, компонент правильно интегрирован в LSP сервер, но требует исправления critical bugs перед использованием.

---

## План исправления

```
1. [Priority 1] Исправить byte/char index handling для кириллицы
   - Добавить char_to_byte_index и byte_to_char_index вспомогательные функции
   - Обновить find_call_context и calculate_active_parameter
   - Перезапустить unit тесты

2. [Priority 2] Добавить type inference для receiver_type
   - Интегрировать с TypeSystemService для определения типов
   - Обновить find_call_context для получения receiver_type
   - Добавить новые тесты

3. [Priority 3] Добавить документацию параметров
   - Загрузить descriptions из платформенных типов
   - Поддержать конфигурационные типы
   - Добавить в SignatureHelp response

4. [Priority 4] Покрыть edge cases
   - Multiline calls
   - Escaped quotes в строках
   - Comments
   - Граничные случаи

5. [Priority 5] Performance optimization
   - Кэширование результатов type inference
   - Оптимизация поиска в SignatureIndex
```

---

## Файлы для изменения

1. **backend/src/bin/lsp_server.rs**
   - Функция `find_call_context()` - добавить char/byte преобразование
   - Функция `calculate_active_parameter()` - исправить индексы
   - Функция `extract_function_name()` - всё OK

2. **shared/src/domain/signature_index.rs**
   - Уже работает корректно

3. **backend/tests/lsp_signature_help_test.rs**
   - Тесты созданы, но тестируют неправильную реализацию
   - Обновить Position values для правильных byte индексов после исправления

---

## Проверено

- ✅ Компиляция workspace
- ✅ Backend тесты (77 passed в основной suite)
- ✅ Unit тесты для SignatureIndex (10 passed)
- ❌ Unit тесты для find_call_context (15 failed)
- ✅ Интеграция с LSP сервером (код скомпилирован)

---

## Версия отчёта

- **Дата создания:** 2025-11-05 13:30 UTC
- **Версия проекта:** 0.4.0
- **Milestone:** 2.20 (Function Signature Validation System)
