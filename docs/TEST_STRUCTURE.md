# Структура тестов BSL Gradual Type System

После рефакторинга проект имеет чёткую структуру для тестов и примеров.

## 📁 Структура директорий

```
bsl-gradual-types/
├── src/
│   └── bin/                    # Только реальные исполняемые файлы
│       ├── analyzer.rs          # BSL анализатор
│       ├── lsp_server.rs        # LSP сервер
│       ├── build_index.rs       # Построение индекса типов
│       └── type_check.rs        # Проверка типов
│
├── examples/                    # Примеры использования
│   ├── query_demo.rs           # Демонстрация парсера запросов
│   ├── queries/                # Примеры запросов 1С
│   │   ├── simple_queries.txt
│   │   └── complex_queries.txt
│   ├── bsl/                    # Примеры BSL кода
│   └── configs/                # Примеры конфигураций
│
├── tests/
│   ├── integration/            # Интеграционные тесты
│   │   ├── query_parser_test.rs
│   │   ├── parser/
│   │   ├── query/
│   │   └── lsp/
│   │
│   └── fixtures/               # Тестовые данные
│       ├── bsl/                # BSL файлы для тестов
│       │   ├── simple_test.bsl
│       │   ├── test_example.bsl
│       │   └── test_query_example.bsl
│       ├── xml/                # XML конфигурации
│       │   ├── Catalogs/
│       │   └── Documents/
│       └── queries/            # Тестовые запросы
```

## 🚀 Как использовать

### Запуск тестов
```bash
# Все тесты
cargo test

# Только интеграционные тесты
cargo test --test "*"

# Конкретный тест
cargo test --test query_parser_test

# Юнит-тесты в библиотеке
cargo test --lib
```

### Запуск примеров
```bash
# Демонстрация парсера запросов
cargo run --example query_demo

# Список всех примеров
cargo run --example
```

### Запуск бинарников
```bash
# LSP сервер
cargo run --bin lsp-server

# Анализатор BSL
cargo run --bin bsl-analyzer -- --file path/to/file.bsl

# Построение индекса типов
cargo run --bin build-index -- --config path/to/config
```

## ✅ Преимущества новой структуры

1. **Чёткое разделение** - бинарники, примеры и тесты не смешиваются
2. **Легко найти** - понятно где искать нужный файл
3. **Cargo-friendly** - стандартная структура для Rust проектов
4. **CI/CD готово** - легко настроить автоматическое тестирование
5. **Масштабируемость** - легко добавлять новые тесты и примеры

## 📝 Добавление новых тестов

### Интеграционный тест
Создайте файл в `tests/integration/`:
```rust
// tests/integration/my_test.rs
use bsl_gradual_types::*;

#[test]
fn test_something() {
    // ваш тест
}
```

### Пример использования
Создайте файл в `examples/`:
```rust
// examples/my_example.rs
use bsl_gradual_types::*;

fn main() {
    // ваш пример
}
```

Добавьте в Cargo.toml:
```toml
[[example]]
name = "my_example"
path = "examples/my_example.rs"
```

### Тестовые данные
Поместите в соответствующую директорию в `tests/fixtures/`:
- BSL код → `tests/fixtures/bsl/`
- XML конфигурации → `tests/fixtures/xml/`
- Запросы → `tests/fixtures/queries/`