# 🗺️ BSL Gradual Types — Roadmap 2025

**Проект:** BSL Gradual Type System для 1С:Предприятие
**Философия:** Right-Sized Architecture — начинаем просто, масштабируем по необходимости
**Версия:** 1.0 → 2.0 → 3.0
**Дата:** 2025-10-05

---

## 📊 Текущее состояние проекта (Версия 1.0)

### ✅ Что работает отлично

#### Backend (Rust)
- **Right-Sized Architecture** — 6-8 компонентов вместо 25-30 ✅
- **SystemCoordinator** — единая точка координации и DI management ✅
- **TypeSystemService** — application layer с бизнес-логикой ✅
- **TypeResolver** — чистая доменная логика без I/O ✅
- **TypeMetadataLookup** — bridge для валидации методов/свойств ✅
- **SyntaxHelperParser** — 3927 типов платформы из документации ✅
- **Web API** — REST endpoints для валидации кода ✅
- **LSP Server** — работающий Language Server Protocol ✅

**Статистика:**
- 🎯 3927 типов платформы 1С
- ⚡ Валидация < 1ms
- 📦 Бинарник LSP сервера: 7.3 MB
- ✅ 0 clippy warnings
- 🧪 Unit-тесты для критичных компонентов

#### VSCode Extension (TypeScript)
- **7,591 строк кода** — модульная архитектура ✅
- **LSP клиент** — STDIO/TCP режимы, health check ✅
- **20+ команд** — BSL Index, Verification, Analyzer ✅
- **5 сайдбар-панелей** — Overview, Diagnostics, Type Index ✅
- **BSL грамматика** — синтаксис-подсветка с кириллицей ✅
- **0 TypeScript ошибок** — strict mode ✅

### ⚠️ Что нужно улучшить

#### VSCode Extension
- 🚨 **Размер 30 MB** — слишком большой (норма < 5 MB)
- 🚨 **10 бинарников** — дублирование функциональности
- 🚨 **CLI вызовы** — вместо LSP requests
- 🚨 **Enhanced Features неактивны** — объявлены, но исключены из сборки
- 🚨 **Отсутствие тестов** — только заглушки

#### Backend
- ⚠️ **Tree-sitter НЕ используется** — BslParser возвращает пустой AST
- ⚠️ **Парсинг кода не работает** — валидация только по именам типов
- ⚠️ **Flow-sensitive analysis** — не реализован
- ⚠️ **Union types** — базовая поддержка без нормализации

---

## 🎯 Версия 2.0 — "Production Ready" (Q1 2025: 8-10 недель)

**Цель:** Превратить MVP в production-ready инструмент для ежедневной работы разработчиков 1С

**Ключевое изменение:** Tree-sitter интеграция — **ОСНОВА** всего остального анализа

---

### 🧠 Milestone 2.1: Tree-sitter Integration (2-3 недели) ← **ПЕРВЫЙ ПРИОРИТЕТ**

**Приоритет:** 🔴 КРИТИЧЕСКИЙ — фундамент для всех остальных фич

**Почему tree-sitter первым:**
```
Tree-sitter AST — фундамент для:
├── Flow-sensitive analysis (отслеживание типов в коде)
├── Type inference (вывод типов из контекста)
├── Advanced Types (Generic требуют парсинга выражений)
├── Semantic highlighting (подсветка по типам)
└── LSP features (goto definition, find references)
```

#### Задачи:

