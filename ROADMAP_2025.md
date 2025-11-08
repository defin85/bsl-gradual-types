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
- ⚠️ **Flow-sensitive analysis** — планируется в версии 3.0 (Milestone 3.5)
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

**Итого завершено:** 18 Milestones
**Прогресс Версии 2.0:** ~90% завершено (18/20 Milestones)

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

### 🔧 Milestone 3.5: Flow-Sensitive Analysis (5-7 дней)

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

### 🎨 Milestone 3.6: Enhanced Hover Experience (10-12 дней)

**Приоритет:** 🟡 СРЕДНИЙ — значительное улучшение UX, но не критично для функциональности

**Проблема:**

Текущий hover недостаточно гибкий и информативный:
- ❌ Нет настроек — пользователь не может выбрать уровень детализации
- ❌ Не показываются фасеты (Manager vs Object vs Reference)
- ❌ Generic типы не объясняются (`Массив<Строка>` выглядит как обычный тип)
- ❌ Нет ссылок на platform documentation
- ❌ Методы с большим количеством параметров плохо читаются

**Исследование:**

Проведён детальный анализ hover в современных IDE (Rust Analyzer, TypeScript, Pylance, JetBrains IDEA).
**Документ:** `docs/research/hover-best-practices-2025.md` (67 страниц)

**Ключевые находки:**
- 🏆 **Rust Analyzer** — золотой стандарт (настройки, интерактивность, expandable sections)
- ⚙️ **TypeScript** — фокус на кастомизации (уровни детализации)
- 📖 **Pylance** — rich formatting с docstrings
- 💡 **Best Practice:** три уровня детализации (compact → full → detailed)

**Решение:**

Реализовать настраиваемый hover с тремя уровнями детализации, фасетами, Generic типами и ссылками на документацию.

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

**Зависимости:**
- ✅ Milestone 2.9 (Inline Scope Analysis) — hover уже работает
- ✅ Milestone 2.11 (Span Extraction) — координаты корректны
- 📄 Исследование hover best practices (завершено)

**Оценка времени:** 10-12 дней

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

---

### 🎯 Результаты Версии 3.0 (через 6 месяцев от старта)

**Технические метрики:**
- ✅ Goto Definition, Find References, Rename
- ✅ 20+ Code Actions (Quick Fixes, Refactorings)
- ✅ 50+ Static Analysis Rules
- ✅ Code Quality Dashboard
- ✅ Flow-Sensitive Analysis — hover корректно работает на вызовах методов
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
