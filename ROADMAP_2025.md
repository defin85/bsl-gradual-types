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
5. [🌐 Версия 4.0 — Collaboration & Ecosystem](#-версия-40--collaboration--ecosystem-q3-q4-2025-6-месяцев)
6. [📅 Timeline Summary](#-timeline-summary)

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

**Итого завершено:** 19 Milestones
**Прогресс Версии 2.0:** ~95% завершено (19/20 Milestones)

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
- 🔗 Rust MCP SDK: https://github.com/modelcontextprotocol/rust-sdk
- 🔗 Примеры MCP серверов: https://github.com/rust-mcp-stack/rust-mcp-filesystem

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
rmcp = { version = "0.8.0", features = ["server", "macros"] }
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

### 🔧 Milestone 3.5: Flow-Sensitive Analysis ✅

**Статус:** ✅ ЗАВЕРШЁН
**Дата:** 2025-11-08
**Время реализации:** 5 дней (architect → coder → reviewer → tester → coder → reviewer)
**Приоритет:** 🔴 КРИТИЧНЫЙ — исправляет баг с hover на вызовах методов

**Проблема:**

Текущая реализация hover теряет тип переменной при обращении к её методам. Пример из `test_hover_milestone_2_11.bsl`:

```bsl
ТаблицаЗначенійТип = Новый ТаблицаЗначений;  // Строка 25
Кол = ТаблицаЗначенійТип.НеСуществующийМетод();  // Строка 26
```

**Актуальное поведение:**
- Строка 25: hover на `ТаблицаЗначенійТип` → ✅ показывает `Тип: ТаблицаЗначений` с методами
- Строка 26: hover на `ТаблицаЗначенійТип` → ❌ показывает `Тип: Неопределено`

**Причина бага:**

В `backend/src/application/ast_to_ir.rs:522`:
```rust
let object_type = self.infer_expression_type(&object);  // Возвращает "ТаблицаЗначений" (тип)

let node = SemanticNode {
    kind: SemanticNodeKind::MemberAccess {
        object_type,  // ← Хранится ТИП, а не ИМЯ переменной!
        member_name: property,
        is_method: true,
    },
    ...
};
```

Потом в `shared/src/ir/mod.rs:491-495`:
```rust
SemanticNodeKind::MemberAccess { object_type, .. } => {
    // object_type может быть именем переменной или типом
    // Попробуем найти как переменную в scope
    object_type.clone()  // ← Ищем переменную "ТаблицаЗначений" вместо "ТаблицаЗначенійТип"
}
```

**Решение:**

Хранить в `MemberAccess` **и имя переменной, и её тип**:
```rust
SemanticNodeKind::MemberAccess {
    object_name: String,   // ← "ТаблицаЗначенійТип" (имя переменной)
    object_type: String,   // ← "ТаблицаЗначений" (тип)
    member_name: String,
    is_method: bool,
}
```

#### Задачи:

**Task 1: Рефакторинг SemanticNodeKind::MemberAccess (1 день)**

Обновить структуру в `shared/src/ir/mod.rs`:

```rust
pub enum SemanticNodeKind {
    // ... другие варианты

    MemberAccess {
        object_name: String,   // ✅ НОВОЕ: имя переменной для резолюции
        object_type: String,   // Тип переменной
        member_name: String,
        is_method: bool,
    },

    FunctionCall {
        function_name: String,
        object_name: Option<String>,  // ✅ НОВОЕ: имя объекта для методов
        object_type: Option<String>,  // Тип объекта (если вызов метода)
        argument_types: Vec<String>,
    },

    // ... другие варианты
}
```

**Обновить все места использования:**
- `backend/src/application/ast_to_ir.rs` — добавить извлечение `object_name`
- `shared/src/ir/mod.rs:find_variable_at_position()` — использовать `object_name`
- `backend/src/bin/lsp_server.rs` — обновить обработку hover

---

**Task 2: Извлечение object_name в ast_to_ir (1-2 дня)**

Обновить `backend/src/application/ast_to_ir.rs`:

```rust
// Обработка PropertyAccess
if let Expression::PropertyAccess { object, property, .. } = expression {
    // ✅ НОВОЕ: Извлекаем ИМЯ переменной
    let object_name = if let Expression::Identifier { name, .. } = &**object {
        Some(name.clone())
    } else {
        None
    };

    // Инферим ТИП
    let object_type = self.infer_expression_type(&object);

    let node = SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_name: object_name.unwrap_or_else(|| object_type.clone()),
            object_type,
            member_name: property,
            is_method: true,
        },
        span,
        scope_id: self.current_scope,
    };

    self.nodes.push(node);
    return Ok(Some(self.nodes.len() - 1));
}
```

**Аналогично для FunctionCall:**
```rust
Expression::Call { function, args, .. } => {
    if let Expression::PropertyAccess { object, property, .. } = &**function {
        let object_name = if let Expression::Identifier { name, .. } = &**object {
            Some(name.clone())
        } else {
            None
        };

        let object_type = self.infer_expression_type(&object);

        let node = SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name: property.clone(),
                object_name,
                object_type: Some(object_type),
                argument_types,
            },
            span,
            scope_id: self.current_scope,
        };

        self.nodes.push(node);
        return Ok(Some(self.nodes.len() - 1));
    }
}
```

---

**Task 3: Обновить find_variable_at_position (1 день)**

Обновить `shared/src/ir/mod.rs`:

```rust
pub fn find_variable_at_position(&self, line: u32, column: u32) -> Option<(String, TypeHint)> {
    let node = self.find_node_at_position(line, column)?;
    let scope_id = node.scope_id;

    let var_name = match &node.kind {
        SemanticNodeKind::Assignment { variable, .. } => variable.clone(),

        // ✅ ИСПРАВЛЕНО: Используем object_name вместо object_type
        SemanticNodeKind::MemberAccess { object_name, .. } => {
            object_name.clone()
        }

        SemanticNodeKind::VariableDeclaration { name, .. } => name.clone(),

        // ✅ ИСПРАВЛЕНО: Используем object_name вместо object_type
        SemanticNodeKind::FunctionCall { object_name: Some(obj_name), .. } => {
            obj_name.clone()
        }
        SemanticNodeKind::FunctionCall { object_name: None, .. } => {
            return None;
        }

        _ => return None,
    };

    // Ищем переменную по ИМЕНИ (не по типу!)
    let (type_hint, _span) = self.resolve_variable(&var_name, scope_id)?;

    Some((var_name, type_hint))
}
```

---

**Task 4: Обновить обработку hover в LSP (1 день)**

Обновить `backend/src/bin/lsp_server.rs`:

```rust
async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    // ... получение текста файла

    // ✅ Используем обновлённую логику
    if let Some((var_name, type_hint)) = ir_program.find_variable_at_position(line, column) {
        // var_name теперь корректно содержит имя переменной,
        // даже если hover на вызове метода
        return Ok(Some(self.format_variable_hover(&var_name, &type_hint)));
    }

    Ok(None)
}
```

---

**Task 5: Тесты для Flow-Sensitive Analysis (1-2 дня)**

Создать `backend/tests/flow_sensitive_hover_test.rs`:

```rust
#[tokio::test]
async fn test_hover_on_method_call_preserves_type() {
    let code = r#"
Функция Тест()
    ТаблицаТип = Новый ТаблицаЗначений;
    Кол = ТаблицаТип.Количество();
    // ↑ hover на "ТаблицаТип" должен показывать "ТаблицаЗначений", а не "Неопределено"
КонецФункции
    "#;

    let service = TypeSystemService::new(...);
    let ir = service.parse_bsl_code(code).await.unwrap();

    // Позиция на "ТаблицаТип" в строке 4
    let (var_name, type_hint) = ir.find_variable_at_position(4, 10).unwrap();

    assert_eq!(var_name, "ТаблицаТип");
    assert!(matches!(type_hint, TypeHint::Inferred(t) if t == "ТаблицаЗначений"));
}

#[tokio::test]
async fn test_hover_on_nonexistent_method() {
    let code = r#"
Функция Тест()
    Массив = Новый Массив;
    Результат = Массив.НеСуществующийМетод();
    // ↑ hover должен показывать тип "Массив" + error о несуществующем методе
КонецФункции
    "#;

    let service = TypeSystemService::new(...);
    let ir = service.parse_bsl_code(code).await.unwrap();

    let (var_name, type_hint) = ir.find_variable_at_position(4, 17).unwrap();

    assert_eq!(var_name, "Массив");
    assert!(matches!(type_hint, TypeHint::Inferred(t) if t == "Массив"));
}
```

---

**Результат Milestone 3.5:**
- ✅ Hover на вызовах методов показывает корректный тип переменной
- ✅ `SemanticNodeKind::MemberAccess` хранит и `object_name`, и `object_type`
- ✅ `SemanticNodeKind::FunctionCall` хранит и `object_name`, и `object_type`
- ✅ `find_variable_at_position()` корректно резолвит переменные в вызовах методов
- ✅ Тест из `test_hover_milestone_2_11.bsl:26` проходит
- ✅ 2+ новых интеграционных теста для flow-sensitive анализа

**Зависимости:**
- ✅ Milestone 2.8 (Semantic IR Layer)
- ✅ Milestone 2.9 (Inline Scope Analysis)
- ✅ Milestone 2.11 (Tree-Sitter Span Extraction)

**Оценка времени:** 5-7 дней

---

### 🎨 Milestone 3.6: Enhanced UX (Hover + Diagnostics) (15-18 дней)

**Приоритет:** 🟡 СРЕДНИЙ — значительное улучшение UX, но не критично для функциональности

**Проблема:**

**Hover:** Текущий hover недостаточно гибкий и информативный:
- ❌ Нет настроек — пользователь не может выбрать уровень детализации
- ❌ Не показываются фасеты (Manager vs Object vs Reference)
- ❌ Generic типы не объясняются (`Массив<Строка>` выглядит как обычный тип)
- ❌ Нет ссылок на platform documentation
- ❌ Методы с большим количеством параметров плохо читаются

**Diagnostics:** Сообщения об ошибках недостаточно информативны:
- ❌ Не указывается имя переменной — непонятно где искать проблему
- ❌ Нет подсказок для исправления ошибки
- ❌ Нет fuzzy matching для опечаток в именах методов
- ❌ Ошибки параметров не показывают имя переменной-параметра
- ❌ Один уровень детализации — нельзя настроить

**Исследование:**

Проведён детальный анализ hover в современных IDE (Rust Analyzer, TypeScript, Pylance, JetBrains IDEA).
**Документ:** `docs/research/hover-best-practices-2025.md` (67 страниц)

**Ключевые находки:**
- 🏆 **Rust Analyzer** — золотой стандарт (настройки, интерактивность, expandable sections)
- ⚙️ **TypeScript** — фокус на кастомизации (уровни детализации)
- 📖 **Pylance** — rich formatting с docstrings
- 💡 **Best Practice:** три уровня детализации (compact → full → detailed)

**Решение:**

Реализовать **комплексное улучшение UX** с едиными принципами настраиваемости для hover и diagnostics:
- **Hover:** три уровня детализации, фасеты, Generic типы, ссылки на документацию
- **Diagnostics:** контекст переменных, умные подсказки, fuzzy matching, три уровня детализации
- **Общая инфраструктура:** единый `DetailLevel` enum, консистентные настройки, переиспользование компонентов

#### Задачи:

**Phase 1: Settings & Detail Levels (5 дней)**

**Task 1.1: VSCode Extension Settings (1 день)**

Добавить конфигурацию в `vscode-extension/package.json`:

```json
"bsl.hover.detailLevel": {
  "type": "string",
  "enum": ["compact", "full", "detailed"],
  "default": "full",
  "enumDescriptions": [
    "Только тип переменной",
    "Тип + методы (до max)",
    "Тип + методы + свойства + фасеты + документация"
  ]
},
"bsl.hover.maxMethods": {
  "type": "number",
  "default": 10,
  "minimum": 1,
  "maximum": 50
},
"bsl.hover.maxProperties": {
  "type": "number",
  "default": 5
},
"bsl.hover.showCertainty": {
  "type": "boolean",
  "default": true,
  "description": "Показывать уверенность в типе (🟢🟡⚪)"
}
```

Передавать настройки в LSP через `workspace/didChangeConfiguration`.

---

**Task 1.2: LSP Server Configuration Handling (1 день)**

Обновить `backend/src/bin/lsp_server.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
struct BslHoverSettings {
    #[serde(rename = "detailLevel")]
    detail_level: String,  // "compact" | "full" | "detailed"

    #[serde(rename = "maxMethods")]
    max_methods: usize,

    #[serde(rename = "maxProperties")]
    max_properties: usize,

    #[serde(rename = "showCertainty")]
    show_certainty: bool,
}

// Handler для workspace/didChangeConfiguration
async fn on_did_change_configuration(
    params: DidChangeConfigurationParams,
    state: Arc<ServerState>,
) {
    // Обновить hover_settings в state
}
```

---

**Task 1.3: HoverFormatter с DetailLevel (2 дня)**

Обновить `backend/src/helpers/hover_formatter.rs`:

```rust
#[derive(Debug, Clone, Copy)]
pub enum DetailLevel {
    /// Только тип переменной
    Compact,

    /// Тип + методы (до max_methods)
    Full,

    /// Тип + методы + свойства + фасеты + документация
    Detailed,
}

pub struct HoverFormatConfig {
    pub max_methods: usize,
    pub max_properties: usize,
    pub detail_level: DetailLevel,
    pub show_certainty: bool,
    // ... остальное
}

// Обновить format_variable с учётом detail_level
pub fn format_variable(&self, name: &str, resolution: &TypeResolution) -> String {
    match self.config.detail_level {
        DetailLevel::Compact => {
            // Только тип
        }
        DetailLevel::Full => {
            // Тип + методы
        }
        DetailLevel::Detailed => {
            // Тип + методы + свойства + фасеты
        }
    }
}
```

---

**Task 1.4: Multiline Formatting для методов (1 день)**

Улучшить форматирование методов с 4+ параметрами:

```markdown
**До:**
• Вставить(Индекс: Число, Строка1: СтрокаТаблицыЗначений, Строка2: СтрокаТаблицыЗначений, ...) → СтрокаТаблицыЗначений

**После:**
• Вставить(
    Индекс: Число,
    Строка1: СтрокаТаблицыЗначений,
    Строка2: СтрокаТаблицыЗначений,
    ...
  ) → СтрокаТаблицыЗначений
```

---

**Phase 2: Facets, Generics, Documentation (7 дней)**

**Task 2.1: Фасеты в hover (2 дня)**

Добавить отображение фасетов в `DetailLevel::Detailed`:

```markdown
Переменная: НоменклатураСсылка
Тип: СправочникСсылка.Номенклатура
**Фасет:** Reference (ссылка на элемент)

💡 **Доступные фасеты:** Manager, Object, Reference, Selection
```

Реализация в `backend/src/helpers/hover_formatter.rs`:

```rust
fn add_facet_info(mut self, resolution: &TypeResolution) -> Self {
    if let Some(active_facet) = &resolution.active_facet {
        let facet_description = match active_facet {
            FacetKind::Manager => "менеджер объекта",
            FacetKind::Object => "объект с данными",
            FacetKind::Reference => "ссылка на элемент",
            FacetKind::Selection => "выборка элементов",
            FacetKind::List => "список значений",
        };

        self.sections.push(format!(
            "**Фасет:** {:?} ({})",
            active_facet, facet_description
        ));
    }

    self
}
```

---

**Task 2.2: Generic типы (2 дня)**

Добавить пояснения для Generic типов:

```markdown
Переменная: СписокИмен
Тип: Массив<Строка>

💡 **Generic тип:**
• Базовый тип: Массив
• Параметры: Строка
```

Реализация:

```rust
fn add_generic_info(mut self, resolution: &TypeResolution) -> Self {
    if let ResolutionResult::Generic(generic) = &resolution.result {
        let params_str = generic.type_params
            .join(", ");

        self.sections.push(format!(
            "💡 **Generic тип:**\n• Базовый тип: {}\n• Параметры: {}",
            generic.base_type, params_str
        ));
    }

    self
}
```

---

**Task 2.3: Ссылки на документацию (3 дня)**

Добавить ссылки на platform documentation:

```markdown
📖 **Документация:**
• [Синтакс Помощник: Массив](file:///C:/examples/syntax_helper/Array.html)
• [1С Platform Docs](https://docs.1c.ru/search?q=Массив)
```

Реализация:

```rust
fn add_documentation_links(mut self, resolution: &TypeResolution) -> Self {
    if let Some(type_name) = self.get_platform_type_name(resolution) {
        let mut links = Vec::new();

        // Ссылка на локальный syntax_helper
        if let Some(path) = &self.config.syntax_helper_path {
            let html_path = path.join(format!("{}.html", type_name));
            if html_path.exists() {
                links.push(format!(
                    "[Синтакс Помощник: {}](file://{})",
                    type_name,
                    html_path.display()
                ));
            }
        }

        // Ссылка на онлайн документацию
        links.push(format!(
            "[1С Platform Docs](https://docs.1c.ru/search?q={})",
            type_name
        ));

        if !links.is_empty() {
            self.sections.push(format!(
                "📖 **Документация:**\n{}",
                links.iter().map(|l| format!("• {}", l)).collect::<Vec<_>>().join("\n")
            ));
        }
    }

    self
}
```

---

**Phase 3: Enhanced Diagnostic Messages (5-6 дней)**

**Task 3.1: Обогащение контекста ошибок (2 дня)**

Добавить имя переменной в сообщения об ошибках для лучшего контекста.

**Обновить `shared/src/domain/validators.rs`:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TypeErrorKind {
    NonExistentMethod {
        object_type: String,
        method_name: String,
        variable_name: Option<String>,  // ← НОВОЕ: контекст переменной
    },
    IncorrectParameterType {
        method_name: String,
        param_index: usize,
        expected: String,
        actual: String,
        variable_name: Option<String>,  // ← НОВОЕ
        param_variable_name: Option<String>,  // ← Имя переменной-параметра
    },
    NonExistentProperty {
        object_type: String,
        property_name: String,
        variable_name: Option<String>,  // ← НОВОЕ
    },
    SimpleTypeAsCollection {
        type_name: String,
        operation: String,
        variable_name: Option<String>,  // ← НОВОЕ
    },
}
```

**Обновить вызовы в `type_system_service.rs` и `semantic_validation_visitor.rs`:**
- Передавать `object_name` из `FunctionCall` узла в `variable_name`
- Fallback на `None` для edge cases (литералы, прямые вызовы)

---

**Task 3.2: Три уровня детализации diagnostic messages (2 дня)**

Переиспользовать `DetailLevel` enum из hover для консистентности.

**Обновить `shared/src/domain/validators.rs`:**

```rust
impl TypeErrorKind {
    pub fn to_diagnostic(&self, span: Span, detail_level: DetailLevel) -> TypeDiagnostic {
        let message = self.format_message(detail_level);

        TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message,
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }
    }

    fn format_message(&self, detail_level: DetailLevel) -> String {
        match detail_level {
            DetailLevel::Brief => self.format_brief(),
            DetailLevel::Standard => self.format_standard(),
            DetailLevel::Detailed => self.format_detailed(),
        }
    }
}
```

**Форматы для NonExistentMethod:**

```rust
fn format_brief(&self) -> String {
    // Brief (по умолчанию для inline)
    format!("Метод '{}' не существует для типа '{}'", method_name, object_type)
}

fn format_standard(&self) -> String {
    // Standard (с именем переменной если есть)
    if let Some(var) = variable_name {
        format!(
            "Метод '{}' не существует для переменной '{}' типа '{}'",
            method_name, var, object_type
        )
    } else {
        self.format_brief()  // Fallback
    }
}

fn format_detailed(&self) -> String {
    // Detailed (+ подсказки, см. Task 3.3)
    let base = self.format_standard();
    let hints = self.generate_hints();  // См. Task 3.3

    if !hints.is_empty() {
        format!("{}\n\n{}", base, hints)
    } else {
        base
    }
}
```

**Примеры сообщений:**

| Level | Пример |
|-------|--------|
| Brief | `Метод 'Метод' не существует для типа 'ТаблицаЗначений'` |
| Standard | `Метод 'Метод' не существует для переменной 'ТЗ' типа 'ТаблицаЗначений'` |
| Detailed | `Метод 'Метод' не существует для переменной 'ТЗ' типа 'ТаблицаЗначений'`<br><br>`Подсказка: Доступные методы: Добавить(), Количество(), Найти()...` |

---

**Task 3.3: Умные подсказки для Detailed level (1-2 дня)**

Генерировать контекстные подсказки для исправления ошибок.

**Реализовать в `shared/src/domain/validators.rs`:**

```rust
impl TypeErrorKind {
    fn generate_hints(&self, metadata_lookup: &TypeMetadataLookup) -> String {
        match self {
            NonExistentMethod { object_type, method_name, .. } => {
                let methods = metadata_lookup.get_methods_for_type(object_type);

                // 1. Fuzzy matching - поиск похожих имён
                let similar = fuzzy_match_methods(method_name, &methods, 0.7);

                if !similar.is_empty() {
                    let suggestions = similar.iter()
                        .take(3)
                        .map(|m| format!("{}()", m.name))
                        .collect::<Vec<_>>()
                        .join(", ");

                    return format!(
                        "💡 Подсказка: Возможно, вы имели в виду: {}?",
                        suggestions
                    );
                }

                // 2. Показать популярные методы
                let top_methods = methods.iter()
                    .take(5)
                    .map(|m| format!("• {}()", m.name))
                    .collect::<Vec<_>>()
                    .join("\n");

                format!(
                    "💡 Доступные методы типа '{}':\n{}",
                    object_type, top_methods
                )
            },

            IncorrectParameterType { expected, actual, .. } => {
                format!(
                    "💡 Подсказка: Ожидается {}, но передано {}. Преобразуйте тип или используйте другую переменную.",
                    expected, actual
                )
            },

            // ... другие типы ошибок
        }
    }
}
```

**Примеры hints:**

```markdown
# Пример 1: Похожее имя найдено
Метод 'Колво' не существует для переменной 'ТЗ' типа 'ТаблицаЗначений'

💡 Подсказка: Возможно, вы имели в виду: Количество()?

# Пример 2: Нет похожих, показываем доступные
Метод 'ХХХ' не существует для переменной 'М' типа 'Массив'

💡 Доступные методы типа 'Массив':
• Добавить()
• Количество()
• Получить()
• Удалить()
• Очистить()

# Пример 3: Неправильный тип параметра
Некорректный параметр #1 для метода 'Вставить' переменной 'ТЗ': ожидается Число, получена переменная 'индекс' типа Строка

💡 Подсказка: Преобразуйте строку в число через функцию Число(индекс) или используйте числовую переменную.
```

---

**Task 3.4: Интеграция настроек diagnostics с settings.json (1 день)**

Переиспользовать общий `DetailLevel` enum для hover и diagnostics.

**Добавить в `vscode-extension/package.json`:**

```json
"bsl.diagnostics.detailLevel": {
  "type": "string",
  "enum": ["brief", "standard", "detailed"],
  "default": "standard",
  "enumDescriptions": [
    "Краткие сообщения (только тип)",
    "Стандартные (тип + переменная)",
    "Детальные (тип + переменная + подсказки)"
  ],
  "description": "Уровень детализации сообщений об ошибках"
},
"bsl.diagnostics.showHints": {
  "type": "boolean",
  "default": true,
  "description": "Показывать умные подсказки для исправления ошибок"
}
```

**Обновить LSP Server для передачи настроек в TypeValidator:**

```rust
// backend/src/bin/lsp_server.rs
async fn on_did_change_configuration(params: DidChangeConfigurationParams) {
    let settings = params.settings.get("bsl").unwrap_or_default();

    let diagnostic_detail = settings
        .get("diagnostics")
        .and_then(|d| d.get("detailLevel"))
        .and_then(|v| v.as_str())
        .unwrap_or("standard");

    // Обновить в state
    state.diagnostic_settings.update(diagnostic_detail);
}
```

---

**Task 3.5: Edge cases обработка (1 день)**

Обработать все сложные случаи:

**1. Литералы:**
```bsl
"строка".НесуществующийМетод();
```
**Сообщение (Standard):**
```
Метод 'НесуществующийМетод' не существует для литерала строки типа 'Строка'
```

**2. Прямые вызовы:**
```bsl
Справочники.Контрагенты.МетодКоторогоНет();
```
**Сообщение (Standard):**
```
Метод 'МетодКоторогоНет' не существует для выражения типа 'СправочникМенеджер.Контрагенты'
```

**3. Вложенные цепочки:**
```bsl
массив.Получить(0).НесуществующийМетод();
```
**Сообщение (Standard):**
```
Метод 'НесуществующийМетод' не существует для результата вызова 'массив.Получить(0)' типа 'Произвольный'
```

**Реализация:**

```rust
fn get_object_description(object_name: &Option<String>, object_type: &str) -> String {
    match object_name {
        Some(name) if is_literal(name) => {
            format!("литерала {} типа", infer_literal_kind(name))
        },
        Some(name) if is_expression(name) => {
            format!("выражения")
        },
        Some(name) => {
            format!("переменной '{}'", name)
        },
        None => {
            format!("объекта")
        }
    }
}
```

---

**Результат Milestone 3.6:**

**Phase 1:**
- ✅ VSCode settings UI для hover кастомизации
- ✅ LSP Server принимает и обрабатывает настройки
- ✅ Три уровня детализации (compact → full → detailed)
- ✅ Multiline форматирование для методов с 4+ параметрами
- ✅ Настройки применяются динамически без перезапуска

**Phase 2:**
- ✅ Фасеты отображаются в hover с пояснениями на русском
- ✅ Generic типы объясняются понятно (`Массив<Строка>`)
- ✅ Ссылки на локальный syntax_helper и онлайн документацию
- ✅ Hover информативен и кастомизируем

**Phase 3:**
- ✅ Diagnostic messages с именами переменных (контекст)
- ✅ Три уровня детализации (Brief/Standard/Detailed)
- ✅ Умные подсказки для Detailed level (fuzzy matching, доступные методы)
- ✅ Обработка edge cases (литералы, выражения, цепочки вызовов)
- ✅ Настройки через settings.json (консистентность с hover)
- ✅ Информативные сообщения помогают быстро найти и исправить ошибку

**Зависимости:**
- ✅ Milestone 2.9 (Inline Scope Analysis) — hover уже работает
- ✅ Milestone 2.11 (Span Extraction) — координаты корректны
- ✅ Milestone 3.7 (Semantic Diagnostics MVP) — базовые diagnostics работают
- 📄 Исследование hover best practices (завершено)
- 📄 Исследование diagnostic messages индустрии (TypeScript, Rust, Python, C#)

**Оценка времени:** 15-18 дней (5 дней Phase 1 + 7 дней Phase 2 + 5-6 дней Phase 3)

**Пример итогового hover (DetailLevel::Detailed):**

```markdown
Переменная: ТаблицаЗначенійТип
Тип: ТаблицаЗначений
Уверенность: 🟢 Known (100%)
**Фасет:** Object (объект с данными)

💡 **Доступные фасеты:** Manager, Object

Методы (показано 10 из 19):
• Вставить(
    Индекс: Число
  ) → СтрокаТаблицыЗначений
• Добавить() → СтрокаТаблицыЗначений
• Количество() → Число
... и ещё 16 методов

Свойства (показано 2 из 2):
• Индексы: КоллекцияИндексов
• Колонки: КоллекцияКолонок

📖 **Документация:**
• [Синтакс Помощник: ТаблицаЗначений](file:///C:/examples/syntax_helper/ValueTable.html)
• [1С Platform Docs](https://docs.1c.ru/search?q=ТаблицаЗначений)
```

**Пример итоговых diagnostic messages (Phase 3):**

```markdown
# Brief (по умолчанию для inline подсветки):
Метод 'НеСуществует' не существует для типа 'Массив'

# Standard (больше контекста):
Метод 'НеСуществует' не существует для переменной 'списокИмен' типа 'Массив'

# Detailed (максимум помощи):
Метод 'НеСуществует' не существует для переменной 'списокИмен' типа 'Массив'

💡 Подсказка: Возможно, вы имели в виду: Найти(), НайтиЗначение()?

Доступные методы типа 'Массив':
• Добавить(Значение?) → void
• Количество() → Число
• Найти(Значение) → void
• Получить(Индекс: Число) → Произвольный
• Удалить(Индекс: Число) → void
```

---

### 🚨 Milestone 3.7: Semantic Diagnostics MVP ✅

**Приоритет:** 🔴 КРИТИЧНЫЙ — валидаторы готовы, нужна только интеграция в LSP

**Статус:** ✅ COMPLETED

**Дата завершения:** 2025-11-XX (реализовано ранее)

**Проблема:**

TypeValidator уже реализован и работает в Web API, но **НЕ используется в LSP**. Разработчики 1С не видят ошибки в редакторе:

```bsl
МассивДанных = Новый Массив;
МассивДанных.НесуществующийМетод();  // ❌ LSP НЕ показывает ошибку
Число = 42;
Число.Добавить(1);  // ❌ LSP НЕ показывает ошибку (примитив как коллекция)
```

**Текущее состояние:**

✅ **Что УЖЕ работает (Web API):**
- `POST /api/validate` — валидация методов/свойств
- `TypeValidator` с 4 типами валидаций
- 3927 типов платформы в TypeMetadataLookup
- 3 integration теста проходят

❌ **Что НЕ работает (LSP):**
- LSP показывает только **syntax errors** (Milestone 2.18)
- Semantic errors НЕ публикуются в `textDocument/publishDiagnostics`
- VSCode НЕ показывает красные волнистые линии для semantic ошибок

**Исследование:**

Проведён анализ валидаций (см. architect отчёт). Рекомендован **Уровень 1 (MVP)**: 3 критичные валидации, покрывающие ~70% типовых ошибок (Balyuk & Popova, 2021).

**Решение:**

Интегрировать TypeValidator в LSP lifecycle (`did_open`, `did_change`) для публикации semantic diagnostics.

#### Задачи:

**Task 1: SemanticValidationVisitor (1-2 дня)**

**Файл:** Создать `backend/src/application/semantic_validation_visitor.rs`

Visitor для обхода SemanticProgram и сбора semantic errors:

```rust
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::ir::SemanticProgram;

/// Visitor для сбора семантических ошибок из IR
pub struct SemanticValidationVisitor<'a> {
    validator: &'a TypeValidator<'a>,
}

impl<'a> SemanticValidationVisitor<'a> {
    pub fn new(validator: &'a TypeValidator<'a>) -> Self {
        Self { validator }
    }

    /// Обход программы и сбор ошибок
    pub fn collect_errors(&self, program: &SemanticProgram) -> Vec<TypeDiagnostic> {
        let mut errors = Vec::new();

        for node in &program.nodes {
            self.visit_node(node, &mut errors);
        }

        errors
    }

    fn visit_node(&self, node: &SemanticNode, errors: &mut Vec<TypeDiagnostic>) {
        match &node.kind {
            // Проверка вызова метода
            SemanticNodeKind::FunctionCall {
                function_name,
                object_type: Some(obj_type),
                object_name: Some(_),
                ..
            } => {
                // Резолвим тип объекта
                let resolution = TypeResolution::simple(obj_type.clone());

                // Проверяем существование метода
                if let Some(error) = self.validator.validate_method_exists(&resolution, function_name) {
                    errors.push(error.to_diagnostic(node.span.start_line, node.span.start_column));
                }
            }

            // Проверка доступа к свойству
            SemanticNodeKind::MemberAccess {
                object_type,
                member_name,
                is_method: false,  // Свойство, не метод
                ..
            } => {
                let resolution = TypeResolution::simple(object_type.clone());

                if let Some(error) = self.validator.validate_property_exists(&resolution, member_name) {
                    errors.push(error.to_diagnostic(node.span.start_line, node.span.start_column));
                }
            }

            // Другие узлы
            _ => {}
        }
    }
}
```

---

**Task 2: Интеграция в TypeSystemService (1 день)**

**Файл:** `backend/src/application/type_system_service.rs`

Добавить метод для semantic validation:

```rust
/// Валидация семантических ошибок (Milestone 3.7 MVP)
///
/// Проверяет:
/// - Несуществующие методы
/// - Несуществующие свойства
/// - Примитивы как коллекции
pub async fn validate_semantics(&self, code: &str) -> Result<Vec<TypeDiagnostic>> {
    // 1. Парсим код → SemanticProgram (IR)
    let parse_result = self.parser_coordinator.parse(code)?;
    let ir_program = AstToIrConverter::convert(
        parse_result.program,
        code.to_string(),
        "temp.bsl".to_string(),
        self.analysis_engine.get_repository(),
    )?;

    // 2. Создаём TypeValidator
    let validator = TypeValidator::new(&self.metadata_lookup);

    // 3. Обходим IR и собираем ошибки
    let visitor = SemanticValidationVisitor::new(&validator);
    let errors = visitor.collect_errors(&ir_program);

    Ok(errors)
}
```

---

**Task 3: Интеграция в LSP Server (1 день)**

**Файл:** `backend/src/bin/lsp_server.rs`

**Обновить `did_open()` (строки 708-746):**

```rust
async fn did_open(&self, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    let text = params.text_document.text.clone();
    let version = params.text_document.version;

    let mut diagnostics = Vec::new();

    // 1. Syntax validation (уже есть)
    if let Some(type_service) = self.get_type_service() {
        match type_service.parse_and_validate(&text) {
            Ok(syntax_errors) => {
                diagnostics.extend(self.syntax_errors_to_diagnostics(&syntax_errors));
            }
            Err(e) => { /* ... */ }
        }

        // 2. ✅ НОВОЕ: Semantic validation
        match type_service.validate_semantics(&text).await {
            Ok(semantic_errors) => {
                for error in semantic_errors {
                    diagnostics.push(self.semantic_error_to_diagnostic(&error));
                }
            }
            Err(e) => {
                warn!("Semantic validation failed: {}", e);
            }
        }
    }

    // 3. Публикуем ВСЕ диагностики (syntax + semantic)
    self.client
        .publish_diagnostics(uri.clone(), diagnostics, Some(version))
        .await;
}
```

**Добавить метод конвертации:**

```rust
/// Конвертировать TypeDiagnostic → LSP Diagnostic
fn semantic_error_to_diagnostic(&self, error: &TypeDiagnostic) -> Diagnostic {
    // Создаём range для подчёркивания
    let start_pos = Position::new(error.line, error.column);
    let end_pos = Position::new(error.line, error.column + 15);  // TODO: точная длина токена

    Diagnostic {
        range: Range::new(start_pos, end_pos),
        severity: Some(match error.severity {
            DiagnosticSeverity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
            DiagnosticSeverity::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
            _ => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
        }),
        message: error.message.clone(),
        source: Some("bsl-semantic".to_string()),  // ✅ Отличается от "bsl-syntax"
        ..Default::default()
    }
}
```

**Аналогично обновить `did_change()`** (строки 826+)

---

**Task 4: Тестирование (1 день)**

**Файл:** Создать `backend/tests/semantic_diagnostics_lsp_test.rs`

**Integration тесты:**

```rust
#[tokio::test]
async fn test_lsp_shows_nonexistent_method_error() {
    let lsp_server = create_test_lsp_server().await;

    let code = r#"
Функция Тест()
    МассивДанных = Новый Массив;
    МассивДанных.НесуществующийМетод();
КонецФункции
    "#;

    // Открываем документ
    let diagnostics = lsp_server.open_and_get_diagnostics(code).await;

    // Проверяем, что есть semantic error
    let semantic_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.source == Some("bsl-semantic".to_string()))
        .collect();

    assert!(
        !semantic_errors.is_empty(),
        "❌ LSP должен показать semantic error для несуществующего метода"
    );

    let error = &semantic_errors[0];
    assert!(error.message.contains("НесуществующийМетод"));
    assert!(error.message.contains("не существует"));
}

#[tokio::test]
async fn test_lsp_shows_primitive_as_collection_error() {
    let code = r#"
Функция Тест()
    Число = 42;
    Число.Добавить(1);
КонецФункции
    "#;

    let diagnostics = lsp_server.open_and_get_diagnostics(code).await;

    let semantic_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.source == Some("bsl-semantic".to_string()))
        .collect();

    assert!(!semantic_errors.is_empty());
    let error = &semantic_errors[0];
    assert!(error.message.contains("Число") || error.message.contains("примитив"));
}

#[tokio::test]
async fn test_lsp_shows_nonexistent_property_error() {
    let code = r#"
Функция Тест()
    Таблица = Новый ТаблицаЗначений;
    Значение = Таблица.НесуществующееСвойство;
КонецФункции
    "#;

    let diagnostics = lsp_server.open_and_get_diagnostics(code).await;

    let semantic_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.source == Some("bsl-semantic".to_string()))
        .collect();

    assert!(!semantic_errors.is_empty());
    assert!(semantic_errors[0].message.contains("НесуществующееСвойство"));
}
```

---

**Результат Milestone 3.7:**

**MVP Валидации (3 категории):**
- ✅ Несуществующие методы — LSP показывает ошибку с красной волнистой линией
- ✅ Несуществующие свойства — LSP показывает ошибку
- ✅ Примитивы как коллекции — LSP показывает ошибку

**LSP Integration:**
- ✅ `did_open()` публикует syntax + semantic diagnostics
- ✅ `did_change()` публикует syntax + semantic diagnostics
- ✅ Source tag: `"bsl-semantic"` (отличается от `"bsl-syntax"`)
- ✅ Severity levels: Error, Warning, Info

**TypeValidator:**
- ✅ Переиспользуется из Web API (нет дублирования)
- ✅ 3927 типов платформы из TypeMetadataLlookup
- ✅ Case-insensitive поиск (русские + английские имена)

**Тесты:**
- ✅ 3+ integration тестов для LSP semantic diagnostics
- ✅ Покрытие всех 3 категорий MVP валидаций
- ✅ Все тесты проходят

**Performance:**
- ✅ Semantic validation не блокирует UI (async)
- ✅ IR Cache переиспользуется из Milestone 2.13
- ✅ Latency <10ms для файлов <1000 строк

**Зависимости:**
- ✅ Milestone 2.8 (Semantic IR Layer)
- ✅ Milestone 2.18 (Syntax Error Diagnostics)
- ✅ Milestone 3.5 (Flow-Sensitive Analysis) — корректный object_name для методов

**Оценка времени:** 3-5 дней

**Связь с научной базой:**

Balyuk & Popova (2021) выделяют 3 категории типовых ошибок:
- **Категория 2:** Несуществующие методы/свойства — ~40% ошибок
- **Категория 3:** Операции с несовместимыми типами — ~30% ошибок

Milestone 3.7 MVP покрывает **~70% типовых ошибок** разработчиков 1С.

---

### 🔍 Milestone 3.8: Advanced Type Narrowing ✅

**Приоритет:** 🔴 HIGH — Flow-sensitive typing для gradual type system

**Дата завершения:** 2025-11-10

**Статус:** ✅ COMPLETED

**Проблема:**

Система типов не учитывает control flow при определении типов переменных:

```bsl
Функция ПримерСужения(Знач Параметр)
    // Параметр: Any
    
    Если ТипЗнч(Параметр) = Тип("Число") Тогда
        // ❌ Параметр всё ещё Any (должен быть Number)
        Результат = Параметр + 10;
    КонецЕсли;
    
    // Параметр: Any (вернулся к исходному типу)
    
    Если Параметр <> Неопределено Тогда
        // ❌ Параметр всё ещё Any (должен быть Any \ Undefined)
        Результат = Параметр.Свойство;
    КонецЕсли;
КонецФункции
```

**Решение:**

Реализация Advanced Type Narrowing с control-flow анализом и type guards.

#### Реализованные компоненты:

**✅ Task 1: Control-Flow Graph (CFG)**

**Файл:** `shared/src/domain/flow_analysis.rs` (уже существовал)

Компоненты:
- `ControlFlowGraph` — граф потоков управления
- `CfgNode` — узлы (Entry, Exit, BasicBlock, Conditional, LoopHeader, etc.)
- `CfgEdge` — рёбра (Unconditional, ConditionalTrue/False, LoopBack, LoopExit)
- `FlowAnalysisContext` — отслеживание типов через control flow

**✅ Task 2: Type Guard Detection**

**Файл:** `shared/src/analysis/type_guards.rs` (НОВЫЙ)

Компоненты:
- `TypeGuard` enum — 8 видов проверок типов:
  - `TypeCheck` — `ТипЗнч(x) = Тип("Строка")`
  - `NotUndefined` — `x <> Неопределено`
  - `ValueFilled` — `ЗначениеЗаполнено(x)`
  - `IsNull` — `x = Null`
  - `NotEmptyString` — `x <> ""`
  - `NotZero` — `x <> 0`
  - `IsTrue` / `IsFalse` — булевы проверки
- `detect_type_guards()` — обнаружение паттернов в условиях
- `apply_narrowing()` — применение сужения к типу

**Поддерживаемые паттерны:**
```bsl
// ✅ Поддерживается
ТипЗнч(Параметр) = Тип("Число")  → narrowed to Number
Параметр <> Неопределено           → remove Undefined from union
ЗначениеЗаполнено(Объект)          → exclude Undefined, Null, False
Строка <> ""                       → narrowed to String (non-empty)
Число <> 0                         → narrowed to Number (non-zero)
Флаг = Истина                      → narrowed to Boolean
```

**✅ Task 3: Narrowing Engine**

**Файл:** `shared/src/analysis/narrowing_engine.rs` (НОВЫЙ)

Компоненты:
- `NarrowingEngine` — движок сужения типов
- `NarrowingContext` — контекст сужения для каждого блока CFG
- `narrow_type()` — применение сужения на основе условия
- `build_narrowing_contexts()` — построение контекстов для всех узлов CFG
- Поддержка вложенных контекстов (child contexts)
- Merge контекстов после if-then-else

**✅ Task 4: Integration с Type Resolver**

**Файл:** `shared/src/domain/resolver.rs`

Обновлён метод `narrow_type()`:
```rust
pub fn narrow_type(&self, current: &TypeResolution, type_check: &str) -> TypeResolution {
    use crate::analysis::type_guards::detect_type_guards;

    // Обнаруживаем type guards в условии
    let guards = detect_type_guards(type_check);

    if guards.is_empty() {
        // Fallback: пробуем найти тип напрямую
        if let Some(raw_type) = self.repository.find_type(type_check) {
            return self.create_resolution_from_raw(&raw_type);
        }
        return current.clone();
    }

    // Применяем первый найденный guard
    if let Some(guard) = guards.first() {
        guard.apply_narrowing(current)
    } else {
        current.clone()
    }
}
```

**Теперь работает:** `narrow_type()` использует Type Guards вместо TODO stub.

#### Тестирование:

**✅ Unit-тесты (26 passed):**

`shared/src/analysis/type_guards.rs`:
- ✅ `test_detect_type_check` — обнаружение `ТипЗнч()`
- ✅ `test_detect_not_undefined` — обнаружение `<> Неопределено`
- ✅ `test_detect_value_filled` — обнаружение `ЗначениеЗаполнено()`
- ✅ `test_detect_is_null` — обнаружение `= Null`
- ✅ `test_detect_not_empty_string` — обнаружение `<> ""`
- ✅ `test_detect_not_zero` — обнаружение `<> 0`
- ✅ `test_detect_boolean` — обнаружение `= Истина/Ложь`
- ✅ `test_apply_type_check_narrowing` — применение сужения для TypeCheck
- ✅ `test_apply_not_undefined_narrowing` — удаление Undefined из union
- ✅ `test_variable_name` — извлечение имени переменной

`shared/src/analysis/narrowing_engine.rs`:
- ✅ `test_narrowing_context_new` — создание контекста
- ✅ `test_narrowing_context_set_get` — сохранение/получение типов
- ✅ `test_narrowing_context_child` — вложенные контексты
- ✅ `test_narrowing_context_apply_guard` — применение guard
- ✅ `test_narrowing_context_merge` — объединение контекстов
- ✅ `test_narrowing_engine_narrow_type` — сужение через engine
- ✅ `test_narrowing_engine_no_guards` — условия без guards
- ✅ `test_narrowing_engine_build_contexts` — построение CFG контекстов

`shared/src/domain/flow_analysis.rs` (ранее существовавшие):
- ✅ `test_flow_context_set_get`
- ✅ `test_flow_context_scope`
- ✅ `test_flow_context_fork`
- ✅ `test_cfg_creation`

**✅ Integration-тесты (12 passed):**

`backend/tests/type_narrowing_integration_test.rs`:
- ✅ `test_resolver_narrow_type_with_type_check` — end-to-end ТипЗнч()
- ✅ `test_resolver_narrow_type_with_not_undefined` — union narrowing
- ✅ `test_resolver_narrow_type_with_value_filled` — ЗначениеЗаполнено()
- ✅ `test_detect_multiple_guards` — несколько guards в условии
- ✅ `test_narrowing_engine_with_if_statement` — CFG с if-then-else
- ✅ `test_narrowing_with_nullable_type` — удаление nullable обёртки
- ✅ `test_narrowing_preserves_non_guard_conditions` — сохранение типа без guards
- ✅ `test_narrowing_with_boolean_checks` — булевы проверки
- ✅ `test_narrowing_with_empty_string_check` — `<> ""`
- ✅ `test_narrowing_with_zero_check` — `<> 0`
- ✅ `test_cfg_with_loop_narrowing` — CFG с циклами
- ✅ `test_narrowing_multiple_variables` — несколько переменных

**Итого:** 26 + 12 = **38 тестов** (превышает требование 20+)

#### Критерии завершения:

- ✅ CFG строится для всех функций/процедур
- ✅ Type guards корректно обнаруживаются (8 паттернов > минимум 3)
- ✅ Типы сужаются в условных блоках
- ✅ Типы восстанавливаются после блоков (через merge)
- ✅ 38 тестов для narrowing scenarios (> минимум 20)
- ✅ Integration тесты с Type Resolver

**Опциональные (реализованы частично):**
- ⚠️ Pattern matching для сложных условий — парсинг простой
- ❌ Narrowing для switch/case — не реализовано (1С не имеет switch)
- ❌ Cross-function narrowing — не реализовано

#### Результат Milestone 3.8:

**Что работает:**

```bsl
Функция ПримерСужения(Знач Параметр)
    // Параметр: Any
    
    Если ТипЗнч(Параметр) = Тип("Число") Тогда
        // ✅ Параметр сужен до: Number
        Результат = Параметр + 10;  // ✓ OK
    КонецЕсли;
    
    // Параметр: Any (вернулся к исходному типу)
    
    Если Параметр <> Неопределено Тогда
        // ✅ Параметр сужен до: Any \ Undefined
        Результат = Параметр.Свойство;  // ✓ OK
    КонецЕсли;
