# План рефакторинга syntax_helper_parser.rs

## Цель
Разбить файл `syntax_helper_parser.rs` (1910 строк) на логические модули для улучшения читаемости и поддерживаемости.

## Текущая структура файла

```
syntax_helper_parser.rs (1910 строк)
├── Импорты (строки 1-26)
├── Структуры данных (строки 28-226)
│   ├── SyntaxNode, CategoryInfo, TypeInfo и т.д.
│   ├── SyntaxHelperDatabase, TypeIndex
│   └── OptimizationSettings
├── SyntaxHelperParser struct (строки 226-248)
├── Основная реализация (строки 250-1770)
│   ├── Публичные методы (new, parse, get_all_types и т.д.)
│   ├── HTML extractors (~20 методов, строки 1188-1484)
│   ├── Вспомогательные методы
│   └── Индексация
└── ParsingStats + FileType (строки 1772-1910)
```

## Целевая структура

```
backend/src/data/loaders/
├── syntax_helper_parser.rs          # Главный оркестратор (~300-400 строк)
└── syntax_helper/
    ├── mod.rs                        # Публичные реэкспорты
    ├── types.rs                      # Все структуры данных (~250 строк)
    ├── html_extractors.rs            # HTML парсинг методы (~400 строк)
    ├── indexing.rs                   # Построение индексов (~200 строк)
    └── utils.rs                      # Вспомогательные функции (~100 строк)
```

## Пошаговый план выполнения

### Шаг 1: Создать структуру модулей ✅
- [x] Создать директорию `backend/src/data/loaders/syntax_helper/`
- [x] Создать `mod.rs` с базовой структурой

### Шаг 2: Выделить types.rs ✅
**Строки для переноса: 28-226**

Содержимое:
```rust
// Все структуры данных
- SyntaxNode enum
- CategoryInfo, TypeInfo, TypeIdentity, TypeDocumentation, TypeStructure, TypeMetadata
- CodeExample, MethodInfo, PropertyInfo, ConstructorInfo, ParameterInfo, GlobalFunctionInfo
- SyntaxHelperDatabase, TypeIndex
- OptimizationSettings (с impl Default)
- ParsingStats
```

Импорты для types.rs:
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bsl_shared::domain::types::FacetKind;
```

### Шаг 3: Выделить html_extractors.rs
**Строки для переноса: 1188-1484 (примерно)**

Методы для переноса:
```rust
- extract_title()
- extract_element_text()
- extract_description()
- extract_examples()
- extract_parameters()
- extract_return_info()
- extract_return_type()
- extract_property_type()
- extract_english_name()
- extract_availability()
- extract_version()
- extract_aliases()
- extract_collection_element()
- extract_methods_from_html()
- extract_properties_from_html()
- extract_constructors_from_html()
- extract_enum_values_from_html() ⭐ ВАЖНО для enum_values
- extract_members_from_section()
- extract_text_after_chapter()
```

Структура:
```rust
pub struct HtmlExtractor;

impl HtmlExtractor {
    pub fn new() -> Self { Self }

    // Все extract методы как pub fn
}
```

Импорты:
```rust
use scraper::{Html, Selector, ElementRef};
use super::types::{CodeExample, ParameterInfo};
use tracing::debug;
```

### Шаг 4: Выделить indexing.rs
**Методы для переноса:**
```rust
- build_indexes()
- build_type_index()
- build_facet_index()
```

Структура:
```rust
pub struct IndexBuilder;

impl IndexBuilder {
    pub fn build_indexes(...) -> TypeIndex { ... }
    fn build_type_index(...) { ... }
    fn build_facet_index(...) { ... }
}
```

### Шаг 5: Выделить utils.rs
**Методы для переноса:**
```rust
- detect_file_type()
- determine_facet_kind()
- build_path()
- extract_category_path()
```

### Шаг 6: Обновить syntax_helper_parser.rs
**Что остаётся:**
```rust
// Импорты
use super::syntax_helper::*;

// SyntaxHelperParser struct
pub struct SyntaxHelperParser {
    nodes: Arc<DashMap<String, SyntaxNode>>,
    html_extractor: HtmlExtractor,
    // ...
}

impl SyntaxHelperParser {
    // Публичные методы: new, parse, get_all_types и т.д.
    // Используют HtmlExtractor и IndexBuilder
}
```

### Шаг 7: Обновить mod.rs в loaders
```rust
pub mod syntax_helper_parser;
pub use syntax_helper_parser::SyntaxHelperParser;

// Реэкспорт основных типов
pub use syntax_helper_parser::syntax_helper::{
    SyntaxNode, TypeInfo, CategoryInfo, // и т.д.
};
```

### Шаг 8: Проверка компиляции
- [ ] `cargo check --release -p bsl-backend`
- [ ] Исправить ошибки импортов
- [ ] Проверить видимость типов (pub/pub(crate))

### Шаг 9: Тестирование
- [ ] Запустить существующие тесты
- [ ] Проверить работу парсера с syntax_helper
- [ ] Протестировать enum_values для УровеньИспользованияЗащищенногоСоединенияFTP

### Шаг 10: Финализация
- [ ] Добавить документацию к модулям
- [ ] Обновить CLAUDE.md с новой структурой
- [ ] Закоммитить изменения

## Критические моменты

1. **Видимость (pub vs pub(crate))**:
   - Все структуры в types.rs должны быть `pub`
   - HtmlExtractor методы должны быть `pub` или `pub(crate)`
   - Вспомогательные функции могут быть приватными

2. **Импорты**:
   - Избегать циклических зависимостей
   - html_extractors.rs не должен импортировать SyntaxHelperParser
   - Все используют types.rs

3. **Arc и DashMap**:
   - SyntaxHelperParser владеет Arc<DashMap>
   - Экстракторы работают с &Html документами

4. **Трейты и реализации**:
   - Default для OptimizationSettings остаётся в types.rs
   - Default для SyntaxHelperParser остаётся в основном файле

## Порядок выполнения (важно!)

1. Создать все файлы модулей с пустыми структурами
2. Скопировать код в модули (НЕ удаляя из оригинала)
3. Добавить все импорты в модулях
4. Обновить syntax_helper_parser.rs для использования модулей
5. Проверить компиляцию
6. Удалить дублированный код из syntax_helper_parser.rs
7. Финальная проверка

## Ожидаемый результат

- `syntax_helper_parser.rs`: ~350 строк (оркестратор)
- `types.rs`: ~250 строк (структуры)
- `html_extractors.rs`: ~400 строк (парсинг HTML)
- `indexing.rs`: ~200 строк (индексация)
- `utils.rs`: ~100 строк (утилиты)

**Итого: ~1300 строк** вместо 1910 (упрощение за счёт удаления дублирования)

## Связь с проблемой enum_values

После рефакторинга будет проще:
1. Добавить debug логирование в `html_extractors.rs`
2. Изолированно тестировать `extract_enum_values_from_html()`
3. Понять почему enum значения не извлекаются
4. Исправить проблему без изменения основного файла

## Проверочный список

- [ ] Все модули созданы
- [ ] types.rs полностью работает
- [ ] html_extractors.rs компилируется
- [ ] indexing.rs компилируется
- [ ] utils.rs компилируется
- [ ] mod.rs правильно реэкспортирует
- [ ] syntax_helper_parser.rs обновлён
- [ ] Проект компилируется без ошибок
- [ ] Тесты проходят
- [ ] enum_values работают корректно
