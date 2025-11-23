# 🗺️ BSL Gradual Types — Roadmap 2025

**Проект:** BSL Gradual Type System для 1С:Предприятие
**Философия:** Right-Sized Architecture — начинаем просто, масштабируем по необходимости
**Версия:** 1.0 → 2.0 → 3.0
**Дата:** 2025-10-05

---

## 📋 Содержание

1. [Текущее состояние проекта](#-текущее-состояние-проекта-версия-10)
2. [✅ Завершённые Milestones](#-завершённые-milestones-компактный-формат) — **Детали:** [ROADMAP_ARCHIVE_2025.md](ROADMAP_ARCHIVE_2025.md)
3. [🎯 Планируемые Milestones (Версия 2.0)](#-планируемые-milestones-версия-20)
4. [🚀 Версия 3.0 — Advanced Features](#-версия-30--advanced-features-q2-2025-3-месяца)
5. [🔬 Версия 3.5 — LLVM-inspired Static Analysis](#-версия-35--llvm-inspired-static-analysis-q2-2025-4-6-недель)
6. [🌐 Версия 4.0 — Collaboration & Ecosystem](#-версия-40--collaboration--ecosystem-q3-q4-2025-6-месяцев)
7. [📅 Timeline Summary](#-timeline-summary)

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
- ✅ **Flow-sensitive analysis** — реализован в Milestone 3.5 (2025-11-08)
- ⚠️ **Union types** — базовая поддержка без нормализации

---

## 🎯 Версия 2.0 — "Production Ready" (Q1 2025: 8-10 недель)

**Цель:** Превратить MVP в production-ready инструмент для ежедневной работы разработчиков 1С

**Ключевое изменение:** Tree-sitter интеграция — **ОСНОВА** всего остального анализа

---

## ✅ Завершённые Milestones (Компактный формат)

**Детальные описания доступны в архиве:** [ROADMAP_ARCHIVE_2025.md](ROADMAP_ARCHIVE_2025.md)

| Milestone | Статус | Дата завершения | Ключевой результат | Подробности |
|-----------|--------|-----------------|-------------------|-------------|
| 2.1 Tree-sitter Integration | ✅ | 2025-09-XX | Подключена tree-sitter-bsl v0.1.5, TreeSitterAdapter, инкрементальный парсинг < 10ms | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-21-tree-sitter-integration-) |
| 2.2 VSCode Extension Optimization | ✅ | 2025-10-13 | VSIX 3.2 MB (было 30 MB), 1 бинарник, все команды через LSP, 6 test suites | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-22-vscode-extension-optimization--2025-10-13) |
| 2.3 Advanced Type System | ✅ | 2025-10-13 | Union/Intersection/Generic/Nullable Types реализованы, 50 unit-тестов проходят | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-23-advanced-type-system--2025-10-13) |
| 2.5 Унификация визуализации типов | ✅ | 2025-10-XX | Крейт `type-visualization`, HtmlRenderer/JsonRenderer, LSP custom request | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-25-унификация-визуализации-типов-) |
| 2.7 TreeSitterAdapter | ✅ | 2025-10-XX | Полная конвертация tree-sitter AST → BSL IR, обработка ошибок с fallback | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-27-treesitteradapter-) |
| 2.8 Semantic IR Layer | ✅ | 2025-10-XX | `SemanticProgram`, `SymbolTable`, `Parser trait` для DI, `AstToIrConverter` | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-28-semantic-ir-layer-) |
| 2.9 Inline Scope Analysis | ✅ | 2025-10-08 | Анализ типов локальных переменных "на лету", `find_variable_at_position()` | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-29-inline-scope-analysis--2025-10-08) |
| 2.10 LSP Configuration + Type Index | ✅ | 2025-10-08 | LSP initialization options, custom requests `bsl/renderTypeHtml`, `bsl/extractPlatformDocs` | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-210-lsp-configuration--type-index--2025-10-08) |
| 2.11 Tree-Sitter Span Extraction | ✅ | 2025-10-13 | Реальные координаты из tree-sitter, `find_node_at_position()` работает, 10 тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-211-tree-sitter-span-extraction--2025-10-13) |
| 2.13 IR Caching & Performance | ✅ | 2025-11-01 | 37× ускорение hover (<5ms). IR Cache с LRU, xxHash64, LSP invalidation | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-213-ir-caching--performance-optimization--2025-11-01) |
| 2.14 Hash Unification | ✅ | 2025-11-01 | Централизация hash_content в shared/utils/hash.rs, устранение 4 дублирований | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-214-hash-unification--централизация-hash_content--2025-11-01) |
| 2.16 Semantic Tree Visualization | ✅ | 2025-10-17 | VSCode webview, LSP custom request `bsl.getSemanticHtml`, HTML/CSS expand/collapse | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-216-semantic-tree-visualization--2025-10-17) |
| 2.18 LSP Syntax Error Diagnostics | ✅ | 2025-10-18 | Синтаксические ошибки в LSP Diagnostics, UTF-16 координаты, ~300× ускорение парсинга, 40 тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-218-lsp-syntax-error-diagnostics--2025-10-18) |
| 2.17 Configuration Metadata Parser | ✅ | 2025-11-07 | Парсинг Configuration.xml, загрузка типов конфигурации, LSP команда `bsl.parseConfiguration`, батчевая загрузка | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-217-configuration-metadata-parser--2025-11-07) |
| 2.19 Architectural Improvements | ✅ | 2025-11-07 | Unified ParseError (SSOT), TypeSystemService::parse_and_validate() API, Clean Architecture восстановлена, ~97 строк удалено | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-219-architectural-improvements--2025-11-07) |
| 2.20 Enhanced Status Bar | ✅ | 2025-11-07 | Расширенная строка статуса с прогрессом LSP/индексации, контекстом редактора, статистикой TypeRepository | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-220-enhanced-status-bar--2025-11-07) |
| 2.21 WASM Webviews Migration | ✅ | 2025-11-08 | Полная миграция VSCode Extension webviews на Leptos/WASM, устранение дублирования кода (100% DRY), Security +50%, 10 unit тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-221-wasm-webviews-migration--2025-11-08) |
| 3.5 Flow-Sensitive Analysis | ✅ | 2025-11-08 | Исправлен критический баг hover на вызовах методов, реализован flow-sensitive анализ для отслеживания типов через цепочки вызовов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-35-flow-sensitive-analysis-) |
| 3.6 Enhanced UX | ✅ | 2025-11-22 | 3-фазная реализация: DetailLevel настройки, фасетные типы, улучшенные diagnostics. 79 тестов Milestone + 332 regression, Markdown hover | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-36-enhanced-ux-hover--diagnostics--завершён-2025-11-22) |
| 3.7 Semantic Diagnostics MVP | ✅ | 2025-11-XX | Интеграция семантической валидации в LSP: неизвестные типы, несуществующие методы, type mismatch. 40+ интеграционных тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-37-semantic-diagnostics-mvp-) |
| 3.8 Advanced Type Narrowing | ✅ | 2025-11-10 | Control-flow анализ для сужения типов: if/elif проверки, TypeNarrowing trait, поддержка ТипЗнч() и логических операторов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-38-advanced-type-narrowing-) |
| 3.9 Return Type Inference | ✅ | 2025-11-13 | Автоматический вывод типов возврата для 150+ методов платформы, устранение Неопределено для цепочек вызовов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-39-return-type-inference-для-методов) |
| 3.10 Валидация параметров | ✅ | 2025-11-13 | Проверка количества и типов параметров при вызовах методов, поддержка опциональных параметров, 20+ тестов валидации | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-310-валидация-параметров-методов) |

**Итого завершено:** 24 Milestones
**Прогресс Версии 2.0:** ~95% завершено (19/20 Milestones)
**Прогресс Версии 3.0:** ~100% завершено (6/6 Milestones)

---

## 🎯 Планируемые Milestones (Версия 2.0)

### 📈 Milestone 2.4: Persistent Cache & Parallel Analysis (1.5-2 недели)

**Приоритет:** 🟡 СРЕДНИЙ — улучшение производительности при работе с большими проектами

**Контекст:**
Milestone 2.4 частично реализован в рамках Milestone 2.13 (IR Caching) и 2.14 (Hash Unification):
- ✅ In-memory IR Cache с LRU (37× ускорение hover)
- ✅ xxHash64 для быстрого хеширования
- ✅ LSP invalidation при изменении файлов

**Что осталось сделать:**

#### Задачи:

**Task 1: Persistent Cache на диске** (1 неделя)
- ✅ Кеширование AST деревьев в `.bsl_cache/ast/`
- ✅ Кеширование IR (SemanticProgram) в `.bsl_cache/ir/`
- ✅ Инвалидация при изменении файлов (по hash)
- ✅ TTL для автоматической очистки старых файлов (7 дней)
- ✅ Компрессия кеша (gzip или zstd) для экономии места
- 🎯 **Цель:** Загрузка проекта из кеша < 100ms (vs 5-10s холодный старт)

**Архитектура:**
```rust
// backend/src/system/persistent_cache.rs
pub struct PersistentCache {
    cache_dir: PathBuf,  // .bsl_cache/
    ttl_days: u32,       // 7 дней по умолчанию
}

impl PersistentCache {
    pub fn get_ir(&self, file_hash: u64) -> Result<Option<Arc<SemanticProgram>>> {
        let cache_file = self.cache_dir.join("ir").join(format!("{}.bin.gz", file_hash));
        // Читаем, декомпрессируем, deserialize
    }

    pub fn put_ir(&self, file_hash: u64, ir: &SemanticProgram) -> Result<()> {
        let cache_file = self.cache_dir.join("ir").join(format!("{}.bin.gz", file_hash));
        // Serialize, compress, записываем
    }

    pub fn cleanup_old_entries(&self) -> Result<usize> {
        // Удаляем файлы старше ttl_days
    }
}
```

**Интеграция с Milestone 2.13 (IR Cache):**
```rust
// TypeSystemService::get_hover_info()
// 1. Проверяем in-memory cache (Milestone 2.13)
// 2. MISS → проверяем persistent cache (Milestone 2.4)
// 3. MISS → парсим и кешируем в оба слоя
```

**Task 2: Параллельный анализ больших проектов** (1 неделя)
- ✅ Multi-threaded анализ файлов через `rayon`
- ✅ Batch processing: анализ 1000+ файлов
- ✅ Progress bar для CLI/LSP (через `indicatif`)
- ✅ Graceful degradation при ошибках (продолжаем анализ остальных файлов)
- 🎯 **Цель:** Анализ 1000 файлов < 30 секунд (vs 5+ минут последовательно)

**Архитектура:**
```rust
// backend/src/application/batch_analyzer.rs
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};

pub struct BatchAnalyzer {
    type_service: Arc<TypeSystemService>,
    thread_pool_size: usize,  // По умолчанию num_cpus
}

impl BatchAnalyzer {
    pub fn analyze_workspace(&self, workspace_path: &Path) -> Result<AnalysisReport> {
        let bsl_files = self.discover_bsl_files(workspace_path)?;

        let pb = ProgressBar::new(bsl_files.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("[{elapsed}] {bar:40} {pos}/{len} {msg}")?);

        let results: Vec<_> = bsl_files.par_iter()
            .map(|file| {
                let result = self.type_service.analyze_file(file);
                pb.inc(1);
                result
            })
            .collect();

        pb.finish_with_message("Анализ завершен");
        Ok(AnalysisReport::from_results(results))
    }
}
```

**LSP Integration:**
```rust
// При открытии workspace:
// 1. Загружаем metadata из persistent cache
// 2. Запускаем background task для переиндексации измененных файлов
// 3. Показываем прогресс в status bar (Milestone 2.20)
```

**Результат Milestone 2.4:**
- ✅ Persistent Cache между сеансами LSP
- ✅ Холодный старт проекта < 100ms (vs 5-10s без кеша)
- ✅ Параллельный анализ 1000+ файлов < 30s
- ✅ Progress bar для больших операций
- ✅ TTL для автоматической очистки старых кешей
- ✅ Производительность сравнима с rust-analyzer и gopls

**Зависимости:**
- ✅ Milestone 2.13 (IR Caching) — переиспользуем in-memory cache
- ✅ Milestone 2.14 (Hash Unification) — используем hash_content для ключей кеша

**Оценка времени:** 1.5-2 недели

**Тестирование:**
- Unit-тесты: persistent cache read/write/invalidation
- Integration-тесты: batch analysis с rayon
- E2E-тесты: холодный старт LSP с persistent cache
- Performance-тесты: 1000 файлов, замер времени

---

## 🚀 Версия 3.0 — "Advanced Features" (Q2 2025: 3 месяца)

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

### 🤖 Milestone 3.4: MCP (Model Context Protocol) Server Integration (8-10 дней)

**Приоритет:** 🟡 СРЕДНИЙ — расширение возможностей взаимодействия с LLM через стандартизированный протокол