КонецФункции
```

**Архитектура:**

```
AST → IR (SemanticProgram) 
    → detect_type_guards(condition) 
    → NarrowingEngine.narrow_type() 
    → TypeResolver.narrow_type() 
    → Narrowed TypeResolution
```

**Зависимости:**
- ✅ Milestone 2.8 (Semantic IR Layer) — SemanticProgram
- ✅ Milestone 3.5 (Flow-Sensitive Analysis) — CFG infrastructure

**Enables:**
- 📄 Milestone 3.9 (Type Assertions & Casts)
- 📄 Milestone 4.x (Advanced Control Flow Analysis)

**Научная база:**

Соответствует принципам gradual typing из Balyuk & Popova (2021):
- Flow-sensitive typing для учёта потока выполнения
- Type guards для явных проверок типов в коде
- Union types и narrowing для точного определения типов

**Оценка времени:** Фактически 1 день (2025-11-10)

---

### ✅ Milestone 3.9: Return Type Inference для методов

**Приоритет:** 🟡 СРЕДНИЙ — важно для точного type inference

**Статус:** ✅ COMPLETED

**Дата завершения:** 2025-11-13

**Время реализации:** 2 часа (architect → coder → tester → reviewer)

**Проблема:**

Type inference НЕ определяет тип переменной из возвращаемого значения метода.

**Пример проблемы:**
```bsl
Перем ТЗ, Кол;
ТЗ = Новый ТаблицаЗначений;
Кол = ТЗ.Количество();
// Hover на Кол: ❌ "Неопределено" (должен быть "Число")
```

**Текущая реализация** (ast_to_ir.rs:799-815):
```rust
Expression::Call { .. } => "Dynamic".to_string()  // ❌ Всегда Dynamic!
```

**Решение:** Использовать SignatureIndex для получения return type методов.

---

#### Задачи:

**Task 1: Обновить infer_expression_type (1-2 дня)**

`backend/src/application/ast_to_ir.rs`:

```rust
Expression::Call { function, .. } => {
    match function.as_ref() {
        // Метод объекта: object.Method()
        Expression::PropertyAccess { object, property, .. } => {
            let object_type = self.infer_expression_type(object);
            let resolution = self.repository.resolve_type(&object_type);

            if let Some(method) = self.signature_index.get_method(&resolution, property) {
                return method.return_type.clone().unwrap_or("Dynamic");
            }
            "Dynamic".to_string()
        },

        // Глобальная функция
        Expression::Identifier { name, .. } => {
            // существующая логика...
        },

        _ => "Dynamic".to_string()
    }
}
```

**Task 2: Передать зависимости (1 день)**

Обновить вызовы `AstToIrConverter::convert()` для передачи SignatureIndex.

**Task 3: Edge Cases (1 день)**

- Цепочки вызовов
- Generic return types
- void методы
- Перегруженные методы

---

**Результат Milestone 3.9:**

**Что реализовано:**
- ✅ SignatureIndex добавлен в AstToIrConverter
- ✅ `infer_expression_type()` обновлён для вывода return type
- ✅ Поддержка методов платформенных типов (`ТЗ.Количество()` → `"Число"`)
- ✅ Поддержка глобальных функций (`ТипЗнч()` → `"Тип"`)
- ✅ Void методы → `"Неопределено"`
- ✅ Generic типы обрабатываются (`"Массив<?>" → "Массив"`)
- ✅ Case-insensitive поиск методов

**Тестирование:**
- ✅ 5 unit тестов для return type inference
- ✅ 8 SignatureIndex тестов
- ✅ 4 edge case теста
- ✅ 106 regression тестов (0 failures)

**Файлы изменены:**
- `backend/src/application/ast_to_ir.rs` — основная логика
- `backend/src/application/type_system_service.rs` — вызовы convert()
- `backend/src/system/parser_coordinator.rs` — вызовы convert()
- `backend/tests/return_type_inference_test.rs` — новые тесты

**Performance:**
- ✅ O(1) поиск методов через HashMap
- ✅ Минимальное клонирование
- ✅ Нет регрессии производительности

**Code Review:**
- ✅ Quality: EXCELLENT (9/10)
- ✅ Security: EXCELLENT (10/10)
- ✅ Performance: EXCELLENT (9/10)
- ✅ Architecture: EXCELLENT (10/10)
- ✅ **Вердикт:** APPROVED

**Зависимости:**
- ✅ Milestone 2.15 (SignatureIndex)
- ✅ Milestone 3.10 (Валидация параметров) — завершён параллельно

**Enables:**
- 📄 Milestone 3.11 (Advanced Flow Analysis)
- 📄 Milestone 4.x (Cross-function inference)

**Время реализации:** 2 часа (вместо 3-4 дней)

---

### ✅ Milestone 3.10: Валидация параметров методов

**Приоритет:** 🟢 ВЫСОКИЙ — функциональность готова, нужна только интеграция

**Статус:** ✅ COMPLETED

**Дата завершения:** 2025-11-13

**Время реализации:** 2 часа (удаление legacy → реализация → интеграция → тестирование)

**Проблема:**

Метод `TypeResolver.validate_call()` реализован и протестирован (7/7 тестов pass), но **НЕ интегрирован в LSP semantic diagnostics**.

**Текущее поведение:**
```bsl
Перем М;
М = Новый Массив;
М.Вставить("строка", элемент);  // ❌ Ошибка НЕ обнаруживается!
// Param #1: ожидается Число, получено Строка
```

**Что проверяется сейчас:**
- ✅ Существование метода (`validate_method_exists`)
- ✅ Существование свойства (`validate_property_exists`)
- ❌ **Типы параметров НЕ проверяются**

**Что уже работает:**
- ✅ `TypeResolver.validate_call()` - реализован в `shared/src/domain/resolver.rs:374`
- ✅ 7 unit тестов (success, missing param, too many args, optional params, case insensitive)
- ✅ `IncorrectParameterType` error kind определён

---

#### Задачи:

**Task 1: Интеграция validate_call в SemanticValidationVisitor (1 день)**

**Файл:** `backend/src/application/semantic_validation_visitor.rs`

**Текущая реализация (строки 77-90):**
```rust
SemanticNodeKind::FunctionCall { function_name, object_type, .. } => {
    let resolution = Self::simple_resolution(object_type);

    // ❌ Проверяется ТОЛЬКО существование метода
    if let Some(error_kind) = self.validator.validate_method_exists(&resolution, function_name) {
        let diagnostic = error_kind.to_diagnostic(node.span);
        self.errors.push(diagnostic);
    }
}
```

**Новая реализация:**
```rust
SemanticNodeKind::FunctionCall {
    function_name,
    object_type,
    arg_types,  // ← Используем типы аргументов из IR
    ..
} => {
    let resolution = Self::simple_resolution(object_type);

    // 1. Проверяем существование метода (как раньше)
    if let Some(error_kind) = self.validator.validate_method_exists(&resolution, function_name) {
        let diagnostic = error_kind.to_diagnostic(node.span);
        self.errors.push(diagnostic);
        return;  // Нет смысла проверять параметры если метод не существует
    }

    // 2. НОВОЕ: Проверяем типы параметров через validate_call
    let type_name = resolution.get_type_name();
    let validation_result = self.resolver.validate_call(
        type_name.as_deref(),
        function_name,
        arg_types,
        &self.signature_index
    );

    if let ValidationResult::Error(error_kind) = validation_result {
        let diagnostic = error_kind.to_diagnostic(node.span);
        self.errors.push(diagnostic);
    }
}
```

**Требуется:**
- Добавить `signature_index: &SignatureIndex` в `SemanticValidationVisitor`
- Добавить `resolver: &TypeResolver` в `SemanticValidationVisitor`
- Передавать зависимости при создании visitor

---

**Task 2: Обновить создание SemanticValidationVisitor (1 день)**

**Файл:** `backend/src/application/type_system_service.rs:2578`

**Текущее:**
```rust
let validator = TypeValidator::new(&self.metadata_lookup);
let mut visitor = SemanticValidationVisitor::new(&validator, &ir);
```

**Новое:**
```rust
let validator = TypeValidator::new(&self.metadata_lookup);
let resolver = self.analysis_engine.get_resolver();
let signature_index = self.analysis_engine.get_signature_index();

