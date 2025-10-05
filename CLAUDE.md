# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 📋 Содержание

1. [Правила работы с Roadmap](#правила-работы-с-roadmap)
2. [Архитектура проекта](#архитектура-проекта)
3. [Команды разработки](#команды-разработки)
4. [Архитектурная диаграмма](#архитектурная-диаграмма)
5. [Компоненты архитектуры](#компоненты-архитектуры)
6. [Анализ кода](#анализ-кода)
7. [MCP Инструментарий](#mcp-инструментарий)
8. [Научные основы](#научные-основы)

---

## Правила работы с Roadmap

### 🚨 КРИТИЧЕСКОЕ ТРЕБОВАНИЕ: Контрольная проверка выполнения

**ПРИ ВЫПОЛНЕНИИ ЛЮБЫХ ЭТАПОВ ИЗ ROADMAP:**

1. **ПЕРЕД отчётом о выполнении** — ОБЯЗАТЕЛЬНО провести **контрольную проверку фактического выполнения**

2. **Проверка должна включать:**
   - ✅ Чтение реального кода в файлах (Read tool)
   - ✅ Поиск реализованных функций/структур (Grep/Glob tools)
   - ✅ Запуск тестов для проверки работоспособности (Bash: cargo test)
   - ✅ Проверка компиляции (Bash: cargo check/build)

3. **ЗАПРЕЩЕНО:**
   - ❌ Утверждать о выполнении без проверки кода
   - ❌ Отчитываться о готовности всего этапа, если выполнена только часть задачи
   - ❌ Предполагать наличие кода без чтения файлов
   - ❌ Заявлять о прохождении тестов без их реального запуска

4. **Формат отчёта о выполнении:**
   ```markdown
   ## Статус выполнения [Milestone X.Y]

   ### ✅ Task N: [Название] — ВЫПОЛНЕНО
   **Проверка:**
   - ✅ [Файл:строка] — реализация найдена
   - ✅ cargo test [test_name] — тесты проходят
   - ✅ cargo check — компиляция успешна

   ### ❌ Task M: [Название] — НЕ НАЧАТО
   **Проверка:**
   - ❌ grep показывает отсутствие кода
   - ❌ файл не существует

   ### ⚠️ Task K: [Название] — ЧАСТИЧНО (X%)
   **Что есть:**
   - ✅ [конкретные файлы и строки]
   **Что отсутствует:**
   - ❌ [конкретные недостающие компоненты]
   ```

5. **Примеры ПРАВИЛЬНОЙ проверки:**
   ```bash
   # Проверка Task 1: Подключить tree-sitter-bsl
   grep -n "tree-sitter-bsl" Cargo.toml backend/Cargo.toml
   cargo test -p bsl-backend tree_sitter

   # Проверка Task 2: TreeSitterAdapter
   find backend/src -name "*adapter*" | grep tree
   grep -n "TreeSitterAdapter" backend/src -r

   # Проверка Task 4: Flow-sensitive analysis
   grep -n "FlowSensitive\|CFG\|ControlFlowGraph" backend/src -r
   ```

6. **Честность в отчётности:**
   - Если задача выполнена на 10% — так и указывай: **⚠️ ЧАСТИЧНО (10%)**
   - Если код не найден — четко заявляй: **❌ НЕ НАЧАТО**
   - Если тесты не проходят — не считай задачу выполненной

**Цель:** Реальный прогресс вместо иллюзии выполнения. Пользователь должен видеть объективную картину, а не оптимистичные предположения.

---

## Архитектура проекта

BSL Gradual Type System - система градуальной типизации для языка 1С:Предприятие с Right-Sized Architecture философией.

### 📚 Научные основы

Проект основан на исследованиях в области статической типизации для 1С:Предприятие:

**Balyuk, A. S., & Popova, V. A. (2021).** *Static type-checking for programs developed on the platform 1C:Enterprise.* CEUR Workshop Proceedings, Vol-2984. [https://ceur-ws.org/Vol-2984/paper13.pdf](https://ceur-ws.org/Vol-2984/paper13.pdf)

Ключевые концепции из статьи, применённые в проекте:
- **Фасетная система типов** — множественное наследование функциональности объектов 1С (Manager, Object, Reference, Selection, List)
- **Configuration Types Tree (CTT)** — упрощённый формат для описания типов конфигурации
- **Три категории типовых ошибок:**
  1. Некорректная передача параметров методам
  2. Обращение к несуществующим свойствам объектов
  3. Обработка простых типов как коллекций

Валидация этих ошибок реализована в модуле [shared/src/domain/validators.rs](shared/src/domain/validators.rs).

### 🎯 Философия: Right-Sized Architecture

**Start simple, scale up по необходимости** — 6-8 компонентов вместо 25-30.

### 📦 Workspace структура

```
bsl-gradual-types/
├── shared/          # Чистая доменная логика + AnalysisEngine
│   ├── domain/      # TypeResolver, TypeRepository, types
│   ├── engine/      # AnalysisEngine - переиспользуемое ядро
│   └── api/         # DTO и контракты для API
│
├── backend/         # Все серверные слои в одном крейте
│   ├── system/      # SystemCoordinator, AnalysisCache, Observability
│   ├── application/ # TypeSystemService (использует shared::engine)
│   ├── presentation/# LSP Server, Web routes
│   └── data/        # Platform types, Config loaders
│
├── frontend/        # Веб-интерфейс на Leptos WASM
├── cli/             # CLI инструменты
└── vscode-extension/# VSCode расширение (TypeScript)
```

### 🔑 Ключевые компоненты

#### System Layer (в backend)
- **SystemCoordinator** — единая точка координации и DI management
- **AnalysisCache** — простое LRU кеширование в памяти с TTL
- **ParserCoordinator** — TreeSitter (основной) + Regex (fallback)
- **BasicObservability** — структурированное логирование и базовые метрики

#### Application Layer
- **AnalysisEngine** (в shared) — чистый оркестратор анализа без зависимостей от backend
- **TypeSystemService** (в backend) — высокоуровневый API для Web/LSP, использует AnalysisEngine

#### Domain Layer (в shared)
- **TypeResolver** — центральная логика анализа типов с flow-sensitive анализом
- **TypeRepository** — абстракция для работы с данными

### 🎯 Центральная абстракция: TypeResolution

```rust
struct TypeResolution {
    certainty: Certainty,        // Known | Inferred(0.0-1.0) | Unknown
    result: ResolutionResult,    // Concrete | Union | Dynamic
    active_facet: FacetKind,     // Manager | Object | Reference | Metadata
}
```

**Фасетная система** — один тип 1С имеет множество представлений:
- `Справочники.Контрагенты` (Manager) — создание, поиск
- `СправочникОбъект.Контрагенты` (Object) — изменяемый объект
- `СправочникСсылка.Контрагенты` (Reference) — ссылка на элемент
- `СправочникВыборка.Контрагенты` (Selection) — обход элементов
- `СправочникСписок.Контрагенты` (List) — управление списком в форме

*Selection и List добавлены на основе статьи Balyuk & Popova (2021)*

---

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
# Базовый запуск (только примитивные типы)
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true

# С парсингом Синтаксис-помощника (полные типы платформы)
# Передаём родительскую папку - парсер автоматически найдёт обе подпапки:
#   - rebuilt.shcntx_ru (контекстная справка: объекты, методы, свойства)
#   - rebuilt.shlang_ru (справка по языку: примитивные типы, операторы)
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true --syntax-helper-path examples/syntax_helper

# Доступен на: http://127.0.0.1:3002
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

**ВАЖНО: Работа с кириллицей в URL через bash**

GitBash на Windows требует URL-кодирования для кириллических символов. Используй URL-encoded строки:

```bash
# ❌ НЕ РАБОТАЕТ - кириллица напрямую
curl "http://localhost:3002/api/search?q=Массив"

# ✅ РАБОТАЕТ - URL-encoded кириллица
curl "http://localhost:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"

# Конвертация: используй онлайн URL encoder или Python
python3 -c "import urllib.parse; print(urllib.parse.quote('УровеньИспользованияЗащищенногоСоединенияFTP'))"
```

**Примеры API запросов:**

```bash
# Запуск сервера
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true

# Health check
curl "http://localhost:3002/api/health"

# Поиск типа (латиница - работает без encoding)
curl "http://localhost:3002/api/types?search=Array"

# Поиск типа (кириллица - требует URL encoding)
curl "http://localhost:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2" | jq '.'

# Анализ кода
curl -X POST "http://localhost:3002/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"code": "Функция Тест() Возврат 42; КонецФункции"}'
```

---

## Архитектурная диаграмма

### 🏗️ Simplified Architecture Diagram

```mermaid
graph TB
    subgraph "🎯 System Layer (в `backend`)"
        SystemCoordinator["🎯 SystemCoordinator<br/>- Single coordination point<br/>- DI management<br/>- Lifecycle control"]
        
        AnalysisCache["💾 AnalysisCache<br/>- Simple LRU in-memory<br/>- File hash keys<br/>- TTL eviction"]
        
        ParserCoordinator["🎨 ParserCoordinator<br/>- TreeSitter (primary)<br/>- Regex fallback<br/>- Simple selection logic"]
        
        BasicObservability["📊 BasicObservability<br/>- Structured logging<br/>- Basic metrics<br/>- Health endpoint"]
    end

    subgraph "🌐 Presentation Layer (Адаптеры)"
        LSPServer["🔌 LSP Server (`backend`)<br/>- Language Server Protocol<br/>- VS Code integration"]

        WebInterface["🌐 Web Interface (`backend`)<br/>- Simple HTML dashboard<br/>- Type visualization"]

        CLITool["⚙️ CLI Tool (`cli`)<br/>- Command line interface<br/>- Batch analysis"]
    end

    subgraph "🔧 Application Layer" 
        subgraph "`backend`"
            TypeSystemService["🎭 TypeSystemService<br/>- High-level API (Web, LSP)<br/>- Управляет кэшем<br/>- **Использует AnalysisEngine**"]
        end
        subgraph "`shared`"
            AnalysisEngine["🚀 AnalysisEngine<br/>- **Чистая оркестрация анализа**<br/>- Use Case: 'Analyze File'<br/>- Не зависит от Web/CLI"]
        end
    end

    subgraph "🧠 Domain Layer (`shared`)"
        TypeResolver["🧠 TypeResolver<br/>- Core type analysis<br/>- Resolution algorithms<br/>- Business logic"]

        TypeMetadataLookup["🔍 TypeMetadataLookup<br/>- Bridge: TypeResolution → RawTypeData<br/>- Get methods/properties<br/>- Validation support"]

        TypeRepository["📚 TypeRepository<br/>- Type storage<br/>- Query interface<br/>- Data abstraction"]
    end

    subgraph "💾 Data Layer (`shared`)"
        PlatformTypes["📄 Platform Types<br/>- 1C platform metadata<br/>- HTML parsing<br/>- Type definitions"]

        ConfigData["⚙️ Configuration<br/>- XML metadata<br/>- Settings<br/>- User preferences"]
    end

    %% Flow
    SystemCoordinator --> AnalysisCache
    SystemCoordinator --> ParserCoordinator
    SystemCoordinator --> BasicObservability
    SystemCoordinator --> TypeSystemService

    LSPServer --> TypeSystemService
    WebInterface --> TypeSystemService

    TypeSystemService --> AnalysisEngine
    TypeSystemService --> AnalysisCache

    CLITool --> AnalysisEngine
    
    AnalysisEngine --> TypeResolver
    AnalysisEngine --> ParserCoordinator

    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository
    TypeRepository --> PlatformTypes
    TypeRepository --> ConfigData

    %% TypeMetadataLookup используется для получения методов/свойств
    TypeSystemService -.-> TypeMetadataLookup
    AnalysisEngine -.-> TypeMetadataLookup
    
    %% Styling
    classDef systemStyle fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef presentationStyle fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef applicationStyle fill:#e8f5e8,stroke:#388e3c,stroke-width:2px
    classDef domainStyle fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef dataStyle fill:#fce4ec,stroke:#c2185b,stroke-width:2px

    class SystemCoordinator,AnalysisCache,ParserCoordinator,BasicObservability systemStyle
    class LSPServer,WebInterface,CLITool presentationStyle
    class TypeSystemService,AnalysisEngine applicationStyle
    class TypeResolver,TypeMetadataLookup,TypeRepository domainStyle
    class PlatformTypes,ConfigData dataStyle
```

### 📊 Описание потоков данных

**Presentation → Application:**
- LSP Server, Web Interface → TypeSystemService
- CLI Tool → AnalysisEngine (напрямую)

**Application → Domain:**
- TypeSystemService → AnalysisEngine → TypeResolver
- AnalysisEngine → ParserCoordinator

**Domain → Data:**
- TypeResolver → TypeRepository → PlatformTypes/ConfigData

**System Management:**
- SystemCoordinator координирует все backend компоненты

---

## Компоненты архитектуры

### 🔧 Детальное описание компонентов

#### 🎯 SystemCoordinator (в `backend`)
- **Структура:** Содержит экземпляры всех ключевых системных сервисов
- **Назначение:** Composition Root, управление жизненным циклом приложения
- **Зависимости:** AnalysisCache, ParserCoordinator, BasicObservability, TypeSystemService

#### 🚀 AnalysisEngine (в `shared`)
- **Структура:** Содержит TypeResolver и ParserCoordinator
- **Назначение:** Чистый сценарий "проанализировать файл"
- **Особенность:** Не зависит от backend, переиспользуется всеми адаптерами

#### 🎭 TypeSystemService (в `backend`)
- **Структура:** Содержит AnalysisEngine и backend-специфичные компоненты
- **Назначение:** Высокоуровневый API для LSP/Web с кэшированием
- **Использует:** AnalysisEngine, AnalysisCache

#### 💾 AnalysisCache
- **Структура:** LRU-кэш с TTL отслеживанием
- **Назначение:** Кэширование результатов анализа файлов в памяти
- **Ключи:** File hash для быстрого lookup

#### 🎨 ParserCoordinator
- **Структура:** TreeSitter (primary) + Regex (fallback)
- **Назначение:** Парсинг исходного кода с простой стратегией fallback
- **Логика:** Попытка основного парсера → при ошибке → запасной

#### 📊 BasicObservability
- **Структура:** Структурированный логгер + простые метрики
- **Назначение:** Мониторинг работы приложения
- **Функции:** Логирование, метрики производительности, health endpoint

### 🎯 Ключевые принципы архитектуры

1. **AnalysisEngine** (shared) — чистый оркестратор без I/O зависимостей
2. **TypeSystemService** (backend) — высокоуровневый API с кэшированием
3. **SystemCoordinator** — единая точка координации всех компонентов
4. **Фасетная система** — автоматическое переключение контекста 1С объектов
5. **Градуальная типизация** — честность о неопределенности типов
6. **Слои внутри крейтов** — логическое разделение без физической фрагментации

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