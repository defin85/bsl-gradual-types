# Backend - BSL Gradual Type System

> Серверная часть системы статической типизации для языка 1С:Предприятие

## Обзор архитектуры

Backend реализует многослойную архитектуру с чётким разделением ответственности. Основной принцип - **Right-Sized Architecture**: простые, фокусированные компоненты вместо монолитных модулей.

### Диаграмма слоёв

```
┌─────────────────────────────────────────────────────────────┐
│                    Presentation Layer                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ LSP Server   │  │ Web Server   │  │  CLI Tool    │      │
│  │ (bin/lsp_*)  │  │ (main.rs)    │  │  (external)  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                     System Layer                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ SystemCoordinator - координация всех подсистем      │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ AnalysisCache - кеширование результатов анализа     │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ ParserCoordinator - управление парсингом            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   Application Layer                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ TypeSystemService - единая точка входа для LSP/API  │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ Services: hover, completion, diagnostics, etc.      │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ AstToIrConverter - конвертация AST → IR            │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ SemanticValidationVisitor - валидация семантики    │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                     Domain Layer                            │
│  (shared crate - разделяется с frontend)                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ AnalysisEngine - основной движок типизации          │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ TypeResolver - резолвинг типов                      │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ TypeRepository - хранилище типов (3927 типов)      │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                      Data Layer                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ ConfigLoader - загрузка конфигурации 1С             │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ SyntaxHelperLoader - загрузка типов платформы       │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ HBK Recovery - восстановление повреждённых .hbk     │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                     Parsing Layer                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ TreeSitterAdapter - парсинг через tree-sitter       │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Структура модулей

### `application/` - Application Layer

Бизнес-логика типизации и LSP функций.

**Ключевые компоненты:**
- `type_system/` - TypeSystemService и services (hover, completion, diagnostics)
- `semantic_validation_visitor/` - валидация семантических правил
- `ast_to_ir/` - конвертация AST → Intermediate Representation
- `type_inference_service.rs` - вывод типов выражений

**Точки входа:**
- `TypeSystemService::get_hover()` - информация при наведении
- `TypeSystemService::get_completion()` - автодополнение
- `TypeSystemService::validate_semantics()` - семантическая валидация

### `domain/` - Domain Layer (backend-specific)

Backend-специфичная доменная логика.

**Компоненты:**
- `flow_analyzer.rs` - flow-sensitive анализ типов (в процессе разработки)

**Примечание:** Основная доменная логика (TypeResolver, TypeRepository) находится в `shared` crate.

### `data/` - Data Layer

Загрузка данных из внешних источников.

**Структура:**
- `loaders/` - загрузчики данных
  - `config_loader/` - метаданные конфигурации 1С
  - `syntax_helper/` - типы платформы из syntax helper
- `adapters/` - адаптеры к внешним API

**Основные операции:**
- Чтение XML конфигурации 1С
- Парсинг HTML документации синтаксис-помощника
- Восстановление повреждённых .hbk файлов

### `parsing/` - Parsing Layer

Адаптер к tree-sitter парсеру.

**Компоненты:**
- `tree_sitter_adapter.rs` - обёртка над tree-sitter-bsl
- `node_utils.rs` - утилиты для работы с AST

**Особенность:** После Milestone 2.8 AST конвертируется в IR (Intermediate Representation), который не зависит от конкретного парсера.

### `system/` - System Layer

Координация подсистем и кеширование.

**Компоненты:**
- `coordinator/` - SystemCoordinator - главный координатор
- `cache/` - AnalysisCache - кеширование результатов
- `parser_coordinator/` - ParserCoordinator - управление парсингом
- `observability/` - BasicObservability - метрики и мониторинг

**Роль:** Обеспечивает integration point между всеми слоями.

### `presentation/` - Presentation Layer

API endpoints и адаптеры для клиентов.

**Структура:**
- `web_api/` - HTTP API endpoints
- `semantic_html_generator/` - генерация HTML визуализации типов
- `formatters/` - форматирование ответов

**API группы:**
- `/api/hover` - hover информация
- `/api/diagnostics` - синтаксические и семантические ошибки
- `/api/types` - поиск и информация о типах
- `/api/debug/ast` - отладочная информация

### `helpers/` - Helper Layer

Вспомогательные утилиты.

**Компоненты:**
- `hover_formatter.rs` - форматирование hover сообщений
- `type_display.rs` - отображение типов в читаемом виде

### `bin/` - Binary Targets

Исполняемые файлы и серверы.

**Структура:**
- `lsp_server/` - LSP server для VSCode
  - `server/` - core LSP logic
  - `handlers/` - LSP request handlers
  - `commands/` - custom LSP commands
  - `converters/` - конвертация типов LSP ↔ internal
- `main.rs` - Web server (bsl-web-server)

## Типичный поток данных

### Пример: LSP Hover Request

```
1. VSCode Extension отправляет textDocument/hover
           ↓
2. LSP Server (bin/lsp_server/handlers/hover.rs)
   - Получает позицию в документе
           ↓
3. SystemCoordinator.parse_file()
   - ParserCoordinator → TreeSitterAdapter → AST
           ↓
4. AstToIrConverter.convert()
   - AST → Intermediate Representation (IR)
           ↓
5. TypeSystemService.get_hover()
   - Анализ IR
   - AnalysisEngine.infer_type()
   - TypeResolver.resolve_type()
           ↓
6. TypeRepository.get_type_info()
   - Поиск метаданных типа
   - Данные из ConfigLoader / SyntaxHelperLoader
           ↓
7. HoverFormatter.format()
   - Форматирование hover текста
           ↓
8. LSP Server отправляет Hover response в VSCode
```

### Пример: Semantic Diagnostics

```
1. Web API POST /api/diagnostics
           ↓
2. SystemCoordinator.validate_file()
   - Парсинг → AST → IR
           ↓
3. TypeSystemService.validate_semantics()
   - SemanticValidationVisitor.visit(IR)
   - Проверка:
     - Несоответствие типов параметров
     - Неизвестные методы/свойства
     - Некорректные операции над типами
           ↓
4. Semantic errors → JSON response
```

## Начало работы

### Для изучения архитектуры

**Начните с этих файлов (в порядке приоритета):**

1. `application/type_system/service.rs` - главная точка входа
2. `system/coordinator/mod.rs` - координация подсистем
3. `bin/lsp_server/main.rs` - LSP server entry point
4. `data/loaders/mod.rs` - загрузка данных
5. `application/ast_to_ir/converter.rs` - конвертация AST → IR

### Для добавления новой LSP функции

1. Добавить handler в `bin/lsp_server/handlers/`
2. Добавить service метод в `application/type_system/services/`
3. Использовать `TypeSystemService` для доступа к типам
4. Обновить `bin/lsp_server/server/language_server.rs`

### Для добавления нового типа валидации

1. Расширить `SemanticValidationVisitor` в `application/semantic_validation_visitor/`
2. Добавить новый тип ошибки в `shared/src/domain/semantic_error.rs`
3. Добавить тест в `backend/tests/semantic_diagnostics_lsp_test.rs`

## Связанные документы

- [Application Layer README](application/README.md) - детали application layer
- [LSP Client README](../../vscode-extension/src/lsp/README.md) - VSCode extension
- [Архитектура системы типов](../../docs/architecture/type_system_architecture.md) - полная архитектурная диаграмма
- [Web API Reference](../../docs/api/web-api-reference.md) - документация API endpoints

## Ключевые принципы разработки

1. **Separation of Concerns** - каждый слой имеет чёткую ответственность
2. **Dependency Inversion** - высокоуровневые модули не зависят от низкоуровневых
3. **Single Source of Truth** - TypeRepository - единственный источник метаданных
4. **Gradual Typing** - честность о неопределённости типов через `Certainty`
5. **Right-Sized Architecture** - 6-8 компонентов вместо 25-30

## Версия

**Backend версия:** 0.4.2
**Статус:** Production-ready (LSP + Web API)