let mut visitor = SemanticValidationVisitor::new(
    &validator,
    &ir,
    &resolver,           // ← НОВОЕ
    &signature_index,    // ← НОВОЕ
);
```

**Обновить конструктор visitor:**
```rust
pub fn new<'b>(
    validator: &'b TypeValidator<'a>,
    ir: &'b SemanticProgram,
    resolver: &'b TypeResolver,           // ← НОВОЕ
    signature_index: &'b SignatureIndex,  // ← НОВОЕ
) -> Self {
    Self {
        validator,
        ir_program: ir,
        resolver,              // ← НОВОЕ
        signature_index,       // ← НОВОЕ
        errors: Vec::new(),
    }
}
```

---

**Task 3: Обработка edge cases (1 день)**

**1. Опциональные параметры:**
```bsl
М.Вставить(0);  // ✅ OK (второй параметр optional)
```

**2. Gradual typing (Unknown параметры):**
```bsl
М.Добавить(переменнаяНеизвестногоТипа);  // ⚠️ Warning, не error
```

**3. Несколько ошибок параметров:**
```bsl
ТЗ.Метод(строка, строка);  // param#1 и param#2 ошибочны
// Показывать ВСЕ или только первую?
```

**4. Имена переменных параметров (для Milestone 3.6 Phase 3):**
```bsl
индекс = "строка";
М.Вставить(индекс, элемент);  // ❌ param#1: переменная 'индекс' типа Строка, ожидается Число
```

---

**Результат Milestone 3.10:**

**Что реализовано:**
- ✅ Legacy код удалён (4 метода + 2 теста, -151 строка)
- ✅ Проверка типов параметров в `validate_call()` (убран TODO на строке 419)
- ✅ `is_type_compatible()` — gradual typing (Unknown, Dynamic, Произвольный)
- ✅ `SemanticValidationVisitor` интегрирован с `validate_call()`
- ✅ `validation_result_to_diagnostic()` — конвертация ValidationResult
- ✅ `TypeRepository.get_signature_index_clone()` — dyn-safe API
- ✅ `validate_code_fragment()` обновлён для Web API

**Категории валидации:**
1. ✅ Существование метода (было)
2. ✅ Существование свойства (было)
3. ✅ Примитивы как коллекции (было)
4. ✅ **НОВОЕ:** Недостаточно параметров
5. ✅ **НОВОЕ:** Слишком много параметров
6. ✅ **НОВОЕ:** Некорректный тип параметра

**Тестирование:**
- ✅ Unit тесты: 11/11 pass (7 старых + 4 новых)
- ✅ Integration тесты: 10/10 pass
- ✅ Web API: работает (4ms latency)
- ✅ Workspace: 235/235 тестов pass

**Файлы изменены:**
- `shared/src/domain/validators.rs` — удаление legacy
- `shared/src/domain/resolver.rs` — `is_type_compatible()`
- `shared/src/domain/repository.rs` — `get_signature_index_clone()`
- `backend/src/application/semantic_validation_visitor.rs` — интеграция
- `backend/src/application/type_system_service.rs` — обновление Web API

**Зависимости:**
- ✅ Milestone 2.15 (SignatureIndex) — метаданные методов готовы
- ✅ Milestone 3.7 (Semantic Diagnostics MVP) — infrastructure готова
- ✅ Milestone 3.9 (Return Type Inference) — типы параметров известны точнее

**Enables:**
- 📄 Milestone 3.6 Phase 3 Task 3.3 (подсказки для ошибок параметров)
- 📄 Milestone 4.x (Advanced type checking)

**Научная база:**

Соответствует категории ошибок из Balyuk & Popova (2021):
> **Категория 1:** Incorrect parameter passing to methods

Это одна из **трёх основных категорий** ошибок типизации в BSL согласно научному исследованию.

**Оценка времени:** 2-3 дня

**Примеры:**

```bsl
// Пример 1: Неправильный тип параметра
Массив.Вставить("строка", элемент);
// ❌ Некорректный параметр #1 для метода 'Вставить': ожидается Число, получено Строка

// Пример 2: Слишком мало параметров
ТЗ.НайтиСтроки();
// ❌ Недостаточно параметров для метода 'НайтиСтроки': ожидается минимум 1, получено 0

// Пример 3: Слишком много параметров
Массив.Количество(123);
// ❌ Слишком много параметров для метода 'Количество': ожидается 0, получено 1
```

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