1. **Подключить tree-sitter-bsl грамматику** (3-4 дня)
   - ✅ **Грамматика уже существует:** [alkoleft/tree-sitter-bsl](https://github.com/alkoleft/tree-sitter-bsl) v0.1.5
   - ✅ Добавить в `Cargo.toml`:
     ```toml
     tree-sitter-bsl = { git = "https://github.com/alkoleft/tree-sitter-bsl", tag = "v0.1.5" }
     ```
   - ✅ Базовая интеграция в `ParserCoordinator`
   - ✅ Первые тесты парсинга простых конструкций
   - 🎯 **Цель:** Парсинг работает для базовых конструкций (функции, переменные, условия)

2. **Создать адаптер tree-sitter AST → Program AST** (5-6 дней)
   - ✅ Реализовать `TreeSitterAdapter::convert_node()` — конвертация узлов
   - ✅ Маппинг основных конструкций:
     - Выражения (binary, unary, call, member access)
     - Statements (if, while, for, return, assignment)
     - Declarations (function, procedure, variable)
   - ✅ Обработка ошибок парсинга (error nodes)
   - ✅ Fallback на regex при критических ошибках
   - 🎯 **Цель:** Полная конвертация tree-sitter AST в наш `Program` AST

3. **Инкрементальный парсинг для LSP** (3-4 дня)
   - ✅ Реализовать `update_incremental()` для редактирования файлов
   - ✅ Кеширование tree-sitter деревьев в памяти
   - ✅ Обновление только изменённых частей AST
   - ✅ Интеграция с LSP `textDocument/didChange`
   - 🎯 **Цель:** Инкрементальное обновление < 10ms (вместо полного репарсинга)

4. **Flow-sensitive analysis (базовый)** (3-4 дня)
   - ✅ Отслеживание изменений типов переменных в коде
   - ✅ Анализ ветвлений (Если-Тогда-Иначе)
   - ✅ Анализ присваиваний и вызовов методов
   - ✅ Построение графа потока данных (простой CFG)
   - 🎯 **Цель:** Точность определения типов > 70% (базовый уровень)

5. **Тесты и интеграция** (2-3 дня)
   - ✅ Unit-тесты для адаптера на реальных BSL файлах
   - ✅ Integration тесты с TypeResolver
   - ✅ Бенчмарки производительности (парсинг 1000+ строк)
   - ✅ Обновление документации и примеров
   - 🎯 **Цель:** 90% покрытие тестами, парсинг < 200ms для 10000 строк

**Результат Milestone 2.1:**
- ✅ Tree-sitter-bsl v0.1.5 интегрирован
- ✅ Работающий адаптер tree-sitter AST → Program AST
- ✅ Инкрементальный парсинг для LSP < 10ms
- ✅ Flow-sensitive analysis (базовый) > 70% точности
- ✅ Полноценный AST для всех дальнейших фич

---

### 📦 Milestone 2.2: VSCode Extension — Оптимизация (2 недели)

**Приоритет:** 🔴 КРИТИЧЕСКИЙ — использует tree-sitter парсер

#### Задачи:

1. **Уменьшить размер VSIX до < 5 MB** (3-4 дня)
   - ✅ Оптимизировать `.vscodeignore` — исключить `src/`, `*.ts`, `node_modules/`
   - ✅ Bundling через esbuild — минификация + tree-shaking
   - ✅ Вынести бинарники в отдельный релиз — скачивание при первом запуске
   - 🎯 **Цель:** VSIX 4-5 MB (вместо 30 MB)

2. **Оставить только LSP сервер** (2-3 дня)
   - ❌ Удалить 9 дублирующих бинарников
   - ✅ Встроить всю логику в `lsp_server.exe`
   - ✅ LSP сервер использует tree-sitter парсер
   - 🎯 **Цель:** 1 бинарник вместо 10

3. **Переписать команды на LSP requests** (4-5 дней)
   - ❌ Удалить CLI вызовы через `executeBslCommand()`
   - ✅ Реализовать LSP custom requests:
     - `bsl/queryType` — запрос информации о типе
     - `bsl/buildIndex` — построение индекса типов
     - `bsl/validateMethod` — валидация вызовов методов
     - `bsl/getMethodSignature` — сигнатуры методов
   - ✅ Переиспользование LSP соединения — быстрее в 10-100 раз
   - 🎯 **Цель:** 0 fork процессов, все через LSP

4. **Написать тесты** (2-3 дня)
   - ✅ Extension activation test
   - ✅ LSP client lifecycle tests (start/stop/restart)
   - ✅ Command registration и execution tests
   - ✅ Provider tests (Overview, Diagnostics, Type Index)
   - 🎯 **Цель:** 80% coverage для критичных компонентов

**Результат Milestone 2.2:**
- ✅ VSIX < 5 MB
- ✅ 1 бинарник (LSP сервер с tree-sitter)
- ✅ Все команды через LSP requests
- ✅ 80% test coverage

---

### 🔧 Milestone 2.3: Advanced Type System (3 недели)

**Приоритет:** 🟠 ВЫСОКИЙ — работает НА tree-sitter AST

**Зависимость:** Требует tree-sitter AST для вывода типов Generic и анализа flow

#### Задачи:

1. **Union Types с нормализацией** (4-5 дней)
   - ✅ Автоматическое упрощение: `String | String → String`
   - ✅ Упорядочивание: `Number | String → String | Number`
   - ✅ Дедупликация вложенных Union
   - ✅ Весовая система для вероятности типов
   - 🎯 **Цель:** Корректная обработка Union в 95% случаев

2. **Intersection Types** (4-5 дней)
   - ✅ Поддержка `A & B` — тип имеет свойства обоих
   - ✅ Проверка совместимости Intersection
   - ✅ Валидация методов/свойств для Intersection
   - 🎯 **Цель:** Расширение возможностей типизации

3. **Generic Types для коллекций** (5-6 дней) ← Требует tree-sitter AST
   - ✅ `Массив<Строка>` — типизированные массивы
   - ✅ `Соответствие<Строка, Число>` — типизированные словари
   - ✅ **Вывод типов из tree-sitter AST** (например, `arr.Add("text")` → `Массив<Строка>`)
   - ✅ Валидация операций над Generic типами
   - 🎯 **Цель:** Type safety для коллекций с type inference

4. **Nullable Types** (4-5 дней) ← Требует flow-sensitive analysis
   - ✅ `String | Null` — явная поддержка null
   - ✅ **Анализ null safety через CFG** (control flow graph)
   - ✅ Предупреждения о потенциальных NPE
   - ✅ Auto-completion с учётом nullable
   - 🎯 **Цель:** Предотвращение ошибок с null через flow analysis

**Результат Milestone 2.3:**
- ✅ Union Types полностью работают
- ✅ Intersection Types поддерживаются
- ✅ Generic Types для коллекций
- ✅ Null safety анализ

---

### 📈 Milestone 2.4: Performance & Caching (1.5 недели)

**Приоритет:** 🟠 ВЫСОКИЙ — критично для работы с реальными проектами

**Можно делать параллельно с Milestone 2.5**

#### Задачи:

1. **Межсессионное кеширование** (1 неделя)
   - ✅ Кеш AST деревьев в `.bsl_cache/ast/`
   - ✅ Кеш результатов анализа в `.bsl_cache/analysis/`
   - ✅ Инвалидация при изменении файлов (по hash)
   - ✅ TTL для устаревших кешей
   - 🎯 **Цель:** Загрузка из кеша < 50ms

2. **Параллельный анализ проектов** (1 неделя)
   - ✅ Multi-threaded анализ файлов через `rayon`
   - ✅ Прогресс-бар для больших проектов
   - ✅ Graceful degradation при ошибках
   - 🎯 **Цель:** Анализ 1000 файлов < 30 секунд

**Результат Milestone 2.4:**
- ✅ Кеш работает между запусками
- ✅ Анализ больших проектов быстрый
- ✅ Оптимизация памяти

---

### 🎨 Milestone 2.5: Унификация визуализации типов (1.5 недели)

**Приоритет:** 🟡 СРЕДНИЙ — улучшает UX, не блокирует функциональность

**Можно делать параллельно с Milestone 2.4**

**Проблема:** Сейчас визуализация типов реализована **тремя разными способами** без переиспользования:

1. **Web Frontend (Leptos WASM)** — `frontend/src/components/`
   - `TypeCard` — карточное представление
   - `TypeTable` — табличное представление
   - `GraphView` — граф типов (заглушка)
   - **DTOs:** `frontend/src/api/types.rs` (клиентские структуры)

2. **VSCode Extension (TypeScript)** — `vscode-extension/src/webviews/`
   - 7 HTML-генераторов: `getTypeInfoWebviewContent()`, `getMethodInfoWebviewContent()`, etc.
   - **Простой HTML** с inline стилями
   - **Проблема:** каждый webview генерирует свой HTML

3. **Backend DTOs** — `shared/src/api/dtos.rs`
   - `TypeDto`, `AnalysisResultDto`, `MetricsDto`
   - **camelCase** для JSON API
   - **Используется:** Web API `/api/types`, `/api/search`

**Проблемы унификации:**

| Аспект | Web Frontend | VSCode Extension | Backend DTOs |
|--------|--------------|------------------|---------------|
| **Язык** | Rust (Leptos) | TypeScript HTML | Rust (serde) |
| **Структуры** | `TypeInfo` | Нет типов, строки | `TypeDto` |
| **Визуализация** | Leptos components | HTML генераторы | N/A |
| **Стили** | CSS файл | Inline CSS | N/A |
| **Переиспользование** | ❌ Нет | ❌ Нет | ✅ Да (DTOs) |

#### Задачи унификации:

1. **Создать единый TypeVisualization компонент** (1 неделя) ✅ **ЗАВЕРШЕНО 2025-10-05**
   - ✅ Переиспользуемый Rust крейт `type-visualization`
   - ✅ Trait `TypeRenderer` для разных форматов
   - ✅ Реализации: `HtmlRenderer`, `JsonRenderer`, `MarkdownRenderer`
   - ✅ Общие CSS стили в одном месте
   - ✅ Интегрирован в Backend LSP Server
   - ✅ Интегрирован в VSCode Extension через LSP
   - ✅ Удалено 686 строк дублированного TypeScript кода
   - 📄 **Отчёт:** [TYPE_VISUALIZATION_INTEGRATION_COMPLETE.md](docs/TYPE_VISUALIZATION_INTEGRATION_COMPLETE.md)
   - 🎯 **Цель:** DRY — один компонент для всех UI ✅

2. **TypeScript интеграция для VSCode Extension** (2 часа) ✅ **ЗАВЕРШЕНО 2025-10-05**
   - ⚠️ **УСТАРЕЛО:** Заменено на интеграцию Rust TypeVisualization через LSP
   - ❌ **УДАЛЕНО:** `vscode-extension/src/visualization/` (686 строк)
   - ✅ **СОЗДАНО:** `vscode-extension/src/lsp/typeVisualization.ts` (TypeScript wrapper для LSP)
   - ✅ **ОБНОВЛЕНО:** `vscode-extension/src/webviews/webviewContent.ts` (использует Rust через LSP)
   - ✅ LSP request `bsl/renderTypeHtml` добавлен в `backend/src/bin/lsp_server.rs`
   - ✅ Поддержка тем: light/dark/high-contrast через LSP
   - 📄 **Отчёт:** [TYPE_VISUALIZATION_INTEGRATION_COMPLETE.md](docs/TYPE_VISUALIZATION_INTEGRATION_COMPLETE.md)
   - 🎯 **Цель:** Идентичный вид в Web и VSCode через единый Rust компонент ✅

3. **Унифицировать DTOs** (2 дня) ✅ **ЗАВЕРШЕНО 2025-10-05**
   - ✅ Удалён `frontend/src/api/types.rs` (368 строк дубликатов)
   - ✅ Единственный источник истины: `shared/src/api/dtos.rs`
   - ✅ Frontend использует extension traits в `frontend/src/api/extensions.rs`
   - ✅ Type aliases для обратной совместимости (TypeInfo = TypeDto)
   - ✅ Обновлены все 11+ компонентов frontend
   - ✅ Компиляция Rust workspace: 0 ошибок, 0 warnings
   - 🎯 **Цель:** Нет дублирования структур ✅

4. **Добавить темизацию** (2 дня) ✅ **ЗАВЕРШЕНО 2025-10-05**
   - ✅ Поддержка VS Code темы (light/dark/high-contrast)
   - ✅ CSS переменные вместо жёстких цветов
   - ✅ Автоопределение темы через `var(--vscode-*)` и auto-detect в TypeScript
   - ✅ ThemeMode enum в Rust TypeVisualization
   - ✅ Передача темы через LSP request параметры
   - 🎯 **Цель:** Адаптивность к теме редактора ✅

---

### 📊 Итоговая статистика Milestone 2.5

**Удалено дублированного кода:**
- ❌ `frontend/src/api/types.rs` — 368 строк
- ❌ `vscode-extension/src/visualization/HtmlRenderer.ts` — 316 строк
- ❌ `vscode-extension/src/visualization/Theme.ts` — 370 строк
- **ИТОГО: 1054 строки дубликатов удалено**

**Создано интеграционного кода:**
- ✅ `frontend/src/api/extensions.rs` — 280 строк (extension traits)
- ✅ `vscode-extension/src/lsp/typeVisualization.ts` — 145 строк (LSP wrapper)
- ✅ `backend/src/bin/lsp_server.rs` — ~80 строк (LSP handler)
- ✅ Документация — ~200 строк
- **ИТОГО: ~705 строк чистого интеграционного кода**

**Результат:**
- 🎯 **-349 строк кода** при улучшенной архитектуре
- ✅ **100% DRY** — единый источник истины для DTOs и визуализации
- ✅ **0 ошибок, 0 warnings** в Rust и TypeScript
- ✅ Все 4 задачи Milestone 2.5 выполнены

**Статус Milestone 2.5:** ✅ **ПОЛНОСТЬЮ ЗАВЕРШЁН 2025-10-05**

**Архитектура после унификации:**

#### 📊 Место в архитектурной диаграмме

`type-visualization` — это **Presentation Helper Layer** (вспомогательный слой представления):

```mermaid
graph TB
    subgraph "🎯 System Layer"
        SystemCoordinator["🎯 SystemCoordinator"]
        AnalysisCache["💾 AnalysisCache"]
        ParserCoordinator["🎨 ParserCoordinator"]
        BasicObservability["📊 BasicObservability"]
    end

    subgraph "🌐 Presentation Layer"
        LSP["🔌 LSP Server<br/>(bsl-lsp-server процесс)"]
        WebServer["🌐 Web Server<br/>(bsl-web-server процесс)<br/>├── REST API<br/>└── Static files"]
        Frontend["🖥️ Frontend UI<br/>(Leptos WASM в браузере)"]
        VSCode["📦 VSCode Extension<br/>(TypeScript)"]
        CLI["⚙️ CLI Tool"]
    end

    subgraph "🎨 Presentation Helper Layer ✅ ИНТЕГРИРОВАН"
        TypeViz["🎨 type-visualization<br/>├── TypeRenderer trait<br/>├── HtmlRenderer ✅ LSP<br/>├── JsonRenderer ⏳ Web API<br/>└── MarkdownRenderer"]
    end

    subgraph "🔧 Application Layer"
        TypeSystemService["🎭 TypeSystemService"]
        AnalysisEngine["🚀 AnalysisEngine"]
    end

    subgraph "🧠 Domain Layer"
        TypeResolver["🧠 TypeResolver"]
        TypeMetadataLookup["🔍 TypeMetadataLookup"]
        TypeRepository["📚 TypeRepository"]
    end

    subgraph "💾 Data Layer"
        PlatformTypes["📄 Platform Types"]
        ConfigData["⚙️ Configuration"]
    end

    subgraph "📄 API DTOs"
        DTOs["shared/api/dtos.rs<br/>├── TypeDto<br/>├── AnalysisResultDto<br/>└── MetricsDto"]
    end

    %% Flow - System
    SystemCoordinator --> AnalysisCache
    SystemCoordinator --> ParserCoordinator
    SystemCoordinator --> BasicObservability
    SystemCoordinator --> TypeSystemService

    %% Flow - Presentation → Helper (TypeVisualization)
    LSP --> TypeViz
    VSCode --> LSP
    TypeViz -.-> DTOs

    %% Flow - Presentation → Application
    LSP --> TypeSystemService
    WebServer --> TypeSystemService
    Frontend --> WebServer
    Frontend --> DTOs
    CLI --> AnalysisEngine

    TypeSystemService --> AnalysisEngine
    TypeSystemService --> AnalysisCache
    TypeSystemService --> DTOs

    %% Flow - Application → Domain
    AnalysisEngine --> TypeResolver
    AnalysisEngine --> ParserCoordinator
    TypeSystemService -.-> TypeMetadataLookup
    AnalysisEngine -.-> TypeMetadataLookup

    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository
    TypeRepository --> PlatformTypes
    TypeRepository --> ConfigData

    %% Styling
    classDef systemStyle fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef presentationStyle fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef helperStyle fill:#fff9c4,stroke:#f57f17,stroke-width:3px,stroke-dasharray: 5 5
    classDef applicationStyle fill:#e8f5e8,stroke:#388e3c,stroke-width:2px
    classDef domainStyle fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef dataStyle fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    classDef dtoStyle fill:#e1f5fe,stroke:#0277bd,stroke-width:2px

    class SystemCoordinator,AnalysisCache,ParserCoordinator,BasicObservability systemStyle
    class LSP,WebServer,Frontend,VSCode,CLI presentationStyle
    class TypeViz helperStyle
    class TypeSystemService,AnalysisEngine applicationStyle
    class TypeResolver,TypeMetadataLookup,TypeRepository domainStyle
    class PlatformTypes,ConfigData dataStyle
    class DTOs dtoStyle
```

**Легенда:**
- 🎯 **Сплошные линии** — реализованная архитектура
- 🎨 **Пунктирные линии** — вспомогательные связи (TypeViz использует DTOs для рендеринга)

**Потоки данных (реальная реализация):**
1. **VSCode Extension** → LSP Server → TypeVisualization (HtmlRenderer) → HTML webview
2. **Browser** → Web Server (static files) → Frontend (Leptos WASM) → Web Server (REST API) → DTOs
3. **CLI Tool** → AnalysisEngine → DTOs → консольный вывод

**Важно:** Web Server = ОДИН процесс (bsl-web-server) с двумя функциями:
- REST API endpoints (`/api/*`) — возвращает JSON DTOs
- Static file server (`/*`) — отдаёт `index.html`, `*.wasm`, `*.js` файлы Frontend

**Использование TypeVisualization по компонентам:**

| Компонент | Использует TypeViz? | Как визуализирует? | Почему? |
|-----------|---------------------|-------------------|---------|
| **LSP Server** | ✅ HtmlRenderer | LSP request → Rust HTML | Единая HTML генерация для VSCode |
| **VSCode Extension** | ✅ Вызывает через LSP | TypeScript → LSP → Rust | Избегаем дублирования TypeScript кода |
| **Web Server (API)** | ⏳ JsonRenderer (TODO) | Сериализация DTOs | Форматирование JSON ответов |
| **Web Server (Static)** | ❌ НЕТ | Отдаёт WASM файлы | Просто file serving |
| **Frontend (Leptos WASM)** | ❌ НЕТ | Leptos компоненты | Реактивный UI требует компонентов, не строк |
| **CLI** | ⏳ Планируется | Консольный вывод | Форматирование для терминала |

**Архитектурное решение для Frontend:**

**Почему Frontend НЕ использует HtmlRenderer:**
- ✅ **Leptos = компонентная модель** — как React/Vue, требует JSX-подобных view! макросов
- ✅ **Реактивность** — `Signal<TypeDto>` автообновление UI при изменении данных
- ✅ **Type safety** — проверка типов на этапе компиляции
- ✅ **XSS защита** — автоматический escaping в Leptos
- ❌ **HtmlRenderer = статичные строки** — потеря всех преимуществ Leptos

**Пример:**
```rust
// ✅ ПРАВИЛЬНО: Leptos компоненты
view! {
    <div class="type-card">
        {move || type_info.get().name}  // Реактивность!
    </div>
}

// ❌ НЕПРАВИЛЬНО: HtmlRenderer в Leptos
let html = renderer.render(&type_dto);  // Статичная строка
view! { <div inner_html={html}></div> } // Потеря реактивности
```

**Ключевые характеристики Presentation Helper Layer:**
- ✅ Обслуживает специфичные Presentation адаптеры (LSP, CLI)
- ✅ Не содержит бизнес-логики, только formatting
- ✅ Зависит от DTOs (контрактов), а не от Domain/Application
- ✅ Легко тестируется (input: DTO → output: HTML/JSON/Markdown)
- ✅ Следует принципу DRY для статичного UI кода
- ⚠️ **НЕ используется в реактивных фреймворках** (Leptos, React, Vue)

---

**Структура крейта:**

```
type-visualization/          # ✅ РЕАЛИЗОВАНО: Отдельный крейт
  ├── src/
  │   ├── renderers/
  │   │   ├── html.rs         # ✅ HtmlRenderer
  │   │   ├── json.rs         # ✅ JsonRenderer
  │   │   └── markdown.rs     # ✅ MarkdownRenderer
  │   ├── theme.rs            # ✅ ThemeMode (Light/Dark/HighContrast)
  │   ├── traits.rs           # ✅ TypeRenderer trait
  │   └── lib.rs
  └── Cargo.toml

Backend LSP Server (bsl-lsp-server):
  - ✅ HtmlRenderer для VSCode webviews (через LSP request `bsl/renderTypeHtml`)
  - ✅ ThemeMode auto-detection из VSCode

Backend Web Server (bsl-web-server):
  - ✅ REST API endpoints (`/api/*`) — возвращает shared DTOs (JSON)
  - ✅ Static file server (`/*`) — отдаёт Leptos WASM файлы
  - ⏳ JsonRenderer для форматирования (готов, но не подключён)

VSCode Extension:
  - ✅ LSP request → Rust HtmlRenderer → HTML string
  - ✅ Отображает в webview panel
  - ✅ Темы через LSP параметры (light/dark/high-contrast)
  - ❌ TypeScript HtmlRenderer УДАЛЁН (686 строк)

Frontend (Leptos WASM):
  - ✅ Leptos реактивные компоненты (НЕ HtmlRenderer!)
  - ✅ shared DTOs + extension traits
  - ✅ Декларативный UI через view! макрос
  - 💡 HtmlRenderer НЕ нужен (Leptos = компонентная модель как React)
```

**Реализованный Trait интерфейс:**

```rust
// type-visualization/src/traits.rs ✅ РЕАЛИЗОВАНО
pub trait TypeRenderer {
    fn render(&self, data: &TypeDto) -> String;
}

// type-visualization/src/renderers/html.rs ✅ РЕАЛИЗОВАНО
pub struct HtmlRenderer {
    options: RenderOptions,
}

pub struct RenderOptions {
    pub theme: ThemeMode,           // Light/Dark/HighContrast/Auto
    pub syntax_highlight: bool,
    pub enable_links: bool,
    pub compact: bool,
}

impl TypeRenderer for HtmlRenderer {
    fn render(&self, data: &TypeDto) -> String {
        // ✅ Генерация HTML с темизацией и стилями
    }
}

// type-visualization/src/renderers/json.rs ✅ РЕАЛИЗОВАНО
pub struct JsonRenderer {
    options: RenderOptions,
}

impl TypeRenderer for JsonRenderer {
    fn render(&self, data: &TypeDto) -> String {
        // ✅ Форматирование JSON с pretty-print
    }
}
```

**Результат Milestone 2.5:**
- ✅ Один источник визуализации для LSP/VSCode (HtmlRenderer)
- ✅ Frontend использует Leptos компоненты (правильная архитектура)
- ✅ Удалено 1054 строки дублированного кода
- ✅ 100% DRY для DTOs и LSP визуализации
- ⚠️ **Частичная унификация визуальных констант** — цвета/spacing дублируются между VSCode и Frontend

### 🎨 Milestone 2.6: Design System унификация (1 неделя) ⏳ ПЛАНИРУЕТСЯ

**Проблема:**
- ❌ Цвета certainty (high/medium/low) хардкодятся в Rust и CSS
- ❌ Spacing, размеры карточек дублируются
- ❌ Темы (light/dark/high-contrast) имеют разные реализации
- ❌ Нет единого источника истины для визуальных констант

**Решение: Design System с генерацией кода**

**Структура:**
```
bsl-design-system/
  ├── tokens.json          # ✅ Единый источник визуальных констант
  │   ├── colors           # Палитра (certainty, categories, themes)
  │   ├── spacing          # Отступы, размеры
  │   ├── typography       # Шрифты, размеры текста
  │   └── shadows          # Тени для карточек
  │
  ├── build.rs             # ✅ Генератор кода
  │   ├── → src/constants.rs    (Rust константы)
  │   ├── → dist/unified.css    (CSS variables)
  │   ├── → dist/tokens.ts      (TypeScript types)
  │
  └── templates/           # Шаблоны генерации
      ├── rust.hbs
      ├── css.hbs
      └── typescript.hbs
```

**Пример tokens.json:**
```json
{
  "colors": {
    "certainty": {
      "high": { "value": "#28a745", "description": "90-100% уверенность" },
      "medium": { "value": "#ffc107", "description": "50-89% уверенность" },
      "low": { "value": "#dc3545", "description": "<50% уверенность" }
    },
    "category": {
      "platform": "#2196F3",
      "configuration": "#4CAF50",
      "union": "#FF9800",
      "dynamic": "#9E9E9E"
    },
    "theme": {
      "light": {
        "background": "#ffffff",
        "foreground": "#000000",
        "card-bg": "#f5f5f5",
        "card-border": "#e0e0e0"
      },
      "dark": {
        "background": "#1e1e1e",
        "foreground": "#ffffff",
        "card-bg": "#252526",
        "card-border": "#3e3e42"
      }
    }
  },
  "spacing": {
    "card-padding": "16px",
    "card-gap": "12px",
    "card-border-radius": "8px"
  },
  "typography": {
    "font-family": "'Segoe UI', system-ui, sans-serif",
    "font-size-base": "14px",
    "font-size-heading": "18px"
  }
}
```

**Генерируемый код:**

**1. Rust константы (для HtmlRenderer):**
```rust
// bsl-design-system/src/constants.rs (автогенерация)
pub mod colors {
    pub const CERTAINTY_HIGH: &str = "#28a745";
    pub const CERTAINTY_MEDIUM: &str = "#ffc107";
    pub const CERTAINTY_LOW: &str = "#dc3545";

    pub const CATEGORY_PLATFORM: &str = "#2196F3";
    pub const CATEGORY_CONFIGURATION: &str = "#4CAF50";
}

pub mod spacing {
    pub const CARD_PADDING: &str = "16px";
    pub const CARD_GAP: &str = "12px";
}
```

**2. CSS variables (для Frontend Leptos):**
```css
/* bsl-design-system/dist/unified.css (автогенерация) */
:root {
  /* Colors - Certainty */
  --certainty-high: #28a745;
  --certainty-medium: #ffc107;
  --certainty-low: #dc3545;

  /* Colors - Category */
  --category-platform: #2196F3;
  --category-configuration: #4CAF50;

  /* Spacing */
  --card-padding: 16px;
  --card-gap: 12px;
  --card-border-radius: 8px;
}

[data-theme="dark"] {
  --background: #1e1e1e;
  --foreground: #ffffff;
  --card-bg: #252526;
  --card-border: #3e3e42;
}
```

**3. TypeScript types (для VSCode Extension):**
```typescript
// bsl-design-system/dist/tokens.ts (автогенерация)
export const DesignTokens = {
  colors: {
    certainty: {
      high: '#28a745',
      medium: '#ffc107',
      low: '#dc3545',
    },
    category: {
      platform: '#2196F3',
      configuration: '#4CAF50',
    }
  },
  spacing: {
    cardPadding: '16px',
    cardGap: '12px',
  }
} as const;
```

**Использование:**

**В HtmlRenderer (type-visualization):**
```rust
use bsl_design_system::colors::{CERTAINTY_HIGH, CERTAINTY_MEDIUM, CERTAINTY_LOW};

impl HtmlRenderer {
    fn render_certainty_badge(&self, certainty: u8) -> String {
        let color = match certainty {
            90..=100 => CERTAINTY_HIGH,
            50..=89 => CERTAINTY_MEDIUM,
            _ => CERTAINTY_LOW,
        };
        format!(r#"<span style="color: {}">{}</span>"#, color, certainty)
    }
}
```

**В Frontend (Leptos):**
```rust
// Импортирует unified.css
use_stylesheet!("bsl-design-system/dist/unified.css");

view! {
    <div class="type-card" style="padding: var(--card-padding)">
        <span class="certainty-high">{certainty}</span>
    </div>
}
```

**В VSCode Extension (TypeScript):**
```typescript
import { DesignTokens } from 'bsl-design-system/dist/tokens';

const certaintyColor = certainty >= 90
  ? DesignTokens.colors.certainty.high
  : DesignTokens.colors.certainty.medium;
```

**Задачи Milestone 2.6:**

1. **Создать структуру bsl-design-system** (2 дня)
   - Крейт `bsl-design-system`
   - Файл `tokens.json` с визуальными константами
   - Зависимости: `serde`, `serde_json`, `handlebars` (для шаблонов)

2. **Реализовать build.rs генератор** (3 дня)
   - Чтение `tokens.json`
   - Генерация Rust `src/constants.rs`
   - Генерация CSS `dist/unified.css`
   - Генерация TypeScript `dist/tokens.ts`

3. **Интеграция в существующий код** (2 дня)
   - Обновить `type-visualization` для использования констант
   - Обновить `frontend` для импорта `unified.css`
   - Обновить `vscode-extension` для использования TypeScript токенов
   - Удалить хардкоженные константы

4. **Тестирование и валидация** (1 день)
   - Визуальное тестирование: VSCode webview идентичен Frontend
   - Темы работают одинаково
   - Изменение токена обновляет все компоненты

**Результат Milestone 2.6:**
- ✅ 100% унификация визуальных констант (цвета, spacing, typography)
- ✅ Единый источник истины: `tokens.json`
- ✅ Автоматическая генерация кода для Rust/CSS/TypeScript
- ✅ Изменение дизайна в одном месте → обновляется везде
- ✅ Идентичный визуальный вид VSCode webview и Frontend
- ✅ Type-safe доступ к токенам (TypeScript types, Rust константы)

**Что всё ещё остаётся разным (и это правильно):**
- ❌ Рендеринг логика: HtmlRenderer (статичный) vs Leptos (реактивный)
- ✅ Но цвета, размеры, spacing — ИДЕНТИЧНЫ

---

### 🔧 Milestone 2.7: TreeSitterAdapter — Завершение реализации (2 недели) 🚨 **КРИТИЧЕСКИЙ**

**Приоритет:** 🔴 КРИТИЧЕСКИЙ — блокирует все LSP features (hover, completion, diagnostics)

#### Архитектурная диаграмма (текущее состояние после рефакторинга 2025-10-06):

```mermaid
graph TB
    subgraph "🎯 System Layer (backend)"
        SystemCoordinator["🎯 SystemCoordinator<br/>- DI management<br/>- Lifecycle"]
        AnalysisCache["💾 AnalysisCache<br/>- LRU cache<br/>- TTL"]
        ParserCoordinator["🎨 ParserCoordinator<br/>- TreeSitter (primary)<br/>- Regex fallback<br/>✅ returns AST"]
        BasicObservability["📊 BasicObservability<br/>- Logging<br/>- Metrics"]
    end

    subgraph "🌐 Presentation Layer"
        LSPServer["🔌 LSP Server<br/>✅ hover работает"]
        WebServer["🌐 Web Server<br/>REST API"]
        VSCode["📦 VSCode Extension"]
        CLI["⚙️ CLI Tool<br/>❌ MOCK analyze"]
    end

    subgraph "🎨 Helper Layer"
        TypeViz["🎨 type-visualization<br/>✅ HtmlRenderer<br/>JsonRenderer"]
    end

    subgraph "🔧 Application Layer"
        subgraph "backend"
            TypeSystemService["🎭 TypeSystemService<br/>✅ expression_to_type_name()<br/>✅ использует AnalysisEngine"]
        end
        subgraph "shared"
            AnalysisEngine["🚀 AnalysisEngine<br/>✅ resolve_type()<br/>❌ analyze_file() = MOCK"]
        end
    end

    subgraph "🧠 Domain Layer (shared)"
        TypeResolver["🧠 TypeResolver<br/>✅ резолвит типы"]
        TypeMetadataLookup["🔍 TypeMetadataLookup"]
        TypeRepository["📚 TypeRepository<br/>✅ 3927 типов"]
    end

    subgraph "💾 Data Layer (shared)"
        PlatformTypes["📄 Platform Types<br/>✅ включая примитивы"]
        ConfigData["⚙️ Configuration"]
    end

    subgraph "📄 DTOs"
        DTOs["shared/api/dtos.rs"]
    end

    SystemCoordinator --> AnalysisCache
    SystemCoordinator --> ParserCoordinator
    SystemCoordinator --> BasicObservability
    SystemCoordinator --> TypeSystemService

    LSPServer --> TypeSystemService
    LSPServer --> TypeViz
    WebServer --> TypeSystemService
    VSCode --> LSPServer
    CLI --> AnalysisEngine

    TypeSystemService --> AnalysisEngine
    TypeSystemService --> AnalysisCache
    TypeSystemService --> ParserCoordinator
    TypeSystemService -.-> TypeMetadataLookup

    AnalysisEngine --> TypeResolver
    AnalysisEngine -.-> TypeMetadataLookup

    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository
    TypeRepository --> PlatformTypes
    TypeRepository --> ConfigData

    TypeViz -.-> DTOs
    TypeSystemService --> DTOs

    classDef systemStyle fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef presentationStyle fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef helperStyle fill:#fff9c4,stroke:#f57f17,stroke-width:2px
    classDef applicationStyle fill:#e8f5e8,stroke:#388e3c,stroke-width:2px
    classDef domainStyle fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef dataStyle fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    classDef dtoStyle fill:#e1f5fe,stroke:#0277bd,stroke-width:2px

    class SystemCoordinator,AnalysisCache,ParserCoordinator,BasicObservability systemStyle
    class LSPServer,WebServer,VSCode,CLI presentationStyle
    class TypeViz helperStyle
    class TypeSystemService,AnalysisEngine applicationStyle
    class TypeResolver,TypeMetadataLookup,TypeRepository domainStyle
    class PlatformTypes,ConfigData dataStyle
    class DTOs dtoStyle
```

**Ключевые улучшения (2025-10-06):**
- ✅ **Hover работает через AnalysisEngine** — TypeSystemService.get_hover_info() → expression_to_type_name() → AnalysisEngine.resolve_type()
- ✅ **Примитивные типы интегрированы** — Число/Строка/Булево резолвятся через TypeRepository (3927 типов из синтаксис-помощника)
- ✅ **Разделение ответственности** — Application Layer (маппинг AST→строка) + Domain Layer (резолв типа)
- ❌ **Проблема:** AnalysisEngine.analyze_file() ещё MOCK — требует AST в shared (см. Milestone 2.8)

**Текущее состояние (2025-10-06):**
- ✅ Tree-sitter-bsl v0.1.5 интегрирован и работает
- ✅ ParserCoordinator использует tree-sitter как primary парсер
- ✅ **ОБНОВЛЕНО:** TreeSitterAdapter расширен — поддержка 95% конструкций BSL
- ✅ **ОБНОВЛЕНО:** AST расширен — добавлены все недостающие Statement/Expression варианты
- ✅ **ОБНОВЛЕНО:** Workarounds исправлены — for/return/call используют правильные типы
- ⏳ **В ПРОЦЕССЕ:** Тестирование на реальных BSL файлах

**Что пропускается сейчас (из логов):**
```
DEBUG Skipping unknown statement type: PROCEDURE_KEYWORD at 2
DEBUG Skipping unknown statement type: FUNCTION_KEYWORD at 3
DEBUG Skipping unknown expression: expression
DEBUG Skipping unknown expression: arguments
DEBUG Skipping unknown statement type: ENDPROCEDURE_KEYWORD at 7
```

**Root cause:** TreeSitterAdapter не маппит node.kind() tree-sitter-bsl → наши AST структуры.

#### Задачи:

**1. Полный маппинг tree-sitter node kinds (5 дней)**

**Цель:** Поддержка всех основных конструкций BSL из grammar.js tree-sitter-bsl

**Подзадачи:**
- ✅ Изучить `grammar.js` tree-sitter-bsl для полного списка node kinds
- ✅ Реализовать конвертацию для процедур/функций:
  ```rust
  "procedure_declaration" | "PROCEDURE_KEYWORD" => Statement::ProcedureDecl
  "function_declaration" | "FUNCTION_KEYWORD" => Statement::FunctionDecl
  ```
- ✅ Реализовать конвертацию для выражений:
  - `"expression"` → Expression::*
  - `"call_expression"` → Expression::Call
  - `"member_expression"` → Expression::Access
  - `"binary_expression"` → Expression::BinaryOp
  - `"unary_expression"` → Expression::UnaryOp
  - `"new_expression"` → Expression::New
- ✅ Реализовать конвертацию для statements:
  - `"assignment_statement"` → Statement::Assignment
  - `"if_statement"` → Statement::If
  - `"for_statement"` → Statement::For
  - `"while_statement"` → Statement::While
  - `"return_statement"` → Statement::Return
  - `"try_statement"` → Statement::TryExcept

**Метрика успеха:** 90% BSL конструкций парсятся без "Skipping unknown"

**2. Тестирование на реальных BSL файлах (3 дня)**

**Цель:** Убедиться что adapter работает на production коде 1С

**Подзадачи:**
- ✅ Создать тестовый набор из 20+ реальных BSL файлов:
  - Формы документов
  - Модули объектов
  - Общие модули
  - Модули команд
- ✅ Unit-тесты для каждого node kind:
  ```rust
  #[test]
  fn test_parse_procedure_declaration() {
      let code = "Процедура Тест() КонецПроцедуры";
      let program = parse_bsl(code);
      assert_eq!(program.statements.len(), 1);
      assert!(matches!(program.statements[0], Statement::ProcedureDecl { .. }));
  }
  ```
- ✅ Интеграционные тесты с TypeSystemService
- ✅ Проверка hover на реальных файлах (должны показывать типы, а не "Dynamic type")

**Метрика успеха:** Hover показывает корректные типы в 80%+ случаев

**3. Оптимизация производительности (2 дня)**

**Цель:** Парсинг 10000 строк < 200ms (из Milestone 2.1)

**Подзадачи:**
- ✅ Профилирование TreeSitterAdapter (найти узкие места)
- ✅ Оптимизация аллокаций (использовать `Box::new` только где нужно)
- ✅ Оптимизация String cloning (использовать `&str` где возможно)
- ✅ Бенчмарки для разных размеров файлов:
  ```rust
  #[bench]
  fn bench_parse_10k_lines(b: &mut Bencher) {
      let code = include_str!("../fixtures/large_module.bsl");
      b.iter(|| parse_bsl(code));
  }
  ```

**Метрика успеха:** Парсинг 10000 строк < 200ms

**4. Документация и примеры (2 дня)**

**Цель:** Понятная документация для будущих contributors

**Подзадачи:**
- ✅ Документировать маппинг tree-sitter node kinds → AST в README
- ✅ Создать примеры использования TreeSitterAdapter
- ✅ Добавить диагностику для неподдерживаемых конструкций:
  ```rust
  warn!("Unsupported node kind '{}' at {}:{} - please file an issue",
        node.kind(), node.start_position().row, node.start_position().column);
  ```
- ✅ Создать checklist для добавления новых node kinds

**Метрика успеха:** Contributor может добавить новый node kind за 30 минут

**5. Исправление сломанных тестов (2 дня)**

**Цель:** Восстановить workspace test suite после рефакторинга

**Подзадачи:**
- ✅ Исправить импорты `TypeResolution`, `ResolutionResult` в тестах
- ✅ Обновить моки для нового AST
- ✅ Убедиться что `cargo test --workspace` проходит без ошибок

**Метрика успеха:** `cargo test --workspace` — 100% success

---

**Результат Milestone 2.7:**
- ✅ TreeSitterAdapter поддерживает 90% конструкций BSL
- ✅ Hover показывает реальные типы (не "Dynamic type")
- ✅ Парсинг 10000 строк < 200ms
- ✅ 80%+ test coverage для adapter
- ✅ Все workspace тесты проходят
- ✅ Готовность к Milestone 2.2 (VSCode Extension)

**Статус:** 🚧 **В ПРОЦЕССЕ** (80% выполнено, hover работает через AnalysisEngine)

---

### 🏗️ Milestone 2.8: Semantic IR Layer — Разделение Syntax и Semantics (1 неделя) 🎯 **АРХИТЕКТУРНЫЙ**

**Приоритет:** 🔴 КРИТИЧЕСКИЙ — устраняет циклическую зависимость и создаёт правильную архитектуру

**Текущая проблема:**
- ✅ LSP hover работает — реальный парсинг через TreeSitterAdapter
- ❌ CLI analyze_file() — MOCK, возвращает хардкод
- ❌ AnalysisEngine не может работать с AST (AST в backend, AnalysisEngine в shared)
- 🔴 **CRITICAL:** Прямой перенос AST в shared создаст циклическую зависимость `tree-sitter` ↔ `shared`
- 🔴 **CRITICAL:** CLI придётся зависеть от тяжёлого ParserCoordinator (~7-8 MB бинарник)

**Root cause:** Смешивание Syntax (AST) и Semantics (Domain) в одном слое

#### Архитектурное решение:

**Intermediate Representation (IR) — Semantic Layer между Syntax и Domain**

Правильная архитектура должна разделять:
1. **Syntax Layer (AST)** — структура программы, привязана к парсеру (backend)
2. **Semantic Layer (IR)** — семантическая модель, независимая от парсера (shared)
3. **Domain Layer (Types)** — типовая система и анализ (shared)

**Аналогия с другими проектами:**
- **rust-analyzer:** `syntax` (AST) → `hir` (HIR) → `ty` (Type System)
- **TypeScript:** `AST` → `Symbol` → `Type`
- **LLVM:** Source → AST → IR → Machine Code

#### Задачи:

**1. Создать Semantic IR в shared/src/ir/ (2 дня)**

**Цель:** Создать семантическое представление, независимое от парсера

**Подзадачи:**
- [ ] Создать `shared/src/ir/mod.rs` с SemanticProgram, SemanticNode, SymbolTable:
  ```rust
  /// Семантическое представление программы (не зависит от конкретного парсера)
  pub struct SemanticProgram {
      pub symbols: SymbolTable,
      pub nodes: Vec<SemanticNode>,
      pub source_info: SourceInfo, // Метаданные для debugging
  }

  /// Упрощённое семантическое представление элементов программы
  pub enum SemanticNode {
      VariableDeclaration {
          name: String,
          type_hint: Option<String>,
          is_export: bool,
      },
      Assignment {
          variable: String,
          value_type: String,
      },
      FunctionDeclaration {
          name: String,
          params: Vec<Parameter>,
          return_type: Option<String>,
      },
      FunctionCall {
          function: String,
          args: Vec<String>, // Типы аргументов
      },
      // ... минималистичный набор для анализа типов
  }

  pub struct SymbolTable {
      pub variables: HashMap<String, TypeHint>,
      pub functions: HashMap<String, FunctionSignature>,
  }
  ```
- [ ] Добавить `shared/src/ir/visitor.rs` с FlowSensitiveVisitor:
  ```rust
  pub trait SemanticVisitor {
      fn visit_node(&mut self, node: &SemanticNode, context: &mut FlowContext);
      fn enter_scope(&mut self);
      fn exit_scope(&mut self);
  }
  ```
- [ ] Экспортировать из `shared/src/lib.rs`: `pub mod ir;`

**Метрика успеха:** `shared/src/ir/` компилируется без зависимостей от backend/tree-sitter

**2. Создать AST → IR конвертер в backend (2 дня)**

**Цель:** Bridge между синтаксисом (AST) и семантикой (IR)

**Подзадачи:**
- [ ] Создать `backend/src/application/ast_to_ir.rs`:
  ```rust
  pub struct AstToIrConverter {
      symbol_table: SymbolTable,
  }

  impl AstToIrConverter {
      pub fn convert(ast: Program) -> Result<SemanticProgram> {
          let mut converter = Self {
              symbol_table: SymbolTable::new(),
          };

          let nodes = ast.statements
              .into_iter()
              .filter_map(|stmt| converter.convert_statement(stmt))
              .collect();

          Ok(SemanticProgram {
              symbols: converter.symbol_table,
              nodes,
              source_info: SourceInfo::from_ast(&ast),
          })
      }

      fn convert_statement(&mut self, stmt: Statement) -> Option<SemanticNode> {
          match stmt {
              Statement::VarDeclaration { name, type_hint } => {
                  self.symbol_table.register_variable(&name, type_hint.clone());
                  Some(SemanticNode::VariableDeclaration {
                      name,
                      type_hint,
                      is_export: false,
                  })
              }
              Statement::Assignment { target, value } => {
                  if let Expression::Identifier(var_name) = target {
                      let value_type = self.infer_expression_type(&value);
                      Some(SemanticNode::Assignment {
                          variable: var_name,
                          value_type,
                      })
                  } else {
                      None
                  }
              }
              // ... конвертация других statement типов
              _ => None,
          }
      }

      fn infer_expression_type(&self, expr: &Expression) -> String {
          match expr {
              Expression::Number(_) => "Число".to_string(),
              Expression::String(_) => "Строка".to_string(),
              Expression::Boolean(_) => "Булево".to_string(),
              Expression::New { type_name, .. } => type_name.clone(),
              Expression::Identifier(name) => {
                  self.symbol_table.get_variable_type(name)
                      .unwrap_or_else(|| "Unknown".to_string())
              }
              _ => "Dynamic".to_string(),
          }
      }
  }
  ```
- [ ] Обновить ParserCoordinator для возврата IR:
  ```rust
  pub fn parse_to_ir(&self, content: &str) -> Result<SemanticProgram> {
      let ast = self.parse(content)?; // Старый метод остаётся для обратной совместимости
      AstToIrConverter::convert(ast)
  }
  ```
- [ ] Добавить unit-тесты для конвертера

**Метрика успеха:** AST успешно конвертируется в IR, тесты проходят

**3. Создать Parser trait в shared (1 день)**

**Цель:** Dependency Inversion — любой парсер может производить IR

**Подзадачи:**
- [ ] Создать `shared/src/parsing/parser_trait.rs`:
  ```rust
  /// Абстракция парсера — возвращает семантическое представление
  pub trait Parser: Send + Sync {
      fn parse(&self, content: &str) -> Result<SemanticProgram, String>;

      fn supports_incremental(&self) -> bool {
          false // По умолчанию не поддерживает
      }
  }

  /// Lightweight wrapper для CLI (не тянет весь backend)
  pub struct LightweightParser {
      // Минимальная реализация на regex для быстрых задач
  }

  impl Parser for LightweightParser {
      fn parse(&self, content: &str) -> Result<SemanticProgram, String> {
          // Простой regex-based парсинг → IR
          todo!("Implement lightweight parsing")
      }
  }
  ```
- [ ] Реализовать `Parser` trait для ParserCoordinator:
  ```rust
  // backend/src/system/parser_coordinator.rs
  impl bsl_shared::parsing::Parser for ParserCoordinator {
      fn parse(&self, content: &str) -> Result<SemanticProgram, String> {
          self.parse_to_ir(content)
      }

      fn supports_incremental(&self) -> bool {
          true // TreeSitter поддерживает инкрементальный парсинг
      }
  }
  ```

**Метрика успеха:** CLI может использовать `Box<dyn Parser>` без зависимости от backend

**4. Обновить AnalysisEngine для работы с IR (2 дня)**

**Цель:** AnalysisEngine анализирует IR вместо AST

**Подзадачи:**
- [ ] Обновить `shared/src/engine.rs`:
  ```rust
  pub fn analyze_program(&self, program: SemanticProgram) -> CliAnalysisResult {
      let mut resolutions = HashMap::new();

      for node in program.nodes {
          match node {
              SemanticNode::VariableDeclaration { name, type_hint, .. } => {
                  if let Some(type_name) = type_hint {
                      let resolution = self.resolve_type(&type_name);
                      resolutions.insert(name, resolution);
                  }
              }
              SemanticNode::Assignment { variable, value_type } => {
                  let resolution = self.resolve_type(&value_type);
                  resolutions.insert(variable, resolution);
              }
              SemanticNode::FunctionCall { function, args } => {
                  // Анализ вызова функции с аргументами
                  let fn_resolution = self.resolve_type(&function);
                  // Проверка типов аргументов
              }
              _ => {}
          }
      }

      CliAnalysisResult {
          file_path: program.source_info.path,
          type_resolutions: resolutions.into_iter().collect(),
          analysis_duration_ms: 0, // TODO: timing
      }
  }

  pub async fn analyze_file<P, F>(
      &self,
      path: P,
      parser: &dyn Parser, // ✅ Dependency Injection через trait
  ) -> Result<CliAnalysisResult>
  where
      P: AsRef<Path>
  {
      let content = std::fs::read_to_string(path.as_ref())?;
      let program = parser.parse(&content)?;
      Ok(self.analyze_program(program))
  }
  ```
- [ ] Убрать MOCK implementation
- [ ] Добавить IR-кеш (опционально):
  ```rust
  pub struct AnalysisEngine {
      resolver: Arc<TypeResolver>,
      repository: Arc<dyn TypeRepository>,
      ir_cache: DashMap<PathBuf, (u64, SemanticProgram)>, // Кеш IR
  }
  ```

**Метрика успеха:** `analyze_file()` возвращает реальные типы из IR

**5. Обновить TypeSystemService для конвертации AST → IR (1 день)**

**Цель:** LSP hover продолжает работать через IR

**Подзадачи:**
- [ ] Обновить `backend/src/application/type_system_service.rs`:
  ```rust
  pub fn get_hover_info(&self, file_path: &str, line: usize, column: usize)
      -> Option<String>
  {
      // 1. Парсинг AST (как раньше)
      let ast = self.parser.parse(content)?;

      // 2. Конвертация AST → IR
      let ir = AstToIrConverter::convert(ast)?;

      // 3. Поиск нужного семантического узла по позиции курсора
      let node = ir.find_node_at_position(line, column)?;

      // 4. Резолв типа через AnalysisEngine
      match node {
          SemanticNode::VariableDeclaration { name, type_hint, .. } => {
              if let Some(type_name) = type_hint {
                  let resolution = self.analysis_engine.resolve_type(&type_name);
                  Some(self.format_type_for_hover(&type_name, &resolution))
              } else {
                  Some(format!("**Переменная:** `{}`\n\n*Тип не указан*", name))
              }
          }
          SemanticNode::Assignment { variable, value_type } => {
              let resolution = self.analysis_engine.resolve_type(&value_type);
              Some(format!("**Присваивание:** `{} = ...`\n\n{}",
                  variable,
                  self.format_type_for_hover(&value_type, &resolution)
              ))
          }
          _ => None
      }
  }
  ```
- [ ] Сохранить backward compatibility (старый `expression_to_type_name()` можно оставить)

**Метрика успеха:** LSP hover работает без регрессий

**6. Обновить CLI для использования Parser trait (1 день)**

**Цель:** Лёгкий CLI (~2-3 MB) вместо тяжёлого (~7-8 MB)

**Подзадачи:**
- [ ] Обновить `cli/src/main.rs`:
  ```rust
  use bsl_shared::parsing::{Parser, LightweightParser};
  use bsl_shared::engine::AnalysisEngine;

  #[tokio::main]
  async fn main() -> Result<()> {
      let args = Args::parse();

      // ✅ CLI использует лёгкий парсер без backend зависимости
      let parser: Box<dyn Parser> = if args.use_full_parser {
          // Опционально: можно подключить ParserCoordinator из backend
          Box::new(create_full_parser()?)
      } else {
          Box::new(LightweightParser::new())
      };

      let engine = AnalysisEngine::new(resolver, repository);

      for file in &args.files {
          let result = engine.analyze_file(file, parser.as_ref()).await?;
          println!("{:?}", result);
      }

      Ok(())
  }
  ```
- [ ] Обновить `cli/Cargo.toml` (убрать backend зависимость по умолчанию):
  ```toml
  [dependencies]
  bsl-shared = { path = "../shared" }

  [features]
  full-parser = ["bsl-backend"]  # Опциональная фича для полного парсера
  ```

**Метрика успеха:**
- `cargo build --bin bsl-type-check` → ~2-3 MB бинарник
- `cargo build --bin bsl-type-check --features full-parser` → ~7-8 MB бинарник

---

**Результат Milestone 2.8:**
- ✅ IR (Intermediate Representation) живёт в shared — разделение Syntax и Semantics
- ✅ AnalysisEngine.analyze_file() работает с реальным IR (не MOCK)
- ✅ CLI показывает реальные типы через лёгкий парсер (~2-3 MB бинарник)
- ✅ Parser trait — DI для разных парсеров (TreeSitter, Regex, Lightweight)
- ✅ Архитектура без циклических зависимостей
- ✅ LSP hover продолжает работать через AST → IR → TypeResolver
- ✅ Готовность к flow-sensitive анализу (Milestone 2.3)

**Диаграмма AFTER (Полная архитектура после Milestone 2.8 с IR Layer):**

```mermaid
graph TB
    subgraph "🎯 System Layer (в `backend/src/system`)"
        SystemCoordinator["🎯 SystemCoordinator"]
        AnalysisCache["💾 AnalysisCache"]
        ParserCoordinator["🎨 ParserCoordinator<br/>- TreeSitter + Regex<br/>✅ ПОСЛЕ 2.8: → IR через AstToIrConverter"]
        BasicObservability["📊 BasicObservability"]
    end

    subgraph "🌐 Presentation Layer (Адаптеры - разные процессы)"
        subgraph "LSP Process"
            LSPServer["🔌 LSP Server (backend)"]
            VSCode["📦 VSCode Extension (TypeScript)"]
        end

        subgraph "Web Process"
            WebServer["🌐 Web Server (backend)"]
            Frontend["🖥️ Frontend UI (Leptos WASM)"]
        end

        CLITool["⚙️ CLI Tool (cli)<br/>✅ ПОСЛЕ 2.8: LightweightParser (~2-3 MB)"]
    end

    subgraph "🎨 Helper Layer"
        TypeViz["🎨 type-visualization"]
    end

    subgraph "🔧 Application Layer"
        subgraph "`backend/src/application`"
            TypeSystemService["🎭 TypeSystemService<br/>✅ LSP hover через AST → IR"]
            AstToIr["🔄 AstToIrConverter<br/>✅ ПОСЛЕ 2.8: AST → IR bridge<br/>- Конвертирует синтаксис в семантику<br/>- Строит SymbolTable"]
        end
        subgraph "`shared/src/engine`"
            AnalysisEngine["🚀 AnalysisEngine<br/>✅ ПОСЛЕ 2.8: analyze_program(IR)<br/>- Работает с SemanticProgram<br/>- Не зависит от парсеров"]
        end
    end

    subgraph "🌟 Semantic Layer (✅ НОВЫЙ! shared/src/ir/)"
        IR["📄 Intermediate Representation<br/>✅ ПОСЛЕ 2.8: shared/src/ir/<br/>- SemanticProgram<br/>- SemanticNode (упрощённый набор)<br/>- SymbolTable<br/>- FlowSensitiveVisitor<br/>✨ Независим от парсера!"]

        ParserTrait["🔌 Parser trait<br/>✅ ПОСЛЕ 2.8: shared/src/parsing/<br/>- parse() → SemanticProgram<br/>- DI для разных парсеров<br/>- LightweightParser для CLI"]
    end

    subgraph "🧠 Domain Layer (в `shared/src/domain`)"
        TypeResolver["🧠 TypeResolver"]
        TypeMetadataLookup["🔍 TypeMetadataLookup"]
        TypeRepository["📚 TypeRepository (3927 типов)"]
    end

    subgraph "💾 Data Layer"
        PlatformTypes["📄 Platform Types<br/>(Syntax Helper: Строка, Число, etc.)"]
        ConfigData["⚙️ Configuration"]
    end

    subgraph "📄 DTOs"
        DTOs["shared/api/dtos.rs"]
    end

    %% System coordination
    SystemCoordinator --> AnalysisCache
    SystemCoordinator --> ParserCoordinator
    SystemCoordinator --> BasicObservability
    SystemCoordinator --> TypeSystemService

    %% Presentation → Application
    LSPServer --> TypeSystemService
    WebServer --> TypeSystemService
    VSCode --> LSPServer
    Frontend --> WebServer
    CLITool --> AnalysisEngine

    %% Helper layer
    LSPServer --> TypeViz
    TypeViz -.-> DTOs

    %% Application → Semantic Layer (КЛЮЧЕВОЕ ИЗМЕНЕНИЕ 2.8)
    TypeSystemService --> AnalysisEngine
    TypeSystemService --> AstToIr
    TypeSystemService --> ParserCoordinator

    ParserCoordinator -.->|"✅ ПОСЛЕ 2.8: converts AST"| AstToIr
    AstToIr -.->|"produces"| IR

    %% CLI использует Parser trait (Dependency Injection)
    CLITool -.->|"✅ uses ParserTrait"| ParserTrait
    ParserTrait -.->|"returns"| IR

    %% ParserCoordinator implements Parser trait
    ParserCoordinator -.->|"✅ implements"| ParserTrait

    %% AnalysisEngine работает с IR
    AnalysisEngine -.->|"✅ analyzes"| IR
    AnalysisEngine --> TypeResolver

    %% Domain layer
    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository
    TypeSystemService -.-> TypeMetadataLookup
    AnalysisEngine -.-> TypeMetadataLookup

    TypeRepository --> PlatformTypes
    TypeRepository --> ConfigData

    TypeSystemService --> DTOs
    TypeSystemService --> AnalysisCache

    %% Styling
    classDef systemStyle fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef presentationStyle fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef helperStyle fill:#fff9c4,stroke:#f57f17,stroke-width:2px
    classDef applicationStyle fill:#e8f5e8,stroke:#388e3c,stroke-width:2px
    classDef semanticStyle fill:#ffe0b2,stroke:#e65100,stroke-width:4px,stroke-dasharray: 5 5
    classDef domainStyle fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef dataStyle fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    classDef dtoStyle fill:#e1f5fe,stroke:#0277bd,stroke-width:2px

    class SystemCoordinator,AnalysisCache,ParserCoordinator,BasicObservability systemStyle
    class LSPServer,WebServer,Frontend,VSCode,CLITool presentationStyle
    class TypeViz helperStyle
    class TypeSystemService,AstToIr,AnalysisEngine applicationStyle
    class IR,ParserTrait semanticStyle
    class TypeResolver,TypeMetadataLookup,TypeRepository domainStyle
    class PlatformTypes,ConfigData dataStyle
    class DTOs dtoStyle
```

**Ключевые изменения в архитектуре:**

1. **✅ Semantic Layer (IR)** — новый слой между Syntax (AST) и Domain (Types):
   - `SemanticProgram` — упрощённое представление программы
   - `SemanticNode` — минималистичный набор узлов для анализа типов
   - `SymbolTable` — таблица символов с типовыми аннотациями

2. **✅ AST остаётся в backend** — избегаем циклической зависимости tree-sitter ↔ shared

3. **✅ AstToIrConverter** — bridge в Application Layer, конвертирует AST → IR

4. **✅ Parser trait** — Dependency Inversion для лёгкого CLI:
   - `ParserCoordinator` (backend) implements `Parser`
   - `LightweightParser` (shared) — простой парсер для CLI без backend зависимости

5. **✅ AnalysisEngine работает с IR** — больше не MOCK, анализирует SemanticProgram

6. **✅ Разделение Syntax и Semantics:**
   ```
   Source Code → AST (Syntax, backend) → IR (Semantics, shared) → Types (Domain, shared)
   ```

**Статус:** ⏳ **ПЛАНИРУЕТСЯ** (после завершения Milestone 2.7)

---

### 🎯 Результаты Версии 2.0 (через 8-10 недель)

**Timeline обновлён:**
```
Неделя 1-3:   🧠 Milestone 2.1 - Tree-sitter Integration (80% ВЫПОЛНЕНО ✅)
Неделя 4-5:   🔧 Milestone 2.7 - TreeSitterAdapter + hover (80% ВЫПОЛНЕНО 🚧)
Неделя 5-6:   🏗️ Milestone 2.8 - Semantic IR Layer (КРИТИЧЕСКИЙ ⏳)
Неделя 6-7:   📦 Milestone 2.2 - VSCode Extension (КРИТИЧЕСКИЙ)
Неделя 8-10:  🔧 Milestone 2.3 - Advanced Type System + Flow-sensitive (ВЫСОКИЙ)
Неделя 11-12: 📈 Milestone 2.4 - Performance Optimization (СРЕДНИЙ)
ЗАВЕРШЕНО:    🎨 Milestone 2.5 - Унификация визуализации (ЗАВЕРШЁН ✅)
ПЛАНИРУЕТСЯ:  🎨 Milestone 2.6 - Design System (ПЛАНИРУЕТСЯ ⏳)
```

**Технические метрики:**
- ⚠️ **Tree-sitter-bsl v0.1.5 интегрирован** — парсинг работает, но adapter неполный (30%)
- ❌ **Flow-sensitive analysis > 70%** — БЛОКИРОВАН (требует полный AST)
- ✅ **Инкрементальный парсинг < 10ms** — LSP performance (реализовано)
- ⏳ VSCode Extension: 4-5 MB (вместо 30 MB) — ПЛАНИРУЕТСЯ Milestone 2.2
- ⏳ 1 бинарник (вместо 10) — ПЛАНИРУЕТСЯ Milestone 2.2
- ⏳ 80% test coverage — В ПРОЦЕССЕ (некоторые тесты сломаны)
- ⏳ Union/Intersection/Generic Types с type inference — ПЛАНИРУЕТСЯ Milestone 2.3
- ❌ Null safety анализ через CFG — БЛОКИРОВАН (требует flow-sensitive)
- ⏳ Кеширование < 50ms — ПЛАНИРУЕТСЯ Milestone 2.4

**Пользовательские метрики:**
- ⏳ Автодополнение работает мгновенно — ЧАСТИЧНО (только regex fallback)
- ❌ Hover показывает точные типы — НЕТ (показывает "Dynamic type")
- ❌ Diagnostics предотвращают 80% типовых ошибок — БЛОКИРОВАН (требует AST)
- ❌ Semantic highlighting помогает читать код — НЕ РЕАЛИЗОВАНО

---

## 🚀 Версия 3.0 — "Advanced Features" (Q2 2025: 3 месяца)

**Цель:** Превратить инструмент в полноценную IDE для 1С разработки

### 📦 Milestone 3.1: Code Intelligence (4 недели)

#### Задачи:

1. **Goto Definition** (1 неделя)
   - ✅ Переход к определению функций/процедур
   - ✅ Переход к определению переменных
   - ✅ Переход к определению типов конфигурации
   - 🎯 **Цель:** Мгновенная навигация

2. **Find References** (1 неделя)
   - ✅ Поиск всех использований символа
   - ✅ Показ в Results panel
   - ✅ Group by file
   - 🎯 **Цель:** Рефакторинг без страха

3. **Rename Symbol** (1 неделя)
   - ✅ Безопасное переименование
   - ✅ Preview изменений
   - ✅ Undo support
   - 🎯 **Цель:** Рефакторинг одним кликом

4. **Signature Help** (1 неделя)
   - ✅ Подсказки параметров функций
   - ✅ Документация параметров
   - ✅ Навигация по параметрам
   - 🎯 **Цель:** Помощь при вызове функций

**Результат Milestone 3.1:**
- ✅ Полная навигация по коду
- ✅ Безопасный рефакторинг
- ✅ Интеллектуальные подсказки

---

### 🔧 Milestone 3.2: Code Actions (3 недели)

#### Задачи:

1. **Quick Fixes** (1 неделя)
   - ✅ Автоисправление типовых ошибок
   - ✅ Добавление недостающих импортов
   - ✅ Конвертация типов
   - 🎯 **Цель:** 1 клик для исправления

2. **Refactorings** (1 неделя)
   - ✅ Extract Method
   - ✅ Extract Variable
   - ✅ Inline Variable
   - 🎯 **Цель:** Улучшение структуры кода

3. **Generate Code** (1 неделя)
   - ✅ Generate Constructor
   - ✅ Generate Getters/Setters
   - ✅ Generate Tests
   - 🎯 **Цель:** Автоматизация рутины

**Результат Milestone 3.2:**
- ✅ 20+ Code Actions
- ✅ Рефакторинг одним кликом
- ✅ Генерация шаблонного кода

---

### 📊 Milestone 3.3: Static Analysis (3 недели)

#### Задачи:

1. **Code Quality Rules** (1 неделя)
   - ✅ Проверка сложности функций (Cyclomatic Complexity)
   - ✅ Проверка длины функций
   - ✅ Проверка дублирования кода
   - 🎯 **Цель:** Метрики качества кода

2. **Security Rules** (1 неделя)
   - ✅ Проверка SQL injection
   - ✅ Проверка XSS уязвимостей
   - ✅ Проверка небезопасного eval
   - 🎯 **Цель:** Безопасный код

3. **Performance Rules** (1 неделя)
   - ✅ Проверка неоптимальных запросов
   - ✅ Проверка циклов внутри циклов
   - ✅ Проверка лишних преобразований
   - 🎯 **Цель:** Оптимальный код

**Результат Milestone 3.3:**
- ✅ 50+ правил статического анализа
- ✅ Code Quality Dashboard
- ✅ Security & Performance отчёты

---

### 🎯 Результаты Версии 3.0 (через 6 месяцев от старта)

**Технические метрики:**
- ✅ Goto Definition, Find References, Rename
- ✅ 20+ Code Actions (Quick Fixes, Refactorings)
- ✅ 50+ Static Analysis Rules
- ✅ Code Quality Dashboard

**Пользовательские метрики:**
- ✅ Навигация как в IntelliJ IDEA
- ✅ Рефакторинг одним кликом
- ✅ Автоматическое улучшение качества кода
- ✅ Предотвращение security & performance проблем

---

## 🌐 Версия 4.0 — "Collaboration & Ecosystem" (Q3-Q4 2025: 6 месяцев)

**Цель:** Создать экосистему для совместной разработки на 1С

### Milestone 4.1: Web Platform (8 недель)

1. **Type Explorer Web App** (4 недели)
   - 📊 Визуализация иерархии типов
   - 🔍 Поиск по методам/свойствам
   - 📈 Граф зависимостей типов
   - 🎯 **Цель:** Интерактивная документация

2. **Code Quality Dashboard** (4 недели)
   - 📊 Метрики по проектам
   - 📈 Тренды качества кода
   - 🚨 Критичные проблемы
   - 🎯 **Цель:** Мониторинг качества

### Milestone 4.2: Team Features (8 недель)

1. **Git Integration** (4 недели)
   - 📝 Code Review с типизацией
   - 🔍 Diff с пониманием типов
   - ✅ PR validation
   - 🎯 **Цель:** Качественный Code Review

2. **Shared Type Definitions** (4 недели)
   - 📚 Библиотека общих типов
   - 🔄 Синхронизация между проектами
   - 📦 Package manager для типов
   - 🎯 **Цель:** Переиспользование типов

### Milestone 4.3: AI Assistant (8 недель)

1. **Type Inference ML Model** (4 недели)
   - 🧠 Обучение на реальном коде
   - 🎯 Предсказание типов с вероятностью
   - 🚀 Улучшение точности до 95%
   - 🎯 **Цель:** AI-powered типизация

2. **Code Generation** (4 недели)
   - 🤖 Генерация кода по комментариям
   - 📝 Автодополнение целых функций
   - 🔧 Рефакторинг на основе AI
   - 🎯 **Цель:** AI помощник

---

## 📅 Timeline Summary

| Версия | Период | Длительность | Ключевые фичи |
|--------|--------|--------------|---------------|
| **1.0** (текущая) | Завершена | - | MVP: LSP, Валидация, VSCode Extension |
| **2.0** | Q1 2025 | 3 месяца | Tree-sitter, Flow-sensitive, Union/Generic Types |
| **3.0** | Q2 2025 | 3 месяца | Code Intelligence, Refactorings, Static Analysis |
| **4.0** | Q3-Q4 2025 | 6 месяцев | Web Platform, Team Features, AI Assistant |

---

## 🎯 Success Metrics по версиям

### Версия 2.0 — Production Ready
- ✅ 1000+ активных пользователей
- ✅ 50+ GitHub stars
- ✅ 80% positive reviews
- ✅ < 5 critical bugs в месяц

### Версия 3.0 — Advanced Features
- ✅ 5000+ активных пользователей
- ✅ 200+ GitHub stars
- ✅ 90% positive reviews
- ✅ Топ-3 в VS Code Marketplace для 1С

### Версия 4.0 — Collaboration & Ecosystem
- ✅ 20000+ активных пользователей
- ✅ 1000+ GitHub stars
- ✅ 95% positive reviews
- ✅ #1 инструмент для разработки 1С

---

## 💡 Заключение

BSL Gradual Types следует философии **Right-Sized Architecture**:

1. **Начали просто** (v1.0) — MVP работает, пользователи есть
2. **Масштабируем по необходимости** (v2.0) — добавляем критичные фичи
3. **Расширяем экосистему** (v3.0-4.0) — создаём полноценную платформу

**Ключевой принцип:** Каждая версия должна приносить **реальную ценность пользователям**, а не просто добавлять фичи.

**Следующий шаг:** Начать работу над Milestone 2.1 — оптимизация VSCode Extension 🚀
