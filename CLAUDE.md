# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Архитектура проекта

BSL Gradual Type System - система градуальной типизации для языка 1С:Предприятие с Right-Sized Architecture философией:

**Философия**: Start simple, scale up по необходимости (6-8 компонентов вместо 25-30).

### Workspace структура
- **shared/** - общие типы, доменная логика и AnalysisEngine (bsl-shared)
- **backend/** - серверная часть с LSP, web API и SystemCoordinator (bsl-backend)
- **frontend/** - веб-интерфейс на Leptos WASM (bsl-frontend)
- **cli/** - CLI инструменты (bsl-cli)
- **vscode-extension/** - VSCode расширение с TypeScript

### Ключевые компоненты упрощенной архитектуры

#### System Layer (в backend)
- **SystemCoordinator** - единая точка координации и DI management
- **AnalysisCache** - простое LRU кеширование в памяти с TTL
- **ParserCoordinator** - TreeSitter (основной) + Regex (fallback)
- **BasicObservability** - структурированное логирование и базовые метрики

#### Application Layer
- **AnalysisEngine** (в shared) - чистый оркестратор анализа без зависимостей от backend
- **TypeSystemService** (в backend) - высокоуровневый API для Web/LSP, использует AnalysisEngine

#### Domain Layer (в shared)
- **TypeResolver** - центральная логика анализа типов с flow-sensitive анализом
- **TypeRepository** - абстракция для работы с данными

### Центральная абстракция: TypeResolution
```rust
struct TypeResolution {
    certainty: Certainty,        // Known | Inferred(0.0-1.0) | Unknown
    result: ResolutionResult,    // Concrete | Union | Dynamic
    active_facet: FacetKind,     // Manager | Object | Reference | Metadata (1С-специфичное)
}
```

**Фасетная система** - один тип 1С имеет множество представлений:
- Справочники.Контрагенты (Manager) - создание, поиск
- СправочникОбъект.Контрагенты (Object) - изменяемый объект
- СправочникСсылка.Контрагенты (Reference) - ссылка на элемент

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

#### Интегрированный Web сервер (API + Frontend)
```bash
cargo run -p bsl-backend --bin bsl-web-server -- --port 3001 --enable-cors true
# Доступен на: http://127.0.0.1:3001
```

#### CLI инструменты
```bash
# Проверка типов (основной CLI)
cargo run --bin bsl-type-check -- "Справочники.Контрагенты"
cargo run --bin bsl-type-check -- --complete "Справочники."
cargo run --bin bsl-type-check -- --help

# Для расширенной CLI функциональности (планируется)
# cargo run --bin bsl-cli -- analyze /path/to/project
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

### Frontend (интегрированный в backend)
```bash
# Сборка WASM файлов (если нужно обновить frontend)
cd frontend
trunk build --release

# Интегрированный веб-сервер (API + Static WASM files)
cargo run -p bsl-backend --bin bsl-web-server -- --port 3001 --enable-cors true

# Доступ к веб-интерфейсу
# http://127.0.0.1:3001
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

### Sourcebot для поиска в репозиториях
```bash
# Доступен через Claude Code MCP инструменты
# Sourcebot предоставляет поиск по коду с regex паттернами
```

**Возможности Sourcebot:**
- **Regex поиск** - точный поиск по regex паттернам в коде
- **Семантический поиск** - поиск концепций и архитектурных паттернов
- **Многоязычность** - поддержка русских терминов и комментариев
- **Фрагменты кода** - возвращает релевантные отрывки с контекстом
- **GitHub интеграция** - прямые ссылки на исходный код

**Примеры использования:**
- Точный поиск: `SystemCoordinator` - найдет все упоминания компонента
- Семантический: `координатор|зависимост|архитектур` - найдет концептуально связанные темы
- Архитектурный анализ: `dependency injection container IoC` - поиск паттернов DI
- Многоязычный: `управление жизненным циклом` - поиск русскоязычной документации

**Рекомендации по использованию Sourcebot:**
- Используйте для исследования архитектурных решений в коде
- Отлично подходит для поиска примеров использования компонентов
- Семантический поиск помогает найти связанные концепции
- Комбинируйте с ast-grep для комплексного анализа кодовой базы

## Полезные примеры

### Тестирование и разработка
```bash
# Все тесты
cargo test

# Configuration-guided discovery (если реализованы)
cargo run --example test_simple
cargo test --test config_parser_guided_test

# Другие примеры (если реализованы)
cargo run --example syntax_helper_parser_demo

# Производительность
cargo bench

# Линтинг текущего кода
cargo clippy --workspace --all-targets --all-features
```

### Web API тестирование
```bash
# Запуск сервера
cargo run -p bsl-backend --bin bsl-web-server -- --port 3001 --enable-cors true

# Тестирование API
curl "http://localhost:3001/api/types?search=Массив"
curl "http://localhost:3001/api/health"
curl -X POST "http://localhost:3001/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"code": "Функция Тест() Возврат 42; КонецФункции"}'
```

## Компоненты архитектуры

### Организация кода по слоям (в рамках крейтов)

**shared/** - чистая доменная логика + общий оркестратор:
```
shared/
├── domain/      # TypeResolver, TypeRepository, types
├── engine/      # AnalysisEngine - переиспользуемое ядро анализа
└── api/         # DTO и контракты для API
```

**backend/** - все серверные слои в одном крейте:
```
backend/
├── system/      # SystemCoordinator, AnalysisCache, BasicObservability
├── application/ # TypeSystemService (использует shared::engine)
├── presentation/# LSP Server, Web routes
└── data/        # Platform types, Config loaders
```

### Ключевые принципы

1. **AnalysisEngine** (shared) - чистый оркестратор без I/O зависимостей
2. **TypeSystemService** (backend) - высокоуровневый API с кэшированием
3. **SystemCoordinator** - единая точка координации всех компонентов
4. **Фасетная система** - автоматическое переключение контекста 1С объектов
5. **Градуальная типизация** - честность о неопределенности типов

## MCP Инструментарий

Claude Code предоставляет богатый набор MCP (Model Context Protocol) инструментов для эффективной работы с BSL проектом.

### Основные инструменты для BSL

#### Chrome DevTools - автоматизация веб-интерфейса
```bash
# Запуск BSL веб-сервера для тестирования
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true

# Автоматизированное тестирование через Chrome DevTools:
# - take_screenshot - снимки интерфейса
# - click, fill, hover - взаимодействие с элементами
# - list_network_requests - анализ API запросов
# - performance_start_trace - измерение производительности
# - evaluate_script - выполнение JavaScript на странице
```

**Типовые сценарии Chrome DevTools:**
- Тестирование поиска типов и фильтрации
- Проверка производительности WASM компонентов
- Анализ API запросов к `/api/types` и `/api/search`
- Автоматические скриншоты для документации

#### Language Server Protocol - Rust диагностика
```bash
# Доступные LSP команды:
# - diagnostics - проверка ошибок компиляции
# - hover - информация о типах и функциях
# - definition - переход к определению символа
# - references - поиск всех использований
# - rename_symbol - безопасный рефакторинг

# Особенно полезно для:
# - Анализа TypeResolver и AnalysisEngine
# - Рефакторинга SystemCoordinator
# - Проверки совместимости API между крейтами
```

#### Sourcebot - поиск архитектурных паттернов
```bash
# Поиск ключевых компонентов BSL:
# SystemCoordinator - точка координации
# TypeResolver - логика анализа типов
# AnalysisEngine - оркестратор анализа
# FacetKind - фасетная система 1С

# Семантический поиск:
# "градуальная типизация" - концептуальные материалы
# "flow sensitive analysis" - алгоритмы анализа потоков
# "rust dependency injection" - паттерны DI
```

#### Context7 - документация библиотек
```bash
# Получение актуальной документации:
# - Leptos (frontend WASM)
# - Tower/Axum (web сервер)
# - Tree-sitter (парсинг)
# - Tokio (async runtime)

# Особенно полезно при:
# - Обновлении зависимостей
# - Изучении новых API
# - Поиске примеров использования
```

#### Tavily - веб-исследования
```bash
# Поиск информации о:
# - Градуальной типизации в языках программирования
# - Архитектурных паттернах для анализаторов кода
# - Лучших практиках TypeScript/Rust интеграции
# - Производительности WASM в браузерах
```

### Комплексные сценарии

#### Полное тестирование BSL системы
```bash
# 1. Запуск backend
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002

# 2. Chrome DevTools автотесты:
#    - Загрузка интерфейса и снимок
#    - Тестирование поиска "Справочники"
#    - Проверка фильтров и навигации
#    - Измерение LCP и Core Web Vitals

# 3. Language Server диагностика:
#    - Проверка всех Rust файлов на ошибки
#    - Анализ типов в shared/src/domain/
#    - Валидация API контрактов
```

#### Рефакторинг архитектуры
```bash
# 1. Sourcebot - поиск паттернов использования компонента
# 2. Language Server - анализ зависимостей и типов
# 3. Chrome DevTools - проверка не поломался ли UI
# 4. Context7 - изучение альтернативных подходов
```

#### Исследование производительности
```bash
# 1. Chrome DevTools Performance trace
# 2. Анализ Network requests для API оптимизации
# 3. Tavily - поиск бенчмарков WASM vs JS
# 4. Context7 - документация по оптимизации Leptos
```

### Рекомендации по использованию

**Проактивное использование:**
- Chrome DevTools автоматически после изменений UI
- Language Server при рефакторинге Rust кода
- Sourcebot для изучения архитектурных решений
- Context7 перед обновлением зависимостей

**Эффективные комбинации:**
- LSP диагностика + Chrome DevTools тестирование
- Sourcebot поиск + Context7 документация
- Performance trace + Tavily исследование оптимизаций

**Специфика BSL проекта:**
- Фасетная система требует особого внимания к типам
- WASM компоненты лучше тестировать в реальном браузере
- Русскоязычные термины 1С в поиске и документации