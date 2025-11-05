# Проверка реализации SignatureHelp для BSL LSP Сервера

**Дата:** 2025-11-05
**Версия проекта:** 0.4.0
**Компонент:** LSP SignatureHelp (Milestone 2.20)
**Статус:** ⚠️ ТРЕБУЕТ ИСПРАВЛЕНИЯ CRITICAL BUGS

---

## Содержание

1. [Результаты компиляции](#результаты-компиляции)
2. [Результаты тестирования](#результаты-тестирования)
3. [Анализ реализованной функциональности](#анализ-реализованной-функциональности)
4. [Обнаруженные баги](#обнаруженные-баги)
5. [Готовность к production](#готовность-к-production)
6. [План исправления](#план-исправления)

---

## Результаты компиляции

### ✅ Успешная компиляция

```bash
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.25s
```

**Проверено:**
- ✅ Все workspace пакеты компилируются без ошибок
- ✅ LSP сервер компилируется (backend/src/bin/lsp_server.rs)
- ✅ Shared компонент с SignatureIndex компилируется
- ✅ Нет warnings связанных с новым кодом

**Вывод:** Код скомпилирован успешно. Синтаксис и типы корректны.

---

## Результаты тестирования

### Обзор

| Компонент | Тесты | Passed | Failed | Статус |
|-----------|-------|--------|--------|--------|
| shared::signature_index | 3 | 3 | 0 | ✅ OK |
| backend::lsp_signature_help | 25 | 10 | 15 | ❌ FAIL |
| backend::other tests | 77 | 77 | 0 | ✅ OK |
| **Total** | **105** | **90** | **15** | ⚠️ PARTIAL |

### Детальные результаты

#### ✅ Passed Tests (10 тестов)

**SignatureIndex функциональность (все работают):**

```
✅ test_signature_index_add_and_find
   - Проверяет добавление и поиск методов в типах
   - Правильно находит сигнатуру "Добавить" в типе "Массив"

✅ test_signature_index_case_insensitive_search
   - Проверяет case-insensitive поиск (кириллица)
   - Находит методы при разных регистрах: "добавить", "ДОБАВИТЬ", "Добавить"

✅ test_signature_index_global_function
   - Проверяет добавление и поиск глобальных функций
   - Правильно находит "Сообщить"

✅ test_signature_index_global_function_case_insensitive
   - Case-insensitive поиск глобальных функций
   - Находит при разных регистрах

✅ test_extract_function_name_global_function
   - Извлечение имён глобальных функций из текста перед скобкой
   - Правильно парсит "Сообщить"

✅ test_extract_function_name_method_call
   - Извлечение имён методов из "Объект.Метод"
   - Правильно парсит "Метод" из "МойОбъект.Метод"

✅ test_extract_function_name_with_spaces
   - Обработка пробелов при извлечении имён
   - Корректно trim'ит пробелы

✅ test_extract_function_name_case_insensitive
   - Сохранение регистра при извлечении
   - Возвращает исходный регистр из текста

✅ test_edge_case_empty_code
   - Обработка пустого кода
   - Возвращает None правильно

✅ test_find_call_context_no_call
   - Обработка кода без вызовов функций
   - Возвращает None правильно
```

**Вывод:** Все компоненты SignatureIndex работают отлично. Проблема только в функциях, работающих с Position индексами.

#### ❌ Failed Tests (15 тестов)

**Все ошибки связаны с обработкой UTF-8 индексов при кириллице:**

```
❌ test_find_call_context_simple_function
   Error: assertion `left == right` failed
   left: "Сооб"
   right: "Сообщить"
   Причина: find_call_context использует byte индексы вместо char индексов

❌ test_find_call_context_with_parameters
   Error: assertion `left == right` failed
   left: "Сооб"
   right: "Сообщить"
   Причина: то же самое

❌ test_find_call_context_method_call
   Error: byte index 15 is not a char boundary; it is inside 'к' (bytes 14..16)
   Причина: индекс выходит на середину кириллического символа

❌ test_find_call_context_nested_calls
   Error: byte index 17 is not a char boundary
   Причина: byte/char index mismatch

❌ test_find_call_context_multiline
   Error: assertion failed: context.is_some()
   Причина: multiline parsing bug

❌ test_calculate_active_parameter_first_param
   Error: byte index 7 is not a char boundary; it is inside 'к' (bytes 6..8)
   Причина: byte/char index mismatch

❌ test_calculate_active_parameter_second_param
   Error: byte index 7 is not a char boundary

❌ test_calculate_active_parameter_third_param
   Error: byte index 7 is not a char boundary

❌ test_calculate_active_parameter_with_string_containing_comma
   Error: byte index 7 is not a char boundary

❌ test_calculate_active_parameter_with_nested_call
   Error: byte index 7 is not a char boundary

❌ test_edge_case_mismatched_parens
   Error: byte index 17 is not a char boundary

❌ test_edge_case_unicode_function_names
   Error: assertion `left == right` failed
   left: "МойМ"
   right: "МойМетод"
   Причина: char index bug

❌ test_edge_case_method_with_underscores
   Error: byte index 16 is not a char boundary

❌ test_edge_case_very_long_parameter_list
   Error: byte index 7 is not a char boundary

❌ test_edge_case_string_with_escaped_quotes
   Error: byte index 7 is not a char boundary
```

**Вывод:** 100% ошибок связаны с одной проблемой - неправильным использованием byte vs char индексов при работе с кириллицей.

---

## Анализ реализованной функциональности

### 📋 Что было реализовано

#### 1. **LSP ServerCapabilities** ✅

Файл: `backend/src/bin/lsp_server.rs` (строки 346-358)

```rust
signature_help_provider: Some(SignatureHelpOptions {
    trigger_characters: Some(vec![
        "(".to_string(),
        ",".to_string(),
    ]),
    retrigger_characters: Some(vec![
        ")".to_string(),
    ]),
    work_done_progress_options: WorkDoneProgressOptions {
        work_done_progress: Some(false),
    },
}),
```

**Статус:** ✅ Правильно реализовано
- Trigger characters установлены ("(", ",")
- Retrigger characters установлены (")")
- Work done progress отключен

#### 2. **LSP Handler** ✅

Файл: `backend/src/bin/lsp_server.rs` (строки 1159-1205)

```rust
async fn signature_help(
    &self,
    params: SignatureHelpParams,
) -> JsonRpcResult<Option<SignatureHelp>>
```

**Статус:** ✅ Правильно реализовано
- Получает document content
- Находит call context
- Получает сигнатуру функции
- Вычисляет активный параметр
- Формирует SignatureHelp response

**Логика потока:**
```
SignatureHelpParams
    ↓
get_document_content() → file_content
    ↓
find_call_context() → CallContext (BUGGY - UTF-8 issue)
    ↓
get_signature_for_function() → MethodSignature
    ↓
calculate_active_parameter() → u32 (BUGGY - UTF-8 issue)
    ↓
build_signature_help_response() → SignatureHelp
```

#### 3. **Helper Functions** ⚠️

##### 3a. find_call_context() (строки 2443-2499)

**Статус:** ⚠️ **CRITICAL BUG - UTF-8 handling**

Логика:
- Движется назад от курсора ищет открывающую скобку "("
- Отслеживает глубину вложенности скобок
- Извлекает имя функции перед скобкой

**Проблема:**
```rust
let end_char = if line_idx == position.line as usize {
    position.character as usize  // ← BUG: tower_lsp использует BYTE indices!
} else {
    line.len()
};

// Позже:
line[..end_char]  // ← Panics если end_char попадает в середину UTF-8 char
```

**Пример:**
```
Код: "Сообщить("
Байты: [С О О б щ и т ь (]
       [0 2 4 6 8 10 12 14 16 17]  ← byte indices

Position.character = 17 (правильный byte index)
Но когда мы берём substring, мы должны работать с character indices!
```

##### 3b. calculate_active_parameter() (строки 2520-2556)

**Статус:** ⚠️ **CRITICAL BUG - UTF-8 handling**

Логика:
- Считает запятые между начало вызова и позицией курсора
- Отслеживает строки и глубину скобок

**Проблема:** То же самое - неправильное использование byte indices

##### 3c. extract_function_name() (строки 2499-2512)

**Статус:** ✅ **OK**

Логика:
- Ищет последнюю точку в тексте (метод вызова)
- Если точка есть - извлекает name после точки
- Если точки нет - это глобальная функция

**Работает правильно** потому что не работает с Position/индексами напрямую.

##### 3d. build_signature_help_response() (строки 2568-2618)

**Статус:** ✅ **OK**

Логика:
- Форматирует label: "ФункцияНаименование(Параметр: Тип)"
- Опциональные параметры в [квадратных скобках]
- Создаёт ParameterInformation для каждого параметра

**Работает правильно** потому что работает только с MethodSignature, не с индексами.

#### 4. **SignatureIndex** ✅

Файл: `shared/src/domain/signature_index.rs` (243 строки)

**Статус:** ✅ **Полностью функционально**

Компоненты:
- `SignatureIndex` - хранилище сигнатур
- `MethodSignature` - структура сигнатуры метода/функции
- `SignatureSource` - enum источников сигнатур (Platform, Configuration, UserCode)
- `SignatureValidationResult` - результаты валидации

**Методы:**
- ✅ `find_method(type_name, method_name)` - case-insensitive поиск
- ✅ `find_global_function(name)` - case-insensitive поиск
- ✅ `add_platform_method()` - добавление методов типов
- ✅ `add_global_function()` - добавление глобальных функций
- ✅ `validate_signature()` - валидация сигнатур

**Встроенные тесты:** 3/3 passed ✅

#### 5. **Интеграция с TypeRepository** ✅

Файл: `shared/src/domain/repository.rs` (строки 120, 282-292)

```rust
fn find_method_signature(
    &self,
    owner_type: Option<&str>,
    method_name: &str,
) -> Option<crate::domain::signature_index::MethodSignature>
```

**Статус:** ✅ **Правильно реализовано**

Логика:
- Получает доступ к SignatureIndex
- Для типизированных методов: ищет в find_method()
- Для глобальных функций: ищет в find_global_function()

---

## Обнаруженные баги

### 🔴 Critical Bug #1: Byte vs Char Index Mismatch

**Серьезность:** CRITICAL
**Затрагивает:** find_call_context(), calculate_active_parameter()
**Проблемные языки:** Кириллица и все языки с multi-byte UTF-8 символами

#### Описание проблемы

`tower_lsp` использует **byte-based** индексы в `Position`:
- `Position.character` = byte offset в строке

Но Rust string slicing работает с **char-based** или **byte-based**, в зависимости от контекста:
- `str[..end_char]` работает с **byte indices**
- `str.chars().take(n)` работает с **char indices**

Текущая реализация путает эти два понятия.

#### Пример проблемы

```
Строка: "Сообщить("
Символы:    С о о б щ и т ь (
Индексы:    0 1 2 3 4 5 6 7 8 (char indices)

Байты в UTF-8: С о о б щ и т ь (
               [0-1] [2-3] [4-5] [6-7] [8-9] [10-11] [12-13] [14-15] [16] (byte indices)

tower_lsp Position.character = 17 (это BYTE index, конец строки)

Проблема:
let end_char = position.character as usize;  // = 17
let substr = &line[..end_char];  // OK, это работает с BYTE индексом
let function_name = extract_function_name(substr);
// function_name = "Сообщить" ✅ (пока OK)

Но когда мы передаём Position в find_call_context:
let position = Position {
    line: 0,
    character: code.len() as u32  // ← BUG!
};

code.len() = 17 (BYTES), но это не то же самое что code.chars().count()
```

#### Решение

Нужны вспомогательные функции:

```rust
fn char_to_byte_index(s: &str, char_idx: u32) -> usize {
    s.chars()
        .take(char_idx as usize)
        .map(|c| c.len_utf8())
        .sum()
}

fn byte_to_char_index(s: &str, byte_idx: usize) -> u32 {
    s[..byte_idx].chars().count() as u32
}
```

### 🟡 Issue #2: Multiline Support

**Серьезность:** MEDIUM
**Затрагивает:** find_call_context() при многострочных вызовах
**Статус:** Неясно, связано ли с byte/char issue или отдельный bug

Тест `test_find_call_context_multiline` падает:
```rust
let code = "Сообщить(\n  \"текст\"\n";  // 3 строки
let position = Position {
    line: 2,
    character: 0,
};
```

**Гипотеза:** Логика поиска скобки работает неправильно при crossing multiple lines.

### 🟡 Issue #3: No Type Inference for Receiver

**Серьезность:** MEDIUM
**Затрагивает:** find_call_context() - receiver_type всегда None
**Статус:** По дизайну (пока не реализовано)

```rust
let (function_name, receiver_type) = self.extract_function_name(before_paren)?;
// receiver_type = None всегда
```

**Нужно:** Type inference через SystemCoordinator/AnalysisEngine

### 🟡 Issue #4: No Parameter Documentation

**Серьезность:** LOW
**Затрагивает:** build_signature_help_response()
**Статус:** По дизайну (не реализовано)

ParameterInformation может содержать `documentation: Option<MarkupContent>`, но сейчас всегда None.

---

## Готовность к production

### ❌ НЕ ГОТОВО

**Причины:**

1. **Critical UTF-8 bug** - функция полностью сломана при работе с кириллицей (русский язык)
2. **15 unit тестов fail** - 60% успешных тестов недостаточно
3. **Основная целевая аудитория - Russian developers** - а код не работает с русским

### Оценка компонентов

| Компонент | Оценка | Примечания |
|-----------|--------|-----------|
| SignatureIndex | ✅ 100% | Полностью готово |
| extract_function_name() | ✅ 100% | Работает правильно |
| build_signature_help_response() | ✅ 95% | Нужна документация параметров |
| find_call_context() | ❌ 0% | Critical UTF-8 bug |
| calculate_active_parameter() | ❌ 0% | Critical UTF-8 bug |
| get_signature_for_function() | ✅ 100% | Работает через TypeRepository |
| LSP ServerCapabilities | ✅ 100% | Правильно настроено |
| LSP Handler | ⚠️ 50% | Handler работает, но helper functions buggy |

### Итоговая оценка

**Архитектура:** ✅ Правильная, хорошо интегрирована с LSP
**Функциональность:** ❌ Сломана при работе с кириллицей
**Тестирование:** ⚠️ Тесты выявили проблемы (это хорошо!)
**Готовность к использованию:** ❌ НЕ ГОТОВО

---

## План исправления

### Phase 1: Исправить UTF-8 bug (CRITICAL)

**Время:** ~2 часа
**Файл:** backend/src/bin/lsp_server.rs

Шаги:
1. Добавить вспомогательные функции char↔byte преобразование
2. Обновить find_call_context() использовать правильные индексы
3. Обновить calculate_active_parameter() использовать правильные индексы
4. Перезапустить unit тесты (целевой результат: 25/25 passed)

```rust
fn char_to_byte_index(s: &str, char_idx: u32) -> usize {
    s.chars()
        .take(char_idx as usize)
        .map(|c| c.len_utf8())
        .sum()
}

fn byte_to_char_index(s: &str, byte_idx: usize) -> u32 {
    s[..byte_idx].chars().count() as u32
}
```

### Phase 2: Добавить type inference для receiver_type (MEDIUM)

**Время:** ~4 часа
**Файл:** backend/src/bin/lsp_server.rs

Шаги:
1. Добавить метод для type inference в BslLanguageServer
2. Использовать SystemCoordinator.get_analysis_engine()
3. Вызвать type inference для выражения перед точкой
4. Обновить find_call_context() возвращать правильный receiver_type
5. Добавить тесты для type inference

### Phase 3: Добавить documentation параметров (LOW)

**Время:** ~3 часа
**Файл:** backend/src/bin/lsp_server.rs

Шаги:
1. Добавить документацию в ParameterInfo структуру
2. Загрузить описания параметров из syntax_helper
3. Поддержать конфигурационные типы
4. Обновить build_signature_help_response() заполнять documentation

### Phase 4: Покрыть edge cases (LOW)

**Время:** ~2 часа
**Файл:** backend/tests/lsp_signature_help_test.rs

Шаги:
1. Добавить тесты для escaped quotes в строках
2. Добавить тесты для комментариев
3. Добавить тесты для очень глубокой вложенности
4. Добавить regression тесты

### Phase 5: Performance optimization (OPTIONAL)

**Время:** ~2 часа

Шаги:
1. Кэширование результатов type inference
2. Оптимизация поиска в SignatureIndex (например, HashMap вместо Vec)
3. Benchmark на больших файлах (1000+ строк)

---

## Детальный код обзор

### CallContext структура

Файл: `backend/src/bin/lsp_server.rs` (не в shared, локальная в binary)

```rust
#[derive(Debug)]
struct CallContext {
    function_name: String,
    receiver_type: Option<String>,
    call_start: Position,
}
```

**Статус:** ✅ Правильная структура
**Возможные улучшения:**
- Добавить receiver_type: Option<String> (пока всегда None)
- Добавить call_end: Position для многострочных вызовов

### MethodSignature структура

Файл: `shared/src/domain/signature_index.rs`

```rust
pub struct MethodSignature {
    pub name: String,
    pub owner_type: Option<String>,
    pub params: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub source: SignatureSource,
}
```

**Статус:** ✅ Правильная структура
**Используется в:**
- TypeRepository (find_method_signature)
- build_signature_help_response

### ParameterInfo структура

Файл: `shared/src/domain/types.rs`

```rust
pub struct ParameterInfo {
    pub name: String,
    pub type_name: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}
```

**Статус:** ✅ Имеет все необходимые поля
**Используется в:** MethodSignature.params

---

## Тестовое покрытие

### Созданные тесты

Файл: `backend/tests/lsp_signature_help_test.rs` (604 строк, 25 тестов)

**Структура:**
- Helper functions (4): find_call_context, extract_function_name, calculate_active_parameter
- find_call_context tests (7): simple, with_params, method, nested, multiline, no_call, after_close_paren
- calculate_active_parameter tests (5): first, second, third, with_string_comma, with_nested_call
- extract_function_name tests (5): global, method, with_spaces, case_insensitive, empty
- SignatureIndex tests (4): add_find, case_insensitive, global_function, global_case_insensitive
- Edge cases (4): empty_code, mismatched_parens, unicode_names, method_with_underscores, long_params, escaped_quotes

### Встроенные тесты в компонентах

**shared/src/domain/signature_index.rs** (3 встроенных теста):
- ✅ test_signature_index_basic
- ✅ test_signature_index_case_insensitive
- ✅ test_signature_index_not_found

---

## Примеры использования (LSP perspective)

### Пример 1: Вызов глобальной функции

Клиент (VSCode) отправляет:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "textDocument/signatureHelp",
  "params": {
    "textDocument": {
      "uri": "file:///workspace/script.bsl"
    },
    "position": {
      "line": 5,
      "character": 17
    }
  }
}
```

Код в файле:
```bsl
Сообщить("Hello", █
```

Сервер должен вернуть:
```json
{
  "signatures": [
    {
      "label": "Сообщить(Текст: Строка)",
      "parameters": [
        {
          "label": "Текст: Строка"
        }
      ],
      "activeParameter": 1
    }
  ],
  "activeSignature": 0,
  "activeParameter": 1
}
```

### Пример 2: Вызов метода объекта

Код:
```bsl
Массив.Добавить(█
```

Сервер должен вернуть:
```json
{
  "signatures": [
    {
      "label": "Добавить(Элемент: Произвольный)",
      "parameters": [
        {
          "label": "Элемент: Произвольный"
        }
      ],
      "activeParameter": 0
    }
  ],
  "activeSignature": 0,
  "activeParameter": 0
}
```

---

## Заключение

### Что хорошо

✅ Архитектура правильная и хорошо интегрирована
✅ SignatureIndex полностью функционален
✅ LSP капабилитиз правильно настроены
✅ Логика потока (flow) правильная
✅ Тесты выявили проблемы (это хорошо!)

### Что нужно исправить

❌ Critical UTF-8 bug в find_call_context и calculate_active_parameter
⚠️ Нет type inference для receiver_type
⚠️ Нет документации параметров
⚠️ Multiline support может быть затронут

### Оценка состояния

```
Архитектура:     ████████░░  90% ✅
Функциональность: ███░░░░░░░  30% ❌
Тестирование:    █████░░░░░  50% ⚠️
Документация:    ██░░░░░░░░  20% ❌
Готовность:      ██░░░░░░░░  20% ❌
```

### Рекомендация

**БЛОКИРУЮЩАЯ ПРОБЛЕМА:** UTF-8 bug делает функцию неопределимой для русского кода.

**Статус:** Требует срочного исправления перед использованием в production.

**Приоритет:** CRITICAL (P0)

---

## Файлы для изменения

| Файл | Строки | Статус | Примечание |
|------|--------|--------|-----------|
| backend/src/bin/lsp_server.rs | 1159-1205 | async fn signature_help() | Handler OK |
| backend/src/bin/lsp_server.rs | 2443-2499 | find_call_context() | 🔴 BUGGY |
| backend/src/bin/lsp_server.rs | 2520-2556 | calculate_active_parameter() | 🔴 BUGGY |
| backend/src/bin/lsp_server.rs | 2568-2618 | build_signature_help_response() | ✅ OK |
| shared/src/domain/signature_index.rs | 1-243 | Весь файл | ✅ OK |
| shared/src/domain/repository.rs | 282-292 | find_method_signature() | ✅ OK |
| backend/tests/lsp_signature_help_test.rs | 1-604 | Новый файл | 📝 Тесты |

---

## История изменений

| Дата | Версия | Автор | Изменение |
|------|--------|-------|-----------|
| 2025-11-05 | 0.1 | QA Engineer | Первая проверка, обнаружен UTF-8 bug |

---

## Чек-лист перед Production

- [ ] Phase 1 completed: UTF-8 bug fixed, 25/25 tests passed
- [ ] Phase 2 completed: Type inference works, receiver_type populated
- [ ] Phase 3 completed: Parameter documentation added
- [ ] Phase 4 completed: Edge cases covered
- [ ] All 105 workspace tests pass
- [ ] Manual testing with real BSL code
- [ ] Performance testing (files > 10000 lines)
- [ ] Integration testing with VSCode extension
- [ ] Documentation updated

---

## Контакты и ссылки

- **Milestone:** 2.20 (Function Signature Validation System)
- **ROADMAP:** ROADMAP_2025.md
- **Документация:** docs/architecture/type_system_architecture.md
- **API Reference:** docs/api/web-api-reference.md

---

**Дата отчёта:** 2025-11-05 13:45 UTC
**Версия проекта:** 0.4.0
**Статус:** ⚠️ Требует исправления
