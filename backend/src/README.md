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
│  │ DiskCache / AstCache - кеширование артефактов       │   │
│  ├──────────────────────────────────────────────────────┤   │
│  │ ParserCoordinator - управление парсингом            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   Application Layer                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ type_system - entrypoints для LSP/Web/CLI           │   │
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

`bsl-backend` в первую очередь содержит presentation и binary targets; остальные слои поставляются через re-export из `bsl-runtime` (см. `backend/src/lib.rs`).

### `backend/src/presentation/` - Presentation Layer

HTTP/Web интерфейс и presentation-компоненты для визуализации/CLI. LSP сервер живёт в `backend/src/bin/lsp_server/`.

**Ключевые компоненты:**
- `backend/src/presentation/web/` - Web API handlers (в т.ч. diagnostics / semantic tree)
- `backend/src/presentation/semantic_html_generator/` - генерация HTML визуализации типов
- `backend/src/presentation/cli/` - CLI presentation (если используется)

### `backend/src/bin/` - Binary Targets

**Ключевые компоненты:**
- `backend/src/bin/lsp_server/` - LSP server для VSCode
- `backend/src/main.rs` - Web server (bsl-web-server)

### `backend/src/config/` - Конфигурация

Конфиг/параметры запуска backend.

### `bsl-runtime/src/application/` - Application Layer

Entry points для hover/completion/diagnostics (в т.ч. v2 pipeline).

**Примеры:**
- `bsl-runtime/src/application/type_system/services/hover_service.rs`
- `bsl-runtime/src/application/type_system/services/completion_service.rs`

### `bsl-runtime/src/system/` - System Layer

Координация подсистем и кеширование.

**Примеры:**
- `bsl-runtime/src/system/deps_bundle_v2.rs`
- `bsl-runtime/src/system/system_coordinator/`

### `bsl-runtime/src/data/` и `bsl-runtime/src/parsing/`

Загрузка данных и парсинг BSL (tree-sitter).

### `bsl-runtime/src/domain/` и `shared/src/domain/`

Domain-логика. Основные доменные компоненты (TypeResolver/TypeRepository) находятся в `shared`.

## Типичный поток данных

### Пример: LSP Hover Request

```
1. VSCode Extension отправляет textDocument/hover
           ↓
2. LSP Server (bin/lsp_server/handlers/hover.rs)
   - Получает позицию в документе
           ↓
3. `analysis_v2_runtime.snapshot()`
   - v2 snapshot (writer thread + observed ids)
           ↓
4. salsa queries: `analysis.ir(file_id)`
   - `SemanticProgram` (IR) из снапшота
           ↓
5. `application::get_hover_info_with_semantic_program()`
   - IR + deps snapshot + metadata lookup + `type_at_position` (v2)
           ↓
6. `TypeRepository` (из deps snapshot)
   - метаданные типов платформы/конфигурации
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
2. `presentation/web/handlers.rs`
   - snapshot + queries (`syntax_diagnostics`, `semantic_diagnostics`)
           ↓
3. Диагностики конвертируются в DTO/JSON
           ↓
4. Semantic errors → JSON response
```

## Начало работы

### Для изучения архитектуры

**Начните с этих файлов (в порядке приоритета):**

1. `bin/lsp_server/server/analysis_v2_runtime.rs` - v2 runtime (writer thread + snapshots)
2. `../../bsl-runtime/src/application/type_system/services/completion_service.rs` - completion логика
3. `../../bsl-runtime/src/application/type_system/services/hover_service.rs` - hover логика
4. `../../bsl-runtime/src/system/deps_bundle_v2.rs` - deps snapshot (P8/P9)
5. `../../bsl-runtime/src/system/system_coordinator/` - загрузка типов платформы/конфигурации

### Для добавления новой LSP функции

1. Добавить handler в `bin/lsp_server/handlers/`
2. Добавить service метод в `../../bsl-runtime/src/application/type_system/services/`
3. Использовать v2 снапшот (IR + deps bundle) и entrypoints из `../../bsl-runtime/src/application/type_system/`
4. Обновить `bin/lsp_server/server/language_server.rs`

### Для добавления нового типа валидации

1. Добавить правило/проверку в `../../semantic-diagnostics/src/visitor.rs` (и/или `../../semantic-diagnostics/src/validators/`)
2. При необходимости расширить диагностический формат в `../../shared/src/domain/types/diagnostics.rs`
3. Добавить тест в `backend/tests/context_diagnostics_lsp_test.rs`

## Связанные документы

- [Application Layer README](../../bsl-runtime/src/application/README.md) - детали application layer
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