**Проблема:**
Сейчас система типов BSL доступна только через LSP и Web API. Для эффективной работы с LLM (Claude, ChatGPT и другие) нужен стандартизированный протокол доступа к контексту кода. MCP (Model Context Protocol) от Anthropic решает эту проблему, предоставляя:
- Универсальный протокол для подключения AI к источникам данных
- Структурированные Resources (данные), Tools (действия) и Prompts (шаблоны)
- Подписки на изменения и real-time обновления

**Цель:**
Создать MCP сервер для BSL Gradual Types, который предоставит LLM полный контекст о типах, коде и структуре проекта через стандартизированный протокол.

**Справка:**
- 🔗 Model Context Protocol: https://modelcontextprotocol.io/
- 🔗 Rust MCP SDK (официальный Anthropic): https://github.com/modelcontextprotocol/rust-sdk
- 🔗 Примеры MCP серверов (community): https://github.com/rust-mcp-stack/rust-mcp-filesystem

**Архитектура:**
```
┌──────────────┐     ┌────────────────┐     ┌─────────────────┐
│  Claude/AI   │────▶│   MCP Server   │────▶│  BSL TypeSystem │
│              │◀────│  (новый crate) │◀────│   (существующий)│
└──────────────┘     └────────────────┘     └─────────────────┘
                              │
                              ▼
                     ┌────────────────┐
                     │ File Watcher   │
                     │   (notify)     │
                     └────────────────┘
```

**Принципы:**
- **Максимальное переиспользование** — использовать существующие `TreeSitterAdapter`, `SemanticProgram`, `TypeRepository`
- **Right-Sized Architecture** — не усложняем, добавляем только MCP-специфичную логику
- **Кросс-платформенность** — notify crate уже есть в workspace dependencies (версия 6.1)

#### Задачи:

**Task 1: Создание структуры MCP Server crate (1 день)**

Создать новый crate `mcp-server/`:

```toml
# mcp-server/Cargo.toml
[package]
name = "bsl-mcp-server"
version.workspace = true

[[bin]]
name = "bsl-mcp"
path = "src/main.rs"

[dependencies]
rmcp = { version = "0.8.5", features = ["server", "macros"] }  # Официальный Anthropic SDK
bsl-backend = { path = "../backend" }
bsl-shared = { workspace = true }
notify = { workspace = true }
tokio = { workspace = true, features = ["full"] }
serde_json = { workspace = true }
```

Структура модулей:
```rust
pub mod server;      // MCP server implementation
pub mod resources;   // Resources handlers (файлы, типы, AST)
pub mod tools;       // Tools handlers (анализ, валидация)
pub mod prompts;     // Prompts для генерации BSL кода
pub mod watcher;     // File watching с notify
pub mod cache;       // Переиспользуем IrCache из backend
```

---

**Task 2: File Watcher с notify (1 день)**

Реализовать кросс-платформенный мониторинг BSL файлов:
- Windows: ReadDirectoryChangesW
- Linux: inotify
- macOS: FSEvents

```rust
// mcp-server/src/watcher.rs
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

pub struct BslFileWatcher {
    watcher: RecommendedWatcher,
    glob_set: GlobSet,  // Фильтр для *.bsl, *.os
}

impl BslFileWatcher {
    pub fn new(workspace_paths: Vec<PathBuf>) -> Result<Self> {
        let mut watcher = notify::recommended_watcher(|event| {
            match event {
                Ok(Event { kind: Modify(_), paths, .. }) => {
                    // Инвалидируем IR Cache
                    // Уведомляем подписчиков MCP
                }
                _ => {}
            }
        })?;

        for path in &workspace_paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
        }

        Ok(Self { watcher, glob_set })
    }
}
```

---

**Task 3: MCP Resources для BSL контекста (2 дня)**

Реализовать Resources handlers:

**Resources:**
- `bsl://files` — список всех BSL файлов в проекте
- `bsl://file/{path}` — содержимое файла с типами и метаданными
- `bsl://types` — индекс всех типов (платформа + конфигурация)
- `bsl://type/{typename}` — детали типа (facets, методы, свойства)
- `bsl://ast/{path}` — AST дерево файла

```rust
// mcp-server/src/resources.rs
impl BslResourceHandler {
    async fn get_file_content(&self, path: &str) -> ResourceContent {
        let content = tokio::fs::read_to_string(path).await?;
        let hash = hash_content(&content);

        // Проверяем IR Cache (Milestone 2.13)
        let ir = if let Some(cached) = self.ir_cache.get(hash).await {
            cached
        } else {
            let tree = self.tree_adapter.parse(&content)?;
            let ir = self.tree_adapter.convert_to_semantic(&tree)?;
            self.ir_cache.put(hash, Arc::new(ir)).await;
            ir
        };

        ResourceContent {
            text: Some(content),
            metadata: Some(json!({
                "types": extract_types(&ir),
                "functions": extract_functions(&ir),
            })),
        }
    }
}
```

---

**Task 4: MCP Tools для анализа и валидации (2 дня)**

Реализовать Tools для выполнения действий:

**Tools:**
- `validate_type` — валидация типа выражения на позиции
- `find_type_usages` — поиск всех использований типа
- `rename_type` — рефакторинг переименования типа
- `generate_docs` — генерация документации для модуля
- `analyze_complexity` — анализ сложности функций

```rust
// mcp-server/src/tools.rs
impl BslToolHandler {
    #[tool]
    pub async fn validate_type(
        &self,
        file_path: String,
        line: u32,
        column: u32,
    ) -> Result<TypeValidationResult> {
        let content = tokio::fs::read_to_string(&file_path).await?;
        let ir = self.tree_adapter.parse_to_semantic(&content)?;
        let expr = ir.find_expression_at(line, column)?;
        let inferred_type = self.type_service.infer_type(&expr)?;

        Ok(TypeValidationResult {
            expression: expr.to_string(),
            inferred_type: inferred_type.to_string(),
            errors: self.type_service.validate(&inferred_type)?,
        })
    }

    #[tool]
    pub async fn find_type_usages(
        &self,
        type_name: String,
        workspace_path: String,
    ) -> Result<Vec<TypeUsage>> {
        // Поиск по всем BSL файлам
        let mut usages = Vec::new();
        for entry in walkdir::WalkDir::new(&workspace_path) {
            // Парсинг и поиск использований типа
        }
        Ok(usages)
    }
}
```

---

**Task 5: MCP Prompts для генерации кода (1 день)**

Реализовать готовые Prompts для генерации BSL кода:

**Prompts:**
- `generate_function` — генерация функции с типами
- `generate_module` — генерация модуля конфигурации
- `generate_tests` — генерация unit-тестов для функции
- `refactor_to_typed` — рефакторинг кода с добавлением типов

```rust
// mcp-server/src/prompts.rs
pub struct BslPromptHandler {
    type_service: Arc<TypeSystemService>,
}

impl BslPromptHandler {
    pub async fn list_prompts(&self) -> Vec<Prompt> {
        vec![
            Prompt {
                name: "generate_function".to_string(),
                description: "Генерирует BSL функцию с типами параметров и возврата".to_string(),
                arguments: vec![
                    PromptArgument {
                        name: "function_name".to_string(),
                        description: "Имя функции на русском".to_string(),
                        required: true,
                    },
                    PromptArgument {
                        name: "params".to_string(),
                        description: "Параметры функции (JSON массив)".to_string(),
                        required: false,
                    },
                ],
            },
            Prompt {
                name: "refactor_to_typed".to_string(),
                description: "Добавляет типизацию в существующий BSL код".to_string(),
                arguments: vec![
                    PromptArgument {
                        name: "code".to_string(),
                        description: "BSL код для рефакторинга".to_string(),
                        required: true,
                    },
                ],
            },
        ]
    }

    pub async fn get_prompt(&self, name: &str, args: &Value) -> Result<String> {
        match name {
            "generate_function" => {
                let func_name = args["function_name"].as_str().unwrap();
                Ok(format!(
                    "Сгенерируй BSL функцию '{}' с типизацией параметров и возврата. \
                     Используй доступные типы платформы 1С: {}",
                    func_name,
                    self.get_available_types().await?
                ))
            }
            "refactor_to_typed" => {
                let code = args["code"].as_str().unwrap();
                let ir = self.parse_and_infer_types(code).await?;
                Ok(format!(
                    "Добавь типизацию в следующий BSL код:\n\n{}\n\n\
                     Инферированные типы:\n{}",
                    code,
                    serde_json::to_string_pretty(&ir.inferred_types)?
                ))
            }
            _ => Err(anyhow::anyhow!("Unknown prompt: {}", name)),
        }
    }
}
```

---

**Task 6: Subscriptions и notifications (1 день)**

Реализовать подписки на изменения ресурсов:

```rust
// mcp-server/src/server.rs
pub struct BslMcpServer {
    subscriptions: Arc<RwLock<HashMap<String, Vec<SubscriptionId>>>>,
}

impl BslMcpServer {
    async fn handle_file_change(&self, event: FileChangeEvent) {
        match event {
            FileChangeEvent::Modified(path) => {
                // 1. Переиндексировать файл
                self.indexer.reindex_file(&path).await;

                // 2. Уведомить подписчиков через MCP
                let uri = format!("bsl://file/{}", path.display());
                self.notify_subscribers(&uri).await;
            }
            _ => {}
        }
    }

    async fn notify_subscribers(&self, resource_uri: &str) {
        let subs = self.subscriptions.read().await;
        if let Some(subscribers) = subs.get(resource_uri) {
            for sub_id in subscribers {
                // MCP notifications/resources/updated
                self.send_notification("resources/updated", json!({
                    "uri": resource_uri,
                })).await;
            }
        }
    }
}
```

---

**Task 7: Главный сервер и интеграция (2 дня)**

Собрать всё вместе и протестировать:

```rust
// mcp-server/src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let workspace_paths = vec![PathBuf::from(&args[1])];

    // Создаём MCP сервер
    let server = BslMcpServer::new(workspace_paths).await?;

    // Запускаем через stdio transport
    let transport = StdioTransport::new(tokio::io::stdin(), tokio::io::stdout());
    server.serve(transport).await?;

    Ok(())
}
```

**Конфигурация для Claude Desktop:**
```json
// claude_desktop_config.json
{
  "mcpServers": {
    "bsl-types": {
      "command": "bsl-mcp",
      "args": ["C:/path/to/1c/project"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**Интеграционные тесты:**
```rust
#[tokio::test]
async fn test_file_watcher_detects_changes() {
    let temp_dir = tempdir::TempDir::new("bsl_test").unwrap();
    let watcher = BslFileWatcher::new(vec![temp_dir.path().to_path_buf()]).await.unwrap();

    let test_file = temp_dir.path().join("test.bsl");
    tokio::fs::write(&test_file, "Функция Тест()\nКонецФункции").await.unwrap();

    // Проверяем событие создания
    let event = rx.recv().await.unwrap();
    assert!(matches!(event, FileChangeEvent::Created(_)));
}

#[tokio::test]
async fn test_mcp_resources() {
    let server = BslMcpServer::new(vec![PathBuf::from("./examples")]).await.unwrap();
    let resources = server.handle_list_resources().await.unwrap();

    assert!(!resources.is_empty());
    assert!(resources.iter().any(|r| r.uri.contains("bsl://file/")));
}

#[tokio::test]
async fn test_mcp_prompts() {
    let server = BslMcpServer::new(vec![PathBuf::from("./examples")]).await.unwrap();
    let prompts = server.handle_list_prompts().await.unwrap();

    assert!(prompts.iter().any(|p| p.name == "generate_function"));
    assert!(prompts.iter().any(|p| p.name == "refactor_to_typed"));
}
```

---

**Результат Milestone 3.4:**
- ✅ MCP сервер запускается и принимает соединения
- ✅ File watcher отслеживает изменения BSL файлов (Windows/Linux/macOS)
- ✅ Resources: файлы, типы, AST
- ✅ Tools: валидация, поиск, рефакторинг, анализ
- ✅ Prompts: генерация функций, модулей, тестов, рефакторинг с типами
- ✅ Subscriptions: real-time уведомления об изменениях
- ✅ IR Cache переиспользуется из Milestone 2.13
- ✅ Claude Desktop интегрирован с BSL проектом
- ✅ Производительность: <10ms для cached resources, <100ms для парсинга

**Зависимости:**
- ✅ Milestone 2.8 (Semantic IR Layer)
- ✅ Milestone 2.13 (IR Cache)
- ✅ Milestone 2.7 (TreeSitterAdapter)

**Оценка времени:** 8-10 дней

---

### 📦 Milestone 3.12: Enhanced Configuration Parser — Forms, Modules & Contexts (2-3 недели)

**Приоритет:** 🔴 КРИТИЧЕСКИЙ — необходим для Milestone 3.11 (Context-Aware Facets)

**Статус:** 🚧 В ПРОЦЕССЕ — Phase 1-2 завершены ✅ (2025-11-23)

**Прогресс:**
- ✅ **Phase 1: CommonModule Properties** (3-4 дня) — ЗАВЕРШЕНА
  - ExecutionContext enum
  - CommonModuleProperties struct
  - Парсинг Server/Client/Global/ServerCall свойств
  - 12 unit тестов (100% pass)
  - Коммит: a7b7a1d
- ✅ **Phase 2: Forms Parsing** (4-5 дней) — ЗАВЕРШЕНА
  - FormMetadata, FormAttribute structures
  - FormParser для парсинга Form.xml
  - Извлечение реквизитов с типами (включая композитные)
  - Парсинг событий и ExecutionContext из Module.bsl
  - 9 integration тестов (100% pass)
  - Коммит: 10b6397
- 📝 **Phase 3: Object/Manager Modules** (2-3 дня) — ПЛАНИРУЕТСЯ
- 📝 **Phase 4: Context Resolution** (3-4 дня) — ПЛАНИРУЕТСЯ

**Проблема:**

Текущий Configuration Parser (Milestone 2.17) имеет ограниченную функциональность:

1. **Не парсит контекстные свойства общих модулей:**
   ```xml
   <CommonModule>
     <Server>true</Server>      <!-- ❌ НЕ парсится -->
     <Client>false</Client>     <!-- ❌ НЕ парсится -->
     <ServerCall>true</ServerCall>
   </CommonModule>
   ```

2. **Не парсит формы и их модули:**
   - Формы элементов (ItemForm)
   - Формы списков (ListForm)
   - Реквизиты форм
   - Модули форм с директивами &НаСервере/&НаКлиенте

3. **Не определяет контекст выполнения по месту кода:**
   ```bsl
   // Модуль объекта Справочника - ВСЕГДА серверный
   // Модуль формы - зависит от директивы
   // Общий модуль - зависит от свойств модуля
   ```

4. **Не парсит модули объектов:**
   - Модуль менеджера (ManagerModule)
   - Модуль объекта (ObjectModule)
   - Модуль набора записей (RecordSetModule)

**Исследование:**

**Типы модулей в 1С и их контексты:**

| Тип модуля | Контекст по умолчанию | Переопределяется директивой? | Доступ к БД |
|-----------|----------------------|------------------------------|------------|
| **Общий (Server=true, Client=false)** | Серверный | ❌ Нет | ✅ Полный |
| **Общий (Server=false, Client=true)** | Клиентский | ❌ Нет | ❌ Нет |
| **Общий (Server=true, Client=true)** | Оба | ✅ Да (&НаСервере/&НаКлиенте) | Зависит |
| **Модуль объекта** | Серверный | ❌ Нет | ✅ Полный |
| **Модуль менеджера** | Серверный | ❌ Нет | ✅ Полный |
| **Модуль формы** | Оба | ✅ Да (&НаСервере/&НаКлиенте) | Зависит |
| **Модуль команды** | Зависит от настроек | Зависит | Зависит |

**Свойства общих модулей (из Configuration.xml):**

```xml
<CommonModule>
  <Properties>
    <Name>ИмяМодуля</Name>
    <Server>true|false</Server>                    <!-- Выполняется на сервере -->
    <Client>true|false</Client>                    <!-- Выполняется на клиенте -->
    <ServerCall>true|false</ServerCall>            <!-- Вызов с клиента -->
    <Privileged>true|false</Privileged>            <!-- Привилегированный режим -->
    <Global>true|false</Global>                    <!-- Глобальный контекст -->
    <ClientManagedApplication>true|false</ClientManagedApplication>
    <ExternalConnection>true|false</ExternalConnection>
    <ReturnValuesReuse>DontUse|DuringRequest|DuringSession</ReturnValuesReuse>
  </Properties>
</CommonModule>
```

**Структура форм в конфигурации:**

```xml
<Catalog>
  <Name>Контрагенты</Name>
  <Forms>
    <Form uuid="...">
      <Properties>
        <Name>ФормаЭлемента</Name>
        <FormType>Managed</FormType>  <!-- Управляемая форма -->
      </Properties>
      <!-- Реквизиты формы хранятся в отдельном Form.xml -->
    </Form>
  </Forms>
  <ObjectModule>...</ObjectModule>
  <ManagerModule>...</ManagerModule>
</Catalog>
```

**Файл формы (Form.xml):**

```xml
<Form>
  <Attributes>
    <Attribute>
      <Name>Объект</Name>
      <Type>
        <Type>CatalogRef.Контрагенты</Type>  <!-- Тип реквизита -->
      </Type>
    </Attribute>
    <Attribute>
      <Name>ДополнительноеПоле</Name>
      <Type>
        <Type>String</Type>
        <StringQualifiers><Length>50</Length></StringQualifiers>
      </Type>
    </Attribute>
  </Attributes>
</Form>
```

**Решение:**

Расширить Configuration Parser для извлечения:
1. Контекстных свойств общих модулей (Server/Client/ServerCall)
2. Информации о формах (структура, реквизиты, типы)
3. Модулей объектов (ObjectModule, ManagerModule)
4. ExecutionContext mapping для определения доступных методов

#### Задачи:

**Phase 1: CommonModule Properties Parsing (3-4 дня)**

**Task 1.1: Расширить UniversalMetadataObject (1 день)**

Добавить поля для контекстных свойств:

```rust
// backend/src/data/loaders/config_metadata_parser/types.rs
pub struct UniversalMetadataObject {
    // ...existing fields...

    // ✅ NEW: Контекстные свойства для общих модулей
    pub context_properties: Option<ModuleContextProperties>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleContextProperties {
    pub server: bool,
    pub client: bool,
    pub server_call: bool,
    pub privileged: bool,
    pub global: bool,
    pub client_managed_application: bool,
    pub external_connection: bool,
    pub return_values_reuse: ReturnValuesReuse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReturnValuesReuse {
    DontUse,
    DuringRequest,
    DuringSession,
}
```

**Task 1.2: Парсинг свойств из Configuration.xml (1 день)**

Обновить `backend/src/data/loaders/config_metadata_parser/xml_parser.rs`:

```rust
fn parse_common_module(element: &Element) -> Result<UniversalMetadataObject> {
    let name = extract_text(element, "Properties/Name")?;

    // ✅ NEW: Парсинг контекстных свойств
    let context_properties = ModuleContextProperties {
        server: extract_bool(element, "Properties/Server").unwrap_or(false),
        client: extract_bool(element, "Properties/Client").unwrap_or(false),
        server_call: extract_bool(element, "Properties/ServerCall").unwrap_or(false),
        privileged: extract_bool(element, "Properties/Privileged").unwrap_or(false),
        global: extract_bool(element, "Properties/Global").unwrap_or(false),
        // ...
    };

    Ok(UniversalMetadataObject {
        name,
        kind: MetadataKind::CommonModule,
        context_properties: Some(context_properties),
        // ...
    })
}
```

**Task 1.3: ExecutionContext mapping (1 день)**

Создать логику определения контекста:

```rust
// shared/src/domain/execution_context.rs
impl ModuleContextProperties {
    pub fn get_execution_context(&self) -> ExecutionContext {
        match (self.server, self.client) {
            (true, false) => ExecutionContext::ServerOnly,
            (false, true) => ExecutionContext::ClientOnly,
            (true, true) => ExecutionContext::Both,  // Клиент-Сервер
            (false, false) => ExecutionContext::Unknown,
        }
    }

    pub fn can_call_database_methods(&self, current_directive: Option<&CompilerDirective>) -> bool {
        match self.get_execution_context() {
            ExecutionContext::ServerOnly => true,
            ExecutionContext::ClientOnly => false,
            ExecutionContext::Both => {
                // Зависит от директивы
                matches!(current_directive, Some(CompilerDirective::OnServer))
            }
            ExecutionContext::Unknown => false,
        }
    }
}
```

**Task 1.4: Unit тесты (1 день)**
- Тесты парсинга всех комбинаций Server/Client
- Тесты ExecutionContext mapping
- 15-20 тестов покрытия

---

**Phase 2: Forms Parsing (4-5 дней)**

**Task 2.1: Структуры для форм (1 день)**

Создать новые типы данных:

```rust
// backend/src/data/loaders/config_metadata_parser/form_types.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormMetadata {
    pub name: String,
    pub uuid: String,
    pub form_type: FormType,
    pub attributes: Vec<FormAttribute>,
    pub module_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormType {
    Managed,     // Управляемая
    Ordinary,    // Обычная
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormAttribute {
    pub name: String,
    pub type_name: String,  // "CatalogRef.Контрагенты", "String", "Number"
    pub is_main_attribute: bool,  // Основной реквизит (обычно "Объект")
}
```

**Task 2.2: Парсинг форм из Configuration.xml (2 дня)**

Добавить обработку Forms для всех объектов метаданных:

```rust
fn parse_catalog_forms(catalog_element: &Element, catalog_name: &str) -> Result<Vec<FormMetadata>> {
    let forms = catalog_element.find_all("ChildObjects/Form");

    let mut result = Vec::new();
    for form_elem in forms {
        let form_name = extract_text(form_elem, "Properties/Name")?;
        let uuid = extract_text(form_elem, "@uuid")?;

        // Путь к Form.xml (нужно читать отдельно)
        let form_path = format!("Catalogs/{}/Forms/{}/Ext/Form.xml", catalog_name, form_name);

        let form = FormMetadata {
            name: form_name,
            uuid,
            form_type: FormType::Managed,  // Определяется из Form.xml
            attributes: vec![],  // Парсятся из Form.xml (Task 2.3)
            module_path: Some(PathBuf::from(form_path)),
        };

        result.push(form);
    }

    Ok(result)
}
```

**Task 2.3: Парсинг Form.xml (реквизиты форм) (1 день)**

Реализовать парсер отдельных файлов Form.xml:

```rust
// backend/src/data/loaders/config_metadata_parser/form_parser.rs
pub fn parse_form_xml(path: &Path) -> Result<FormDetails> {
    let xml_content = std::fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&xml_content)?;

    // Парсинг реквизитов
    let attributes = doc.root()
        .descendants()
        .filter(|n| n.tag_name().name() == "Attribute")
        .map(|attr_node| {
            FormAttribute {
                name: extract_text(&attr_node, "Name")?,
                type_name: extract_text(&attr_node, "Type/Type")?,  // "CatalogRef.X"
                is_main_attribute: extract_text(&attr_node, "Name")? == "Объект",
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(FormDetails {
        form_type: determine_form_type(&doc)?,
        attributes,
        commands: parse_form_commands(&doc)?,
    })
}
```

**Task 2.4: Integration тесты (1 день)**
- Тесты парсинга форм из реальной конфигурации
- Тесты извлечения реквизитов
- 10-15 тестов

---

**Phase 3: Object/Manager Modules Parsing (2-3 дня)**

**Task 3.1: Парсинг модулей объектов (1 день)**

Добавить извлечение путей к модулям:

```rust
pub struct ObjectModulesInfo {
    pub manager_module: Option<PathBuf>,  // ManagerModule.bsl
    pub object_module: Option<PathBuf>,   // ObjectModule.bsl
    pub record_set_module: Option<PathBuf>,  // Для регистров
}

fn parse_catalog_modules(catalog_element: &Element, catalog_name: &str) -> Result<ObjectModulesInfo> {
    Ok(ObjectModulesInfo {
        manager_module: if has_node(catalog_element, "ManagerModule") {
            Some(PathBuf::from(format!("Catalogs/{}/Ext/ManagerModule.bsl", catalog_name)))
        } else {
            None
        },
        object_module: if has_node(catalog_element, "ObjectModule") {
            Some(PathBuf::from(format!("Catalogs/{}/Ext/ObjectModule.bsl", catalog_name)))
        } else {
            None
        },
        record_set_module: None,
    })
}
```

**Task 3.2: Хранение информации о модулях (1 день)**

Расширить `RawTypeData`:

```rust
pub struct RawTypeData {
    // ...existing fields...

    // ✅ NEW: Информация о модулях
    pub modules: Option<ObjectModulesInfo>,
    pub context_properties: Option<ModuleContextProperties>,
}
```

**Task 3.3: Integration тесты (1 день)**
- Тесты определения контекста по типу модуля
- 10 тестов

---

**Phase 4: Context Resolution System (3-4 дня)**

**Task 4.1: CodeLocation определение (1 день)**

Создать систему определения места кода:

```rust
// shared/src/domain/code_location.rs
pub struct CodeLocation {
    pub file_path: PathBuf,
    pub module_type: ModuleType,
    pub execution_context: ExecutionContext,
}

pub enum ModuleType {
    CommonModule { properties: ModuleContextProperties },
    ObjectModule,
    ManagerModule,
    FormModule,
    RecordSetModule,
    CommandModule,
}

impl CodeLocation {
    pub fn determine_from_path(path: &Path, config: &Configuration) -> Result<Self> {
        // Логика определения типа модуля по пути:
        // "CommonModules/ОбщийМодуль1/Ext/Module.bsl" → CommonModule
        // "Catalogs/Контрагенты/Ext/ObjectModule.bsl" → ObjectModule
        // "Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl" → FormModule
    }

    pub fn can_call_database_methods(&self, directive: Option<&CompilerDirective>) -> bool {
        match &self.module_type {
            ModuleType::CommonModule { properties } => {
                properties.can_call_database_methods(directive)
            }
            ModuleType::ObjectModule | ModuleType::ManagerModule => true,
            ModuleType::FormModule => {
                matches!(directive, Some(CompilerDirective::OnServer))
            }
            _ => false,
        }
    }
}
```

**Task 4.2: LSP Integration (1 день)**

Передавать CodeLocation в TypeSystemService:

```rust
// backend/src/bin/lsp_server.rs
async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let file_path = uri.to_file_path().ok()?;

    // ✅ NEW: Определяем CodeLocation
    let code_location = CodeLocation::determine_from_path(&file_path, &self.configuration)?;

    // Передаём в get_hover_info
    let hover_text = self.type_service
        .get_hover_info_with_context(code, line, column, Some(&code_location))
        .await?;

    // ...
}
```

**Task 4.3: TypeResolver с контекстом (1 день)**

Обновить резолвинг с учётом CodeLocation:

```rust
impl TypeResolver {
    pub fn resolve_expression_with_context(
        &self,
        expr: &str,
        code_location: Option<&CodeLocation>,
        directive: Option<&CompilerDirective>,
    ) -> TypeResolution {
        // Проверяем доступность метода в текущем контексте
        if let Some(location) = code_location {
            if !location.can_call_database_methods(directive) {
                // Возвращаем TypeResolution с warning
                return TypeResolution {
                    certainty: Certainty::Unknown,
                    warnings: vec!["База данных недоступна в текущем контексте".to_string()],
                    // ...
                };
            }
        }

        // Обычная резолюция
        self.resolve_expression_sync(expr)
    }
}
```

**Task 4.4: Integration тесты (1 день)**
- 15-20 тестов для различных CodeLocation
- Проверка доступности методов в контекстах

#### Критерии успеха:

1. ✅ **Общие модули парсятся с контекстом:**
   ```rust
   CommonModule {
       name: "ОбщийМодуль1",
       context: ModuleContextProperties {
           server: true,
           client: false,
           server_call: true,
       }
   }
   ```

2. ✅ **Формы парсятся с реквизитами:**
   ```rust
   Form {
       name: "ФормаЭлемента",
       attributes: vec![
           FormAttribute {
               name: "Объект",
               type_name: "СправочникСсылка.Контрагенты",
           }
       ]
   }
   ```

3. ✅ **CodeLocation определяется автоматически:**
   ```rust
   // Путь: "Catalogs/Контрагенты/Ext/ObjectModule.bsl"
   CodeLocation {
       module_type: ModuleType::ObjectModule,
       execution_context: ExecutionContext::ServerOnly,
   }
   ```

4. ✅ **Context-aware резолвинг работает:**
   ```bsl
   // В клиентском общем модуле
   Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
   // Warning: Метод недоступен в клиентском контексте
   ```

5. ✅ **50+ тестов проходят:**
   - 20 тестов парсинга CommonModule properties
   - 15 тестов парсинга Form metadata
   - 15 тестов CodeLocation resolution
   - 10 integration тестов

#### Архитектура:

**Новые модули:**

1. **`backend/src/data/loaders/config_metadata_parser/form_parser.rs`** (NEW)
   - Парсинг Form.xml
   - Извлечение реквизитов
   - Определение типов реквизитов

2. **`backend/src/data/loaders/config_metadata_parser/module_context.rs`** (NEW)
   - ModuleContextProperties
   - ExecutionContext mapping
   - Логика определения доступности

3. **`shared/src/domain/code_location.rs`** (NEW)
   - CodeLocation struct
   - ModuleType enum
   - Определение контекста по пути файла

**Изменяемые компоненты:**

1. **`backend/src/data/loaders/config_metadata_parser/types.rs`**
   - Расширить UniversalMetadataObject
   - Добавить FormMetadata

2. **`backend/src/data/loaders/config_metadata_parser/xml_parser.rs`**
   - Парсинг новых свойств
   - Обработка Forms

3. **`shared/src/domain/types.rs`**
   - Расширить RawTypeData полем modules

4. **`backend/src/bin/lsp_server.rs`**
   - Передача CodeLocation в hover/diagnostics

#### Зависимости:

**Этот Milestone требует:**
- ✅ Milestone 2.17 (Configuration Metadata Parser) — базовая инфраструктура

**Этот Milestone нужен для:**
- ⚠️ Milestone 3.11 (Context-Aware Facets) — **КРИТИЧЕСКАЯ ЗАВИСИМОСТЬ**
- ⚠️ Milestone 3.13 (Advanced Autocomplete) — контекстная фильтрация

**Последовательность реализации:**
```
3.12 Enhanced Config Parser (формы + контексты)
  ↓
3.11 Context-Aware Facets (использует CodeLocation)
  ↓
3.13+ Остальные Milestones
```

#### Риски и митигация:

| Риск | Вероятность | Влияние | Митигация |
|------|-------------|---------|-----------|
| Сложность XML структуры форм | Высокая | Среднее | Инкрементальный парсинг (сначала базовые реквизиты) |
| Различия между версиями платформы | Средняя | Высокое | Поддержка нескольких форматов XML |
| Производительность парсинга | Низкая | Среднее | Кеширование результатов |
| Неполнота метаданных | Высокая | Низкое | Graceful degradation (пропуск несущественных свойств) |

#### Оценка времени:

- **Phase 1:** 3-4 дня (CommonModule properties)
- **Phase 2:** 4-5 дней (Forms parsing)
- **Phase 3:** 2-3 дня (Object/Manager modules)
- **Phase 4:** 3-4 дня (Context resolution)
- **Итого:** 12-16 дней (2-3 недели)

**Буфер:** +4-6 дней для тестирования и отладки
**Всего:** 3-4 недели

---

### 🎭 Milestone 3.11: Context-Aware Facet Selection (2-3 недели)

**Приоритет:** 🔴 КРИТИЧЕСКИЙ — без этого фасетная система типов не работает правильно

**Статус:** 📝 ПЛАНИРУЕТСЯ

**Проблема:**

Текущая реализация не поддерживает context-aware выбор фасетов для типов 1С:

1. **PropertyAccess всегда возвращает строку без фасета:**
   ```bsl
   СправочникКонтрагенты = Справочники.Контрагенты;
   // Ожидается: СправочникМенеджер.Контрагенты (Manager facet)
   // Сейчас: Справочники.Контрагенты (без указания фасета)
   ```

2. **Методы не переключают фасеты:**
   ```bsl
   Объект = Справочники.Контрагенты.СоздатьЭлемент();
   // Ожидается: СправочникОбъект.Контрагенты (Object facet)
   // Сейчас: Неопределено (метод не резолвится)

   Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
   // Ожидается: СправочникСсылка.Контрагенты (Reference facet)
   // Сейчас: Неопределено
   ```

3. **Отсутствует контроль серверного/клиентского контекста:**
   ```bsl
   &НаКлиенте
   Процедура КлиентскийМетод()
       // ❌ Не должно работать на клиенте (нет прямого доступа к БД)
       Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
   КонецПроцедуры
   ```

**Исследование:**

**Фасетная система типов** (из статьи Balyuk & Popova, 2021):

Один метаданный объект 1С имеет множественные представления (фасеты):

| Фасет | Название | Назначение | Пример |
|-------|----------|------------|--------|
| **Manager** | СправочникМенеджер | Создание, поиск элементов | `Справочники.Контрагенты.НайтиПоКоду()` |
| **Object** | СправочникОбъект | Изменяемый объект | `Объект = Справочники.Контрагенты.СоздатьЭлемент()` |
| **Reference** | СправочникСсылка | Ссылка на элемент | `Ссылка.Наименование` |
| **Selection** | СправочникВыборка | Обход элементов | `Выборка = Справочники.Контрагенты.Выбрать()` |
| **List** | СправочникСписок | UI представление | Для форм |

**Серверный/Клиентский контекст:**

В 1С код делится на контексты выполнения (директивы компиляции):

- `&НаСервере` — выполняется на сервере, есть доступ к БД
- `&НаСервереБезКонтекста` — на сервере без контекста формы
- `&НаКлиенте` — выполняется на клиенте, НЕТ прямого доступа к БД
- `&НаКлиентеНаСервереБезКонтекста` — универсальный код (без доступа к контексту)

**Доступность методов:**

| Метод | &НаСервере | &НаКлиенте | Return Facet |
|-------|------------|------------|--------------|
| `Справочники.X` | ✅ | ✅ | Manager |
| `.СоздатьЭлемент()` | ✅ | ❌ | Object |
| `.НайтиПоКоду()` | ✅ | ❌ | Reference |
| `.ПолучитьОбъект()` | ✅ | ❌ | Object |
| `.Выбрать()` | ✅ | ❌ | Selection |
| `.СоздатьСписокЗначений()` | ✅ | ✅ | List |

**Решение:**

Реализовать трёхуровневую архитектуру:

1. **Method Signature Registry** — хранение информации о return facet для каждого метода
2. **Context Tracker** — отслеживание текущего контекста выполнения (&НаСервере/&НаКлиенте)
3. **Facet Selection Engine** — выбор правильного фасета на основе контекста и вызываемого метода

#### Задачи:

**Phase 1: Method Signature Enhancement (3-4 дня)**

**Task 1.1: Расширить SignatureIndex (1 день)**

Добавить поля в `MethodSignature`:

```rust
// shared/src/domain/signature_index.rs
pub struct MethodSignature {
    pub name: String,
    pub owner_type: Option<String>,
    pub params: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub source: SignatureSource,

    // ✅ NEW: Информация о фасетах
    pub return_facet: Option<FacetKind>,  // Какой фасет возвращает метод
    pub context_requirements: ContextRequirements,  // Где доступен
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContextRequirements {
    ServerOnly,      // Только &НаСервере
    ClientOnly,      // Только &НаКлиенте
    Universal,       // Везде
    ServerPreferred, // Работает везде, но лучше на сервере
}
```

**Task 1.2: Populate return_facet для платформенных методов (1-2 дня)**

Обновить `backend/src/data/loaders/platform_types.rs`:

```rust
fn populate_catalog_manager_methods(index: &mut SignatureIndex, catalog_name: &str) {
    // СоздатьЭлемент() → Object facet
    index.add_platform_method(
        catalog_name.clone(),
        MethodSignature {
            name: "СоздатьЭлемент".to_string(),
            return_type: Some(format!("СправочникОбъект.{}", catalog_name)),
            return_facet: Some(FacetKind::Object),  // ✅
            context_requirements: ContextRequirements::ServerOnly,  // ✅
            // ...
        },
    );

    // НайтиПоКоду() → Reference facet
    index.add_platform_method(
        catalog_name.clone(),
        MethodSignature {
            name: "НайтиПоКоду".to_string(),
            return_type: Some(format!("СправочникСсылка.{}", catalog_name)),
            return_facet: Some(FacetKind::Reference),  // ✅
            context_requirements: ContextRequirements::ServerOnly,  // ✅
            // ...
        },
    );

    // Аналогично для других методов: НайтиПоНаименованию, ПолучитьОбъект, Выбрать...
}
```

**Task 1.3: Unit тесты для SignatureIndex (1 день)**

```rust
#[test]
fn test_method_signature_with_facet() {
    let mut index = SignatureIndex::new();

    let signature = MethodSignature {
        name: "СоздатьЭлемент".to_string(),
        return_facet: Some(FacetKind::Object),
        context_requirements: ContextRequirements::ServerOnly,
        // ...
    };

    index.add_platform_method("Справочники.Контрагенты", signature);

    let found = index.find_method("Справочники.Контрагенты", "СоздатьЭлемент");
    assert_eq!(found.unwrap().return_facet, Some(FacetKind::Object));
}
```

---

**Phase 2: Context-Aware Resolution (4-5 дней)**

**Task 2.1: ExecutionContext tracking (2 дня)**

Создать новый модуль для отслеживания контекста выполнения:

```rust
// shared/src/domain/execution_context.rs
pub struct ExecutionContext {
    pub current_directive: CompilerDirective,
    pub in_function: Option<String>,
    pub warnings: Vec<ContextWarning>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerDirective {
    OnServer,                    // &НаСервере
    OnServerNoContext,           // &НаСервереБезКонтекста
    OnClient,                    // &НаКлиенте
    OnClientOnServerNoContext,   // &НаКлиентеНаСервереБезКонтекста
    Unknown,                     // Нет директивы
}

impl ExecutionContext {
    pub fn can_call_method(&self, requirements: &ContextRequirements) -> bool {
        match (&self.current_directive, requirements) {
            (CompilerDirective::OnServer, _) => true,
            (CompilerDirective::OnClient, ContextRequirements::ServerOnly) => false,
            (CompilerDirective::OnClient, ContextRequirements::ClientOnly) => true,
            (CompilerDirective::OnClient, ContextRequirements::Universal) => true,
            // ... другие комбинации
        }
    }
}
```

**Task 2.2: Парсинг директив в AstToIrConverter (1 день)**

Добавить в `backend/src/application/ast_to_ir.rs`:

```rust
// При обработке Function/Procedure
fn convert_function(&mut self, func: &Function) -> Result<usize> {
    // Извлечь директиву из комментария перед функцией
    let directive = self.extract_compiler_directive(&func.comments);

    // Установить контекст для scope
    let context = ExecutionContext {
        current_directive: directive,
        in_function: Some(func.name.clone()),
        warnings: vec![],
    };

    self.symbol_table.set_context_for_scope(scope_id, context);

    // ...
}
```

**Task 2.3: Обновить resolve_member_access() (1 день)**

Обновить `shared/src/domain/resolver.rs`:

```rust
fn resolve_member_access(&self, base: &str, member: &str, context: Option<&ExecutionContext>) -> TypeResolution {
    // Для PropertyAccess на глобальные коллекции
    let (kind, prefix) = match base {
        "Справочники" | "Catalogs" => (MetadataKind::Catalog, "Справочники"),
        "Документы" | "Documents" => (MetadataKind::Document, "Документы"),
        _ => return self.resolve_unknown_member(base, member),
    };

    let type_name = format!("{}.{}", prefix, member);

    // ✅ Всегда возвращаем Manager facet для PropertyAccess
    TypeResolution {
        type_name: format!("{}Менеджер.{}",
            kind_to_prefix(kind),  // Справочники → Справочник
            member
        ),
        active_facet: Some(FacetKind::Manager),
        // ...
    }
}
```

**Task 2.4: Facet switching в infer_expression_type() (1 день)**

Обновить обработку `Expression::Call`:

```rust
// backend/src/application/ast_to_ir.rs
Expression::Call { function, .. } => {
    match function.as_ref() {
        Expression::PropertyAccess { object, property, .. } => {
            let receiver_type = self.infer_expression_type(object);

            // ✅ NEW: Ищем метод в SignatureIndex
            if let Some(signature) = self.signature_index.find_method(&receiver_type, property) {
                // ✅ Используем return_facet если указан
                if let Some(facet) = signature.return_facet {
                    return format!("{}{}. {}",
                        extract_kind_from_type(&receiver_type),  // Справочник
                        facet_to_suffix(facet),  // Object → Объект
                        extract_name_from_type(&receiver_type)   // Контрагенты
                    );
                }

                // Fallback на return_type
                signature.return_type.unwrap_or_else(|| "Dynamic".to_string())
            } else {
                "Dynamic".to_string()
            }
        }
        // ...
    }
}
```

---

**Phase 3: Diagnostics для контекстных ошибок (3-4 дня)**

**Task 3.1: Context validation в SemanticValidator (2 дня)**

Добавить проверку доступности методов в контексте:

```rust
// backend/src/application/semantic_validation_visitor.rs
fn visit_function_call(&mut self, call: &FunctionCall) {
    // Получаем текущий контекст из scope
    let context = self.ir.get_context_for_scope(self.current_scope);

    // Проверяем метод
    if let Some(signature) = self.signature_index.find_method(&receiver_type, &method_name) {
        // ✅ NEW: Валидация контекста
        if !context.can_call_method(&signature.context_requirements) {
            self.errors.push(TypeDiagnostic {
                kind: DiagnosticKind::MethodNotAvailableInContext,
                message: format!(
                    "Метод '{}' недоступен в контексте {:?}. Требуется {:?}",
                    method_name,
                    context.current_directive,
                    signature.context_requirements
                ),
                severity: Severity::Warning,
                // ...
            });
        }
    }
}
```

**Task 3.2: LSP Diagnostics integration (1 день)**

Обновить `backend/src/bin/lsp_server.rs`:
- Конвертировать `MethodNotAvailableInContext` → LSP Diagnostic
- Severity = Warning (не Error, т.к. может работать)

**Task 3.3: Integration тесты (1 день)**

Создать `backend/tests/context_aware_facets_test.rs`:

```rust
#[test]
fn test_property_access_returns_manager_facet() {
    let code = r#"
        СправочникКонтрагенты = Справочники.Контрагенты;
    "#;

    let type_name = infer_variable_type(code, "СправочникКонтрагенты");
    assert_eq!(type_name, "СправочникМенеджер.Контрагенты");
}

#[test]
fn test_create_element_returns_object_facet() {
    let code = r#"
        Объект = Справочники.Контрагенты.СоздатьЭлемент();
    "#;

    let type_name = infer_variable_type(code, "Объект");
    assert_eq!(type_name, "СправочникОбъект.Контрагенты");
}

#[test]
fn test_find_by_code_returns_reference_facet() {
    let code = r#"
        Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
    "#;

    let type_name = infer_variable_type(code, "Ссылка");
    assert_eq!(type_name, "СправочникСсылка.Контрагенты");
}

#[test]
fn test_server_only_method_in_client_context() {
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = validate_semantics(code);
    assert!(diagnostics.iter().any(|d|
        d.kind == DiagnosticKind::MethodNotAvailableInContext &&
        d.message.contains("НайтиПоКоду")
    ));
}
```

---

**Phase 4: VSCode UX improvements (2-3 дня)**

**Task 4.1: Hover enhancement (1 день)**

Обновить `backend/src/helpers/hover_formatter.rs`:

```rust
// Показывать активный фасет
if let Some(facet) = type_resolution.active_facet {
    content.push_str(&format!("\n**Фасет:** {}", facet_to_russian(facet)));
}

// Показывать доступные методы для текущего фасета
content.push_str("\n\n**Доступные методы:");
for method in type_info.methods.iter().filter(|m| m.facet == active_facet) {
    content.push_str(&format!("\n- `{}`", method.name));
}
```

**Task 4.2: Code completion filtering (1 день)**

Обновить completion provider:
- Фильтровать методы по текущему контексту
- Группировать по фасетам
- Показывать warning для недоступных методов

**Task 4.3: Documentation (1 день)**

Создать `docs/features/facet-system.md`:
- Объяснение фасетной системы типов
- Примеры использования
- Таблица методов с фасетами
- Серверный/клиентский контекст

#### Критерии успеха:

1. ✅ **PropertyAccess → Manager facet:**
   ```bsl
   М = Справочники.Контрагенты;  // Тип: СправочникМенеджер.Контрагенты
   ```

2. ✅ **Method calls переключают фасеты:**
   ```bsl
   Объект = М.СоздатьЭлемент();      // Тип: СправочникОбъект.Контрагенты
   Ссылка = М.НайтиПоКоду("001");   // Тип: СправочникСсылка.Контрагенты
   Выборка = М.Выбрать();            // Тип: СправочникВыборка.Контрагенты
   ```

3. ✅ **Context validation работает:**
   ```bsl
   &НаКлиенте
   Процедура Test()
       // Warning: НайтиПоКоду недоступен в клиентском контексте
       Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
   КонецПроцедуры
   ```

4. ✅ **Hover показывает фасет и контекст:**
   ```
   Переменная: СправочникКонтрагенты
   Тип: СправочникМенеджер.Контрагенты
   Фасет: Manager
   Контекст: Universal

   Доступные методы (Manager):
   - СоздатьЭлемент() → Object (ServerOnly)
   - НайтиПоКоду(Код: Строка) → Reference (ServerOnly)
   - Выбрать() → Selection (ServerOnly)
   ```

5. ✅ **50+ тестов проходят:**
   - 15 unit тестов SignatureIndex (return_facet, context_requirements)
   - 20 integration тестов (facet switching, context validation)
   - 15 LSP diagnostics тестов (warnings для клиентского контекста)

#### Архитектура:

**Новые компоненты:**

1. **`shared/src/domain/execution_context.rs`** (NEW)
   - ExecutionContext struct
   - CompilerDirective enum
   - ContextRequirements enum
   - Логика проверки доступности

2. **`shared/src/domain/facet_selector.rs`** (NEW)
   - FacetSelector trait
   - Логика выбора фасета по методу

**Изменяемые компоненты:**

1. **`shared/src/domain/signature_index.rs`**
   - Добавить return_facet и context_requirements в MethodSignature

2. **`backend/src/application/ast_to_ir.rs`**
   - Парсинг директив компиляции (&НаСервере, &НаКлиенте)
   - Хранение ExecutionContext в scope
   - Facet-aware type inference для PropertyAccess и Call

3. **`backend/src/data/loaders/platform_types.rs`**
   - Заполнение return_facet для всех методов Manager
   - Установка context_requirements

4. **`backend/src/application/semantic_validation_visitor.rs`**
   - Валидация доступности методов в контексте

#### Зависимости:

**Используют этот Milestone:**
- ✅ Milestone 3.9 (Return Type Inference) — интегрируется с SignatureIndex
- ✅ Milestone 3.10 (Parameter Validation) — использует обновлённый SignatureIndex
- ✅ Milestone 3.6 (Enhanced UX) — показывает фасеты в hover

**Этот Milestone использует:**
- 🔴 **Milestone 3.12 (Enhanced Config Parser)** — **КРИТИЧЕСКАЯ ЗАВИСИМОСТЬ:** CodeLocation, ModuleContextProperties, FormMetadata
- ✅ Milestone 2.8 (Semantic IR Layer) — SemanticProgram, SymbolTable
- ✅ Milestone 2.17 (Configuration Metadata Parser) — базовая инфраструктура (расширяется в 3.12)
- ✅ Milestone 3.7 (Semantic Diagnostics) — инфраструктура для warnings

**⚠️ ВАЖНО:** Milestone 3.12 должен быть реализован ПЕРВЫМ, так как предоставляет:
- CodeLocation для определения контекста по месту кода (тип модуля)
- ModuleContextProperties для общих модулей (Server/Client/ServerCall)
- FormMetadata для работы с формами и их реквизитами

**Последовательность реализации:**
```
3.12 Enhanced Config Parser → 3.11 Context-Aware Facets
```

#### Риски и митигация:

| Риск | Вероятность | Влияние | Митигация |
|------|-------------|---------|-----------|
| Регрессии в hover/diagnostics | Средняя | Высокое | 50+ интеграционных тестов перед merge |
| Сложность парсинга директив | Низкая | Среднее | Простой regex для `&НаСервере` из комментариев |
| Производительность | Низкая | Среднее | Кеширование facet resolution в IR |
| Неполнота методов с фасетами | Высокая | Низкое | Инкрементальное добавление (начнём с 10-15 ключевых методов) |

#### Оценка времени:

- **Phase 1:** 3-4 дня (SignatureIndex enhancement)
- **Phase 2:** 4-5 дней (Context-aware resolution)
- **Phase 3:** 3-4 дня (Diagnostics)
- **Phase 4:** 2-3 дня (VSCode UX)
- **Итого:** 12-16 дней (2-3 недели)

**Буфер:** +3-5 дней для тестирования и отладки
**Всего:** 3-4 недели

---

### 🎯 Результаты Версии 3.0 (через 6 месяцев от старта)

**Технические метрики:**
- ✅ Goto Definition, Find References, Rename
- ✅ 20+ Code Actions (Quick Fixes, Refactorings)
- ✅ 50+ Static Analysis Rules
- ✅ Code Quality Dashboard
- ✅ Flow-Sensitive Analysis — hover корректно работает на вызовах методов
- ✅ Semantic Diagnostics MVP — несуществующие методы/свойства показываются в LSP
- ✅ Enhanced Hover — три уровня детализации, фасеты, Generic типы, ссылки на документацию
- ✅ LSP Settings для кастомизации hover (как Rust Analyzer)
- ✅ MCP Server для интеграции с LLM (Claude, ChatGPT)
- ✅ File Watching (Windows/Linux/macOS) через notify
- ✅ Resources, Tools, Prompts для AI-ассистентов

**Пользовательские метрики:**
- ✅ Навигация как в IntelliJ IDEA
- ✅ Рефакторинг одним кликом
- ✅ Автоматическое улучшение качества кода
- ✅ Предотвращение security & performance проблем
- ✅ Hover показывает тип переменной даже при вызове методов (исправлен баг из test_hover_milestone_2_11.bsl)
- ✅ Hover кастомизируется через VSCode Settings (compact/full/detailed)
- ✅ Фасеты объясняются понятно (Manager vs Object vs Reference)
- ✅ Ссылки на platform documentation в hover
- ✅ Красные волнистые линии для несуществующих методов/свойств (покрывает ~70% типовых ошибок)
- ✅ Semantic diagnostics в реальном времени (latency <10ms)
- ✅ AI-ассистент с полным контекстом BSL проекта
- ✅ Генерация кода с типизацией через Claude

---

## 🔬 Версия 3.5 — "LLVM-inspired Static Analysis" (Q2 2025: 4-6 недель)

**Цель:** Внедрить продвинутый статический анализ кода BSL по мотивам LLVM/Clang Static Analyzer

**Философия:** Использовать проверенные подходы из LLVM экосистемы для создания мощного статического анализатора 1С кода без использования самого LLVM IR (который слишком низкоуровневый для динамического языка 1С).

**Контекст:**
LLVM (Low Level Virtual Machine) — это компиляторная инфраструктура, которая включает:
- LLVM Core — backend для оптимизации и генерации кода
- Clang — C/C++ компилятор с мощным статическим анализатором
- LLDB — debugger (используется в Milestone 4.4 через CodeLLDB)

Rust компилятор (rustc) использует LLVM backend для генерации машинного кода. Многие идеи из Clang Static Analyzer можно адаптировать для BSL.

---

### 📊 Milestone 5.0: Advanced Static Analysis (по мотивам LLVM)

**Приоритет:** 🟡 СРЕДНИЙ — значительное улучшение качества статического анализа

**Проблема:**
Текущий TypeResolver проверяет только типы. Нужны более глубокие анализы:
- Null Safety — обнаружение обращений к `Неопределено` до runtime
- Dead Code Detection — неиспользуемые переменные/функции/недостижимый код
- Control Flow Analysis — анализ путей выполнения
- Data Flow Analysis — отслеживание изменений переменных

**Вдохновение:** Clang Static Analyzer использует Analysis Passes — независимые проходы по AST/IR для различных видов анализа.

#### Архитектура: Analysis Pipeline

**Концепция:**
```
BSL код → Tree-sitter AST → Semantic IR → Analysis Passes → Diagnostics
                                              ↓
                                    [Pass 1: Type Safety]
                                    [Pass 2: Null Safety]
                                    [Pass 3: Dead Code]
                                    [Pass 4: Control Flow]
                                    [Pass 5: Data Flow]
```

**Преимущества подхода:**
- ✅ Модульность — каждый pass независим (как в LLVM)
- ✅ Масштабируемость — легко добавлять новые passes
- ✅ Переиспользование — passes работают с единым Semantic IR
- ✅ Производительность — можно распараллелить (rayon)

#### Задачи:

**Task 1: Архитектура Analysis Pipeline (3-4 дня)**

Создать базовую инфраструктуру для analysis passes:

```rust
// backend/src/analysis/mod.rs
pub mod pass;           // Trait для analysis passes
pub mod pipeline;       // Pipeline для выполнения passes
pub mod null_safety;    // Pass для null safety
pub mod dead_code;      // Pass для dead code
pub mod control_flow;   // Pass для control flow
pub mod data_flow;      // Pass для data flow

// backend/src/analysis/pass.rs
use crate::semantic_ir::SemanticProgram;
use crate::types::Diagnostic;

/// Trait для analysis pass (аналог LLVM Pass)
pub trait AnalysisPass: Send + Sync {
    /// Имя pass (для логирования/отладки)
    fn name(&self) -> &str;

    /// Запуск анализа на Semantic IR
    fn run(&self, program: &SemanticProgram) -> Vec<Diagnostic>;

    /// Приоритет (порядок выполнения в pipeline)
    /// Lower number = higher priority
    fn priority(&self) -> u32 {
        100
    }
}

// backend/src/analysis/pipeline.rs
pub struct AnalysisPipeline {
    passes: Vec<Box<dyn AnalysisPass>>,
}

impl AnalysisPipeline {
    pub fn new() -> Self {
        Self {
            passes: vec![
                Box::new(TypeSafetyPass),      // Priority 10
                Box::new(NullSafetyPass),      // Priority 20
                Box::new(DeadCodePass),        // Priority 30
                Box::new(ControlFlowPass),     // Priority 40
                Box::new(DataFlowPass),        // Priority 50
            ],
        }
    }

    /// Запуск всех passes с сортировкой по priority
    pub fn run(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        let mut sorted_passes = self.passes.clone();
        sorted_passes.sort_by_key(|p| p.priority());

        let mut all_diagnostics = vec![];
        for pass in sorted_passes {
            tracing::debug!("Running analysis pass: {}", pass.name());
            let diagnostics = pass.run(program);
            all_diagnostics.extend(diagnostics);
        }

        all_diagnostics
    }

    /// Параллельный запуск passes (для больших проектов)
    pub fn run_parallel(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        use rayon::prelude::*;

        self.passes.par_iter()
            .flat_map(|pass| pass.run(program))
            .collect()
    }
}
```

**Интеграция с TypeSystemService:**
```rust
// backend/src/application/type_system_service.rs
impl TypeSystemService {
    pub fn analyze_with_advanced_passes(&self, code: &str) -> Result<AnalysisReport> {
        // 1. Парсинг в Semantic IR (уже есть)
        let program = self.parse(code)?;

        // 2. Запуск Type Safety (уже есть в TypeResolver)
        let type_diagnostics = self.type_resolver.validate(&program)?;

        // 3. Запуск Advanced Analysis Pipeline (новое!)
        let pipeline = AnalysisPipeline::new();
        let advanced_diagnostics = pipeline.run(&program);

        // 4. Объединение результатов
        Ok(AnalysisReport {
            type_diagnostics,
            advanced_diagnostics,
            summary: self.generate_summary(),
        })
    }
}
```

---

**Task 2: Null Safety Analysis Pass (4-5 дней)**

**Цель:** Обнаружить потенциальные NullPointerException на этапе анализа

**Примеры проблем:**
```bsl
// Пример 1: Прямое использование Неопределено
Переменная = Неопределено;
Переменная.Метод();  // ❌ Runtime ошибка!

// Пример 2: Необработанный результат функции
Результат = НайтиПоИдентификатору(ИД);  // может вернуть Неопределено
Результат.Удалить();  // ❌ Потенциальная ошибка

// Пример 3: Условный null
Если УсловиеВыполнено Тогда
    Переменная = НовыйОбъект();
КонецЕсли;
Переменная.Сохранить();  // ❌ Переменная может быть Неопределено
```

**Реализация:**
```rust
// backend/src/analysis/null_safety.rs
pub struct NullSafetyPass;

impl AnalysisPass for NullSafetyPass {
    fn name(&self) -> &str {
        "Null Safety Analysis"
    }

    fn priority(&self) -> u32 {
        20  // После Type Safety (10)
    }

    fn run(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let null_tracker = NullTracker::new();

        // Проходим по всем statements
        for stmt in &program.statements {
            match stmt {
                Statement::Assignment { target, value, span } => {
                    // Отслеживаем присвоения Неопределено
                    if self.is_undefined(value) {
                        null_tracker.mark_as_nullable(target);
                    }
                }
                Statement::MethodCall { receiver, method, span } => {
                    // Проверяем, может ли receiver быть null
                    if null_tracker.is_potentially_null(receiver) {
                        diagnostics.push(Diagnostic::warning(
                            format!("Потенциальное обращение к Неопределено: {}.{}()",
                                   receiver, method),
                            *span,
                        ));
                    }
                }
                _ => {}
            }
        }

        diagnostics
    }
}

/// Трекер для отслеживания nullable переменных
struct NullTracker {
    nullable_vars: HashSet<String>,
}

impl NullTracker {
    fn mark_as_nullable(&mut self, var: &str) {
        self.nullable_vars.insert(var.to_string());
    }

    fn is_potentially_null(&self, var: &str) -> bool {
        self.nullable_vars.contains(var)
    }
}
```

**Тесты:**
```rust
#[test]
fn test_null_safety_direct_undefined() {
    let code = r#"
        Переменная = Неопределено;
        Переменная.Метод();
    "#;

    let diagnostics = run_null_safety_pass(code);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Потенциальное обращение к Неопределено"));
}

#[test]
fn test_null_safety_function_result() {
    let code = r#"
        Результат = ПолучитьДанные();  // может вернуть Неопределено
        Результат.Обработать();
    "#;

    let diagnostics = run_null_safety_pass(code);
    // Должно быть предупреждение о потенциальном null
}
```

**Результат:**
- ✅ Обнаружение ~80% потенциальных NullPointerException
- ✅ Предупреждения в LSP Diagnostics
- ✅ Quick Fix: "Add null check" (генерация `Если Переменная <> Неопределено Тогда`)

---

**Task 3: Dead Code Detection Pass (3-4 дня)**

**Цель:** Найти неиспользуемый код (переменные, функции, недостижимые блоки)

**Примеры проблем:**
```bsl
// Пример 1: Неиспользуемая переменная
Переменная = 10;
// Переменная нигде не используется

// Пример 2: Недостижимый код после Возврат
Функция Пример()
    Возврат Истина;
    Сообщить("Этот код никогда не выполнится");  // ❌ Dead code
КонецФункции

// Пример 3: Неиспользуемая функция
Функция НеиспользуемаяФункция()  // ❌ Никто не вызывает
    Возврат 42;
КонецФункции
```

**Реализация:**
```rust
// backend/src/analysis/dead_code.rs
pub struct DeadCodePass;

impl AnalysisPass for DeadCodePass {
    fn name(&self) -> &str {
        "Dead Code Detection"
    }

    fn priority(&self) -> u32 {
        30
    }

    fn run(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // 1. Найти неиспользуемые переменные
        diagnostics.extend(self.find_unused_variables(program));

        // 2. Найти недостижимый код
        diagnostics.extend(self.find_unreachable_code(program));

        // 3. Найти неиспользуемые функции
        diagnostics.extend(self.find_unused_functions(program));

        diagnostics
    }
}

impl DeadCodePass {
    fn find_unused_variables(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let usage_tracker = VariableUsageTracker::new();

        // Собираем все объявления и использования
        for stmt in &program.statements {
            usage_tracker.visit(stmt);
        }

        // Находим переменные, которые объявлены но не используются
        for (var_name, declaration_span) in usage_tracker.declarations() {
            if !usage_tracker.is_used(var_name) {
                diagnostics.push(Diagnostic::warning(
                    format!("Неиспользуемая переменная: {}", var_name),
                    declaration_span,
                ));
            }
        }

        diagnostics
    }

    fn find_unreachable_code(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        for func in &program.functions {
            let mut found_return = false;

            for stmt in &func.body {
                if found_return {
                    diagnostics.push(Diagnostic::warning(
                        "Недостижимый код после Возврат".to_string(),
                        stmt.span(),
                    ));
                }

                if matches!(stmt, Statement::Return { .. }) {
                    found_return = true;
                }
            }
        }

        diagnostics
    }

    fn find_unused_functions(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        // Найти функции, на которые нет ссылок
        // (сложнее - требует call graph analysis)
        vec![]
    }
}
```

**Результат:**
- ✅ Обнаружение неиспользуемых переменных
- ✅ Обнаружение недостижимого кода
- ✅ Quick Fix: "Remove unused variable/code"
- ✅ Улучшение качества кода на ~15-20%

---

**Task 4: Control Flow Analysis Pass (5-6 дней)**

**Цель:** Анализ путей выполнения программы

**Примеры проблем:**
```bsl
// Пример 1: Неинициализированная переменная на одном из путей
Если Условие Тогда
    Переменная = 10;
КонецЕсли;
// Если Условие = Ложь, Переменная не инициализирована
Результат = Переменная + 5;  // ❌ Потенциальная ошибка

// Пример 2: Функция не всегда возвращает значение
Функция ПолучитьЗначение(Параметр)
    Если Параметр > 0 Тогда
        Возврат Параметр * 2;
    КонецЕсли;
    // ❌ Нет возврата при Параметр <= 0
КонецФункции

// Пример 3: Бесконечный цикл
Пока Истина Цикл
    Сообщить("Бесконечный цикл");
    // Нет break или return
КонецЦикла;
Сообщить("Этот код недостижим");
```

**Реализация:**
```rust
// backend/src/analysis/control_flow.rs
pub struct ControlFlowPass;

impl AnalysisPass for ControlFlowPass {
    fn name(&self) -> &str {
        "Control Flow Analysis"
    }

    fn priority(&self) -> u32 {
        40
    }

    fn run(&self, program: &SemanticProgram) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // 1. Построить Control Flow Graph (CFG)
        let cfg = ControlFlowGraph::build(program);

        // 2. Анализ путей выполнения
        diagnostics.extend(self.analyze_uninitialized_variables(&cfg));
        diagnostics.extend(self.analyze_missing_returns(&cfg));
        diagnostics.extend(self.analyze_infinite_loops(&cfg));

        diagnostics
    }
}

/// Control Flow Graph
struct ControlFlowGraph {
    nodes: Vec<CfgNode>,
    edges: Vec<(usize, usize)>,  // (from, to)
}

enum CfgNode {
    Entry,
    Statement(Statement),
    Branch { condition: Expression, true_branch: usize, false_branch: usize },
    Loop { body: usize, exit: usize },
    Exit,
}

impl ControlFlowGraph {
    fn build(program: &SemanticProgram) -> Self {
        // Построение CFG из statements
        // Аналогично LLVM BasicBlock и PHI nodes (но проще)
        todo!("Implement CFG construction")
    }
}
```

**Алгоритмы:**
- **Reaching Definitions** — какие переменные определены на каждом пути
- **Live Variables** — какие переменные используются после текущей точки
- **Dominators** — какие узлы обязательно выполняются

**Результат:**
- ✅ Обнаружение неинициализированных переменных на ~90% путей
- ✅ Проверка полноты return в функциях
- ✅ Предупреждения о потенциально бесконечных циклах

---

**Task 5: Data Flow Analysis Pass (опционально, 4-5 дней)**

**Цель:** Отслеживание изменений значений переменных

**Примеры проблем:**
```bsl
// Пример 1: Перезапись без использования
Переменная = ПолучитьДанные();  // Дорогая операция
Переменная = 10;  // ❌ Предыдущее значение не использовалось

// Пример 2: Use after free (для объектных переменных)
Объект = НовыйОбъект();
Объект.Удалить();
Объект.Метод();  // ❌ Использование после удаления
```

**Результат:**
- ✅ Обнаружение неэффективных перезаписей
- ✅ Обнаружение use-after-free для объектных переменных
- ✅ Оптимизация производительности кода

---

**Task 6: Интеграция с LSP и VSCode Extension (2-3 дня)**

**Цель:** Показывать результаты анализа в редакторе

**LSP Integration:**
```rust
// lsp-server/src/handlers/diagnostics.rs
impl LspServer {
    pub async fn send_advanced_diagnostics(&self, uri: &Url, code: &str) {
        let analysis_report = self.type_service.analyze_with_advanced_passes(code)?;

        let lsp_diagnostics: Vec<LspDiagnostic> = analysis_report
            .advanced_diagnostics
            .into_iter()
            .map(|d| LspDiagnostic {
                range: d.span.to_lsp_range(),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(d.code.into()),
                source: Some("bsl-advanced-analysis".to_string()),
                message: d.message,
                ..Default::default()
            })
            .collect();

        self.client.publish_diagnostics(uri.clone(), lsp_diagnostics, None).await;
    }
}
```

**VSCode Extension:**
```typescript
// extension/src/features/advancedAnalysis.ts
export class AdvancedAnalysisProvider {
    async analyzeDocument(document: vscode.TextDocument): Promise<void> {
        // Запрос к LSP для advanced analysis
        const diagnostics = await this.client.sendRequest(
            'bsl/analyzeAdvanced',
            { uri: document.uri.toString() }
        );

        // Отображение в Problems panel
        this.diagnosticCollection.set(document.uri, diagnostics);
    }
}
```

**Результат:**
- ✅ Все advanced diagnostics в Problems panel
- ✅ Цветовая кодировка по severity (error/warning/info)
- ✅ Quick Fixes для распространённых проблем
- ✅ Настройка через VSCode Settings: `bsl.analysis.enableAdvanced`

---

#### Тестирование:

**Unit-тесты:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_safety_pass() {
        let code = "Переменная = Неопределено; Переменная.Метод();";
        let diagnostics = run_analysis_pass::<NullSafetyPass>(code);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_dead_code_pass() {
        let code = "Функция F() Возврат 1; Сообщить('dead'); КонецФункции";
        let diagnostics = run_analysis_pass::<DeadCodePass>(code);
        assert!(diagnostics.iter().any(|d| d.message.contains("Недостижимый")));
    }

    #[test]
    fn test_control_flow_pass() {
        let code = "Функция F(X) Если X > 0 Тогда Возврат X; КонецЕсли; КонецФункции";
        let diagnostics = run_analysis_pass::<ControlFlowPass>(code);
        // Должно быть предупреждение о missing return
    }
}
```

**Integration тесты:**
```bash
cargo test -p bsl-backend --test advanced_analysis_integration
```

---

#### Результаты Milestone 5.0:

**Технические метрики:**
- ✅ 5 новых analysis passes (Null Safety, Dead Code, Control Flow, Data Flow, Type Safety)
- ✅ Analysis Pipeline архитектура (модульная, расширяемая)
- ✅ 50+ unit-тестов для каждого pass
- ✅ Интеграция с LSP (real-time diagnostics)
- ✅ Quick Fixes для 80% найденных проблем

**Пользовательские метрики:**
- ✅ Обнаружение ~80% потенциальных NullPointerException до runtime
- ✅ Обнаружение ~90% dead code и неиспользуемых переменных
- ✅ Обнаружение ~85% проблем с control flow (неинициализированные переменные, missing returns)
- ✅ Улучшение качества кода на 20-30% (по метрикам code quality)
- ✅ Сокращение runtime ошибок на 40-50%

**Производительность:**
- ⚡ Анализ файла < 50ms (для файлов ~500 строк)
- ⚡ Параллельный анализ workspace (1000 файлов) < 2 минуты
- ⚡ Real-time diagnostics с latency < 100ms

**Документация:**
- 📚 Руководство: "LLVM-inspired Analysis для BSL" (15-20 страниц)
- 📚 API документация для создания custom passes
- 📚 Примеры: 10 кейсов использования advanced analysis

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

### 🐛 Milestone 4.4: MCP Debug Server — AI-Powered Interactive Debugging (2-3 недели)

**Приоритет:** 🟢 ВЫСОКИЙ — революционная возможность для AI-ассистированной разработки

**Статус:** 📋 PLANNED

**Проблема:**

AI-ассистенты (Claude Code, ChatGPT, etc.) **НЕ могут** интерактивно отлаживать программы через debugger:
- ❌ Нет доступа к GDB/LLDB
- ❌ Не могут устанавливать breakpoints
- ❌ Не могут инспектировать переменные step-by-step
- ❌ Вынуждены использовать print debugging

**Текущий workflow отладки:**
```
1. AI добавляет println!() →
2. Пересборка →
3. Запуск →
4. Анализ output →
5. Повтор
```

⏱️ **Медленно:** 5-10 минут на итерацию

**Цель:**

Создать **MCP Debug Server** с DAP bridge, чтобы AI мог отлаживать программы **интерактивно** как профессиональный разработчик:
- ✅ Установка breakpoints
- ✅ Step-by-step execution
- ✅ Инспекция переменных
- ✅ Stack traces
- ✅ Conditional breakpoints
- ✅ Watch expressions

⏱️ **Быстро:** <1 минута на итерацию

---

**Архитектура:**

```
┌────────────────┐   MCP Protocol    ┌──────────────────┐   DAP Protocol   ┌──────────────┐
│  Claude Code   │ ◄───────────────► │  MCP Debug Server│ ◄───────────────►│  CodeLLDB    │
│  (AI Agent)    │                    │   (Rust crate)   │                  │  (DAP server)│
└────────────────┘                    └──────────────────┘                  └──────────────┘
                                              │                                      │
                                              │                                      ▼
                                              ▼                              ┌──────────────┐
                                       ┌──────────────────┐                  │   LLDB/GDB   │
                                       │  Session Manager │                  │  (debugger)  │
                                       │  (state tracking)│                  └──────────────┘
                                       └──────────────────┘
```

**Принципы:**
- **Protocol Layering:** MCP (для AI) → DAP (для debugger)
- **Language Agnostic:** Работает для Rust, C++, Go, Python (любой язык с DAP support)
- **Stateful Sessions:** Поддержка нескольких debug сессий одновременно
- **Right-Sized:** Переиспользуем существующие DAP servers (CodeLLDB, vscode-cpptools)

---

#### Задачи:

**Task 1: Создание MCP Debug Server crate (2 дня)**

Создать новый crate `mcp-debug-server/`:

```toml
# mcp-debug-server/Cargo.toml
[package]
name = "mcp-debug-server"
version = "0.1.0"

[[bin]]
name = "mcp-debug"
path = "src/main.rs"

[dependencies]
rmcp = { version = "0.8.5", features = ["server", "macros"] }  # Официальный Anthropic SDK
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }  # Для генерации session IDs
```

Структура модулей:
```rust
pub mod server;      // MCP server implementation
pub mod dap_client;  // DAP protocol client
pub mod session;     // Debug session management
pub mod tools;       // MCP Tools для debugging
pub mod resources;   // MCP Resources для debug info
```

---

**Task 2: DAP Client Implementation (3-4 дня)**

Реализовать DAP (Debug Adapter Protocol) client для общения с CodeLLDB/vscode-cpptools:

```rust
// mcp-debug-server/src/dap_client.rs
use serde_json::{json, Value};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// DAP Client для общения с debug adapter (CodeLLDB, etc.)
pub struct DapClient {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    seq_counter: u32,
}

impl DapClient {
    /// Запускает DAP server (CodeLLDB)
    pub async fn spawn(adapter_path: &str) -> Result<Self> {
        let mut child = tokio::process::Command::new(adapter_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        Ok(Self {
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            process: child,
            seq_counter: 1,
        })
    }

    /// Инициализация debug сессии
    pub async fn initialize(&mut self) -> Result<Value> {
        self.send_request("initialize", json!({
            "clientID": "mcp-debug-server",
            "adapterID": "lldb",
            "linesStartAt1": true,
            "columnsStartAt1": true,
        })).await
    }

    /// Установить breakpoint
    pub async fn set_breakpoint(&mut self, file: &str, line: u32) -> Result<Value> {
        self.send_request("setBreakpoints", json!({
            "source": { "path": file },
            "breakpoints": [{ "line": line }]
        })).await
    }

    /// Запустить программу
    pub async fn launch(&mut self, program: &str, args: Vec<String>) -> Result<Value> {
        self.send_request("launch", json!({
            "program": program,
            "args": args,
            "stopOnEntry": false,
        })).await
    }

    /// Step into
    pub async fn step_in(&mut self, thread_id: u32) -> Result<Value> {
        self.send_request("stepIn", json!({
            "threadId": thread_id
        })).await
    }

    /// Step over
    pub async fn next(&mut self, thread_id: u32) -> Result<Value> {
        self.send_request("next", json!({
            "threadId": thread_id
        })).await
    }

    /// Continue execution
    pub async fn continue_execution(&mut self, thread_id: u32) -> Result<Value> {
        self.send_request("continue", json!({
            "threadId": thread_id
        })).await
    }

    /// Получить stack trace
    pub async fn stack_trace(&mut self, thread_id: u32) -> Result<Value> {
        self.send_request("stackTrace", json!({
            "threadId": thread_id
        })).await
    }

    /// Получить значение переменной
    pub async fn evaluate(&mut self, expression: &str, frame_id: u32) -> Result<Value> {
        self.send_request("evaluate", json!({
            "expression": expression,
            "frameId": frame_id,
            "context": "hover"
        })).await
    }

    /// Отправить DAP request и получить response
    async fn send_request(&mut self, command: &str, args: Value) -> Result<Value> {
        let seq = self.seq_counter;
        self.seq_counter += 1;

        let request = json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": args
        });

        // DAP использует Content-Length header
        let body = serde_json::to_string(&request)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(body.as_bytes()).await?;
        self.stdin.flush().await?;

        // Читаем response
        self.read_response().await
    }

    /// Читает DAP response
    async fn read_response(&mut self) -> Result<Value> {
        // Читаем Content-Length header
        let mut header = String::new();
        self.stdout.read_line(&mut header).await?;

        let length: usize = header
            .trim_start_matches("Content-Length: ")
            .trim()
            .parse()?;

        // Пропускаем пустую строку
        self.stdout.read_line(&mut String::new()).await?;

        // Читаем JSON body
        let mut buffer = vec![0u8; length];
        self.stdout.read_exact(&mut buffer).await?;

        let response: Value = serde_json::from_slice(&buffer)?;
        Ok(response)
    }
}
```

---

**Task 3: Debug Session Manager (2 дня)**

Управление состоянием debug сессий:

```rust
// mcp-debug-server/src/session.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ID debug сессии
pub type SessionId = String;

/// Информация о debug сессии
pub struct DebugSession {
    pub id: SessionId,
    pub dap_client: DapClient,
    pub binary_path: String,
    pub current_thread_id: Option<u32>,
    pub breakpoints: HashMap<String, Vec<u32>>,
    pub state: SessionState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionState {
    Initialized,
    Running,
    Stopped,
    Terminated,
}

/// Менеджер debug сессий
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, DebugSession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Создать новую debug сессию
    pub async fn create_session(
        &self,
        binary_path: String,
        adapter_path: String,
    ) -> Result<SessionId> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let mut dap_client = DapClient::spawn(&adapter_path).await?;
        dap_client.initialize().await?;

        let session = DebugSession {
            id: session_id.clone(),
            dap_client,
            binary_path,
            current_thread_id: None,
            breakpoints: HashMap::new(),
            state: SessionState::Initialized,
        };

        self.sessions.write().await.insert(session_id.clone(), session);
        Ok(session_id)
    }

    /// Выполнить команду в сессии
    pub async fn with_session<F, R>(&self, session_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut DebugSession) -> Result<R>,
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        f(session)
    }
}
```

---

**Task 4: MCP Tools для debugging (3 дня)**

Реализовать MCP Tools для управления debug сессией:

```rust
// mcp-debug-server/src/tools.rs
use rmcp::macros::tool;

/// Создать новую debug сессию
#[tool]
pub async fn create_debug_session(
    binary_path: String,
    adapter_path: Option<String>,
    manager: Arc<SessionManager>,
) -> Result<String> {
    let adapter = adapter_path.unwrap_or_else(|| {
        // Default: CodeLLDB для Rust
        "codelldb".to_string()
    });

    let session_id = manager.create_session(binary_path, adapter).await?;

    Ok(format!("Debug session created: {}", session_id))
}

/// Установить breakpoint
#[tool]
pub async fn set_breakpoint(
    session_id: String,
    file: String,
    line: u32,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        session.dap_client.set_breakpoint(&file, line).await?;

        // Сохраняем breakpoint в сессии
        session.breakpoints
            .entry(file.clone())
            .or_insert_with(Vec::new)
            .push(line);

        Ok(format!("Breakpoint set at {}:{}", file, line))
    }).await
}

/// Запустить программу
#[tool]
pub async fn debug_run(
    session_id: String,
    args: Vec<String>,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        let response = session.dap_client.launch(&session.binary_path, args).await?;
        session.state = SessionState::Running;

        Ok(format!("Program started: {:?}", response))
    }).await
}

/// Step into
#[tool]
pub async fn debug_step(
    session_id: String,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        let thread_id = session.current_thread_id
            .ok_or_else(|| anyhow::anyhow!("No active thread"))?;

        let response = session.dap_client.step_in(thread_id).await?;

        // Форматируем ответ для AI
        Ok(format!("Stepped into. Current location: {:?}", response))
    }).await
}

/// Step over
#[tool]
pub async fn debug_next(
    session_id: String,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        let thread_id = session.current_thread_id?;
        session.dap_client.next(thread_id).await?;

        Ok("Stepped over to next line".to_string())
    }).await
}

/// Продолжить выполнение
#[tool]
pub async fn debug_continue(
    session_id: String,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        let thread_id = session.current_thread_id?;
        session.dap_client.continue_execution(thread_id).await?;

        Ok("Continuing execution...".to_string())
    }).await
}

/// Показать значение переменной
#[tool]
pub async fn debug_print(
    session_id: String,
    expression: String,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        let thread_id = session.current_thread_id?;

        // Получаем current frame
        let stack = session.dap_client.stack_trace(thread_id).await?;
        let frame_id = stack["stackFrames"][0]["id"].as_u64().unwrap() as u32;

        // Evaluate expression
        let response = session.dap_client.evaluate(&expression, frame_id).await?;

        let value = response["result"].as_str().unwrap_or("N/A");
        let type_ = response["type"].as_str().unwrap_or("unknown");

        Ok(format!("{} = {} (type: {})", expression, value, type_))
    }).await
}

/// Показать stack trace
#[tool]
pub async fn debug_backtrace(
    session_id: String,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        let thread_id = session.current_thread_id?;
        let response = session.dap_client.stack_trace(thread_id).await?;

        // Форматируем stack trace для читаемости
        let frames = response["stackFrames"].as_array().unwrap();
        let mut result = String::from("Stack trace:\n");

        for (i, frame) in frames.iter().enumerate() {
            let name = frame["name"].as_str().unwrap_or("??");
            let file = frame["source"]["path"].as_str().unwrap_or("??");
            let line = frame["line"].as_u64().unwrap_or(0);

            result.push_str(&format!("  #{} {} at {}:{}\n", i, name, file, line));
        }

        Ok(result)
    }).await
}

/// Установить conditional breakpoint
#[tool]
pub async fn set_conditional_breakpoint(
    session_id: String,
    file: String,
    line: u32,
    condition: String,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        session.dap_client.send_request("setBreakpoints", json!({
            "source": { "path": file },
            "breakpoints": [{
                "line": line,
                "condition": condition
            }]
        })).await?;

        Ok(format!("Conditional breakpoint set: {} if {}", file, condition))
    }).await
}

/// Установить watch expression
#[tool]
pub async fn debug_watch(
    session_id: String,
    expression: String,
    manager: Arc<SessionManager>,
) -> Result<String> {
    manager.with_session(&session_id, |session| {
        // DAP поддерживает watch через evaluate с контекстом "watch"
        // Сохраняем в сессии для повторной оценки на каждом stop
        Ok(format!("Watching: {}", expression))
    }).await
}
```

---

**Task 5: MCP Resources для debug info (1 день)**

Реализовать Resources для доступа к debug информации:

```rust
// mcp-debug-server/src/resources.rs

Resources:
- `debug://sessions` — список активных debug сессий
- `debug://session/{id}/breakpoints` — список breakpoints
- `debug://session/{id}/threads` — список threads
- `debug://session/{id}/variables` — текущие переменные в scope
- `debug://session/{id}/stack` — текущий stack trace
```

---

**Task 6: Event Handling (2 дня)**

Обработка DAP events (stopped, continued, breakpoint hit):

```rust
// mcp-debug-server/src/session.rs

impl DebugSession {
    /// Обработка DAP events в фоне
    pub async fn handle_events(&mut self) -> Result<()> {
        loop {
            let event = self.dap_client.read_event().await?;

            match event["event"].as_str() {
                Some("stopped") => {
                    let reason = event["body"]["reason"].as_str().unwrap();
                    let thread_id = event["body"]["threadId"].as_u64().unwrap() as u32;

                    self.current_thread_id = Some(thread_id);
                    self.state = SessionState::Stopped;

                    tracing::info!("🛑 Program stopped: {} (thread {})", reason, thread_id);

                    // Отправляем notification через MCP
                    // self.send_mcp_notification("debug/stopped", ...);
                }
                Some("continued") => {
                    self.state = SessionState::Running;
                    tracing::info!("▶️ Program continued");
                }
                Some("terminated") => {
                    self.state = SessionState::Terminated;
                    tracing::info!("🏁 Program terminated");
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
```

---

**Task 7: MCP Server Integration (2 дня)**

Собрать всё в MCP server:

```rust
// mcp-debug-server/src/main.rs
use rmcp::server::Server;
use rmcp::transport::StdioTransport;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let session_manager = Arc::new(SessionManager::new());

    // Создаём MCP server
    let server = Server::new("mcp-debug-server", "0.1.0");

    // Регистрируем tools
    server.add_tool(create_debug_session);
    server.add_tool(set_breakpoint);
    server.add_tool(debug_run);
    server.add_tool(debug_step);
    server.add_tool(debug_next);
    server.add_tool(debug_continue);
    server.add_tool(debug_print);
    server.add_tool(debug_backtrace);
    server.add_tool(set_conditional_breakpoint);
    server.add_tool(debug_watch);

    // Запускаем через stdio transport
    let transport = StdioTransport::new(
        tokio::io::stdin(),
        tokio::io::stdout()
    );

    server.serve(transport).await?;

    Ok(())
}
```

**Конфигурация для Claude Desktop:**
```json
// claude_desktop_config.json
{
  "mcpServers": {
    "debugger": {
      "command": "mcp-debug",
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

---

**Task 8: Интеграционные тесты (2 дня)**

```rust
// mcp-debug-server/tests/integration_test.rs

#[tokio::test]
async fn test_debug_session_lifecycle() {
    let manager = SessionManager::new();

    // 1. Создаём сессию
    let session_id = manager.create_session(
        "target/debug/bsl-lsp-server".to_string(),
        "codelldb".to_string(),
    ).await.unwrap();

    // 2. Устанавливаем breakpoint
    manager.with_session(&session_id, |session| {
        session.dap_client.set_breakpoint("src/main.rs", 10).await
    }).await.unwrap();

    // 3. Запускаем
    manager.with_session(&session_id, |session| {
        session.dap_client.launch(&session.binary_path, vec![]).await
    }).await.unwrap();

    // 4. Ждём остановки на breakpoint
    tokio::time::sleep(Duration::from_secs(1)).await;

    // 5. Проверяем stack trace
    let stack = manager.with_session(&session_id, |session| {
        session.dap_client.stack_trace(session.current_thread_id.unwrap()).await
    }).await.unwrap();

    assert!(stack["stackFrames"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_variable_inspection() {
    let manager = SessionManager::new();
    let session_id = setup_debug_session(&manager).await;

    // Устанавливаем breakpoint после присваивания
    manager.with_session(&session_id, |session| {
        session.dap_client.set_breakpoint("test.rs", 15).await
    }).await.unwrap();

    // Запускаем и ждём остановки
    run_and_wait(&manager, &session_id).await;

    // Проверяем переменную
    let value = manager.with_session(&session_id, |session| {
        let frame_id = get_current_frame(session).await?;
        session.dap_client.evaluate("my_variable", frame_id).await
    }).await.unwrap();

    assert_eq!(value["result"].as_str(), Some("42"));
}
```

---

**Результат Milestone 4.4:**

**MCP Tools доступные AI:**
- ✅ `create_debug_session` — создание debug сессии
- ✅ `set_breakpoint` — установка breakpoint
- ✅ `debug_run` — запуск программы
- ✅ `debug_step` — step into
- ✅ `debug_next` — step over
- ✅ `debug_continue` — продолжить выполнение
- ✅ `debug_print` — показать переменную
- ✅ `debug_backtrace` — stack trace
- ✅ `set_conditional_breakpoint` — условный breakpoint
- ✅ `debug_watch` — watch expression

**Поддерживаемые языки:**
- ✅ Rust (через CodeLLDB)
- ✅ C/C++ (через vscode-cpptools или CodeLLDB)
- ✅ Go (через delve DAP adapter)
- ✅ Python (через debugpy)
- ✅ Любой язык с DAP support

**Интеграция:**
- ✅ Claude Desktop (через MCP configuration)
- ✅ Claude Code (встроенная поддержка MCP)
- ✅ Cursor / Windsurf (если поддерживают MCP)

**Performance:**
- ✅ DAP protocol — бинарный, быстрый
- ✅ Async/await — нет блокировки
- ✅ Multiple sessions — параллельная отладка

**Зависимости:**
- ✅ CodeLLDB (уже установлен для Rust development)
- ✅ DAP protocol specification
- ✅ rmcp crate (Rust MCP SDK)

**Enables:**
- 🚀 AI-powered debugging для всех языков
- 🚀 Автоматическая отладка багов
- 🚀 Root cause analysis через AI

**Оценка времени:** 2-3 недели (14-21 дней)

**Сложность:** СРЕДНЯЯ
- DAP protocol хорошо документирован
- Есть примеры реализаций (vscode-debugadapter-node)
- MCP SDK упрощает server implementation

---

**Пример использования AI:**

```
Claude: "Отладим баг с hover на методах"

1. Claude вызывает: create_debug_session(binary="target/debug/bsl-lsp-server")
   → Session: abc123

2. Claude вызывает: set_breakpoint(session="abc123", file="ast_to_ir.rs", line=123)
   → Breakpoint 1 set

3. Claude вызывает: debug_run(session="abc123", args=[])
   → Program started, stopped at breakpoint

4. Claude вызывает: debug_print(session="abc123", expression="object_type")
   → object_type = "Массив<?>" (type: String)

5. Claude вызывает: debug_step(session="abc123")
   → Now at line 124

6. Claude вызывает: debug_backtrace(session="abc123")
   → Stack:
     #0 infer_expression_type at ast_to_ir.rs:124
     #1 convert_statement at ast_to_ir.rs:456

7. Claude анализирует и находит баг: "Ага! object_type содержит Generic параметры,
   нужно их убрать перед поиском в SignatureIndex"

8. Claude исправляет код и повторяет отладку
```

---

**Научная новизна:**

Это будет **первый** MCP server для interactive debugging! Открывает новые возможности:
- 🎓 Обучение программированию с AI mentor
- 🐛 Автоматический bug hunting
- 🔍 Root cause analysis
- 📊 Performance profiling через AI

**Применимость:**
- ✅ BSL Gradual Types (наш проект)
- ✅ Любые Rust проекты
- ✅ C/C++ проекты
- ✅ Любые языки с DAP support

---

## 📅 Timeline Summary

| Версия | Период | Длительность | Ключевые фичи |
|--------|--------|--------------|---------------|
| **1.0** (текущая) | Завершена | - | MVP: LSP, Валидация, VSCode Extension |
| **2.0** | Q1 2025 | 3 месяца | Tree-sitter, Flow-sensitive, Union/Generic Types |
| **3.0** | Q2 2025 | 3 месяца | Code Intelligence, Refactorings, Static Analysis |
| **3.5** | Q2 2025 | 4-6 недель | LLVM-inspired Analysis Pipeline, Null Safety, Dead Code, Control Flow |
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

### Версия 3.5 — LLVM-inspired Static Analysis
- ✅ Обнаружение 80% потенциальных NullPointerException
- ✅ Сокращение runtime ошибок на 40-50%
- ✅ Улучшение качества кода на 20-30%
- ✅ Положительные отзывы от enterprise пользователей

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
