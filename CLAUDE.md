# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Архитектура проекта

BSL Gradual Type System - система градуальной типизации для языка 1С:Предприятие с Rust workspace структурой:

### Workspace структура
- **shared/** - общие типы и доменная логика (bsl-shared)
- **backend/** - серверная часть с LSP и web API (bsl-backend)
- **frontend/** - веб-интерфейс на Leptos (bsl-frontend)
- **cli/** - CLI инструменты (bsl-cli)
- **vscode-extension/** - VSCode расширение с TypeScript
- **src/** - корневые модули для совместимости с тестами

### Ключевые компоненты
- **Domain layer**: типы, анализ, резолверы в shared/
- **Parsing**: tree-sitter адаптер для BSL парсинга
- **LSP сервер**: продвинутый языковой сервер с flow-sensitive анализом
- **Web сервер**: API для анализа типов и веб-интерфейс
- **Configuration-guided Discovery**: автоматический парсинг конфигураций 1С

## Команды разработки

### Сборка
```bash
# Полная сборка workspace
cargo build --release

# Сборка отдельного компонента
cargo build -p bsl-backend --release
cargo build -p bsl-frontend --release
cargo build -p bsl-cli --release
```

### Тестирование
```bash
# Все тесты
cargo test

# Конкретные тесты
cargo test --test config_parser_guided_test
cargo run --example test_simple

# Performance тесты
cargo run --bin bsl-profiler benchmark --iterations 10
```

### Линтинг и форматирование
```bash
cargo fmt      # Форматирование кода
cargo clippy   # Статический анализ
```

### Запуск компонентов

#### LSP сервер
```bash
cargo run --bin bsl-lsp-server
```

#### Web сервер
```bash
cargo run --bin bsl-web-server -- --port 8080
```

#### CLI инструменты
```bash
# Проверка типов
cargo run --bin bsl-type-check -- "Справочники.Контрагенты"
cargo run --bin bsl-type-check -- --complete "Справочники."

# Профилирование
cargo run --bin bsl-profiler benchmark
cargo run --bin bsl-profiler project /path/to/1c --threads 4

# Анализ файлов
cargo run --bin bsl-analyzer -- --file module.bsl
```

### VSCode Extension
```bash
cd vscode-extension

# Установка зависимостей и компиляция
npm install
npm run compile

# Упаковка и установка
npm install -g vsce
vsce package
code --install-extension bsl-gradual-types-1.0.0.vsix

# Тестирование
npm test
npm run lint  # TypeScript проверка
```

### Frontend (Leptos WASM)
```bash
cd frontend

# Установка trunk для WASM сборки
cargo install trunk

# Разработка с hot reload
trunk serve

# Продакшн сборка
trunk build --release
```

## Configuration-guided Discovery

Новый компонент для автоматического парсинга конфигураций 1С:

```bash
# Быстрый тест
cargo run --example test_simple

# Unit-тесты
cargo test --test config_parser_guided_test

# Использование в коде
use bsl_gradual_types::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;
```

## Важные особенности

### Performance profiles
- `dev` - быстрая компиляция для разработки
- `dev-fast` - оптимизированная разработка (opt-level = 1)  
- `release` - полная оптимизация с LTO

### Features
- `web-ui` - включение веб-интерфейса (по умолчанию)
- `lsp-only` - только LSP без веб-компонентов

### Кеширование
Система использует `.bsl_cache/` для кеширования результатов анализа между сессиями.

## Анализ кода

### ast-grep для поиска и анализа
```bash
# Подсчет структур/enum/impl в проекте
ast-grep run -p "struct " -l rust . | wc -l
ast-grep run -p "enum " -l rust . | wc -l  
ast-grep run -p "impl " -l rust . | wc -l

# Поиск архитектурных компонентов
ast-grep run -p "SystemCoordinator\|TypeSystemService\|ParserCoordinator" -l rust .

# Анализ доменных типов
ast-grep run -p "enum" -l rust shared/src/domain/

# Поиск паттернов использования
ast-grep run -p "pub fn" -l rust . | head -20
```

**Рекомендации по использованию ast-grep:**
- Используйте для быстрой статистики и обзора структуры проекта
- Комбинируйте с grep и чтением файлов для глубокого анализа  
- Простые текстовые паттерны работают надежнее сложных AST-паттернов
- Отлично подходит для поиска архитектурных компонентов и подсчета элементов кода

## Полезные примеры

### Анализ производительности
```bash
# Benchmark конкретного компонента
cargo run --bin bsl-profiler benchmark

# Анализ проекта с параллелизмом
cargo run --bin bsl-profiler project /path/to/1c --threads 4
```

### Тестирование парсеров
```bash
# Configuration-guided discovery
cargo run --example test_simple
cargo test --test config_parser_guided_test

# Другие примеры
cargo run --example syntax_helper_parser_demo
```

### Web API тестирование
```bash
# Запуск сервера
cargo run --bin bsl-web-server --port 8080

# Тестирование API
curl "http://localhost:8080/api/types?search=Массив"
curl "http://localhost:8080/api/health"
curl -X POST "http://localhost:8080/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"code": "Функция Тест() Возврат 42; КонецФункции"}'
```