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
- **Type system facade** — application layer с бизнес-логикой ✅
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
| 2.19 Architectural Improvements | ✅ | 2025-11-07 | Unified ParseError (SSOT), type-system facade parse_and_validate() API, Clean Architecture восстановлена, ~97 строк удалено | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-219-architectural-improvements--2025-11-07) |
| 2.20 Enhanced Status Bar | ✅ | 2025-11-07 | Расширенная строка статуса с прогрессом LSP/индексации, контекстом редактора, статистикой TypeRepository | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-220-enhanced-status-bar--2025-11-07) |
| 2.21 WASM Webviews Migration | ✅ | 2025-11-08 | Полная миграция VSCode Extension webviews на Leptos/WASM, устранение дублирования кода (100% DRY), Security +50%, 10 unit тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-221-wasm-webviews-migration--2025-11-08) |
| 3.5 Flow-Sensitive Analysis | ✅ | 2025-11-08 | Исправлен критический баг hover на вызовах методов, реализован flow-sensitive анализ для отслеживания типов через цепочки вызовов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-35-flow-sensitive-analysis-) |
| 3.6 Enhanced UX | ✅ | 2025-11-22 | 3-фазная реализация: DetailLevel настройки, фасетные типы, улучшенные diagnostics. 79 тестов Milestone + 332 regression, Markdown hover | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-36-enhanced-ux-hover--diagnostics--завершён-2025-11-22) |
| 3.7 Semantic Diagnostics MVP | ✅ | 2025-11-XX | Интеграция семантической валидации в LSP: неизвестные типы, несуществующие методы, type mismatch. 40+ интеграционных тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-37-semantic-diagnostics-mvp-) |
| 3.8 Advanced Type Narrowing | ✅ | 2025-11-10 | Control-flow анализ для сужения типов: if/elif проверки, TypeNarrowing trait, поддержка ТипЗнч() и логических операторов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-38-advanced-type-narrowing-) |
| 3.9 Return Type Inference | ✅ | 2025-11-13 | Автоматический вывод типов возврата для 150+ методов платформы, устранение Неопределено для цепочек вызовов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-39-return-type-inference-для-методов) |
| 3.10 Валидация параметров | ✅ | 2025-11-13 | Проверка количества и типов параметров при вызовах методов, поддержка опциональных параметров, 20+ тестов валидации | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-310-валидация-параметров-методов) |
| 3.1 Code Intelligence | ✅ | 2025-11-XX | Goto Definition, Find References, Rename Symbol, Signature Help — полная навигация по коду | — |
| 3.2 Code Actions | ✅ | 2025-11-XX | 20+ Code Actions: Quick Fixes, Refactorings (Extract Method/Variable), Generate Code | — |
| 3.3 Static Analysis | ✅ | 2025-11-XX | 50+ правил статического анализа: Code Quality, Security, Performance Rules | — |
| 3.12 Enhanced Config Parser | ✅ | 2025-11-23 | Парсинг форм, модулей, контекстных свойств. CodeLocation система. 44 теста (100% pass, 4 фазы) | — |
| 3.13 Object-Based Type Comparison | ✅ | 2025-11-25 | TypeCompatibility, is_compatible_with(), фасетная совместимость (Object→Reference OK), validate_call_v2() | [Детали](docs/roadmap/milestones-3.13-3.15-object-types.md) |
| 3.14 Go To Definition для типов | ✅ | 2025-11-26 | TypeDefinitionLocation, LSP textDocument/definition, навигация к модулям объекта/менеджера | [Детали](docs/roadmap/milestones-3.13-3.15-object-types.md) |
| 3.15 Lazy Resolution | ✅ | 2025-11-26 | OnceCell для кэширования return types, prewarm_signature_cache() | [Детали](docs/roadmap/milestones-3.13-3.15-object-types.md) |
| 3.11 Context-Aware Facet Selection | ✅ | 2025-11-26 | PropertyAccess→Manager, Method facet switching, RuntimeExecutionContext, ContextRequirements | — |
| 3.18 IR Type Resolution Refactoring | ✅ | 2025-11-27 | TypeResolution как единственный источник типов в IR, удалён TypeHint, удалён simple_resolution(), graceful degradation для Unknown | [Детали](docs/roadmap/milestone-3.18-ir-type-resolution.md) |

**Итого завершено:** 31 Milestones
**Прогресс Версии 2.0:** ~95% завершено (19/20 Milestones)
**Прогресс Версии 3.0:** ~93% завершено (14/15 Milestones: 3.1-3.3, 3.5-3.15, 3.18)

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
// type-system facade: get_hover_info()
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
    type_service: Arc<TypeSystemFacade>,
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

> **✅ Завершённые Milestones 3.1-3.15** — см. [таблицу завершённых](#-завершённые-milestones-компактный-формат)

---

### 📦 Milestone 3.12: Enhanced Configuration Parser (✅ ЗАВЕРШЁН 2025-11-23)

> **Детали см. в таблице завершённых Milestones**
>
> **Краткое описание:** Парсинг форм, модулей объектов, контекстных свойств. CodeLocation система для context-aware валидации. 44 теста (100% pass).

---

### 📦 Milestones 3.13-3.15: Object-Based Type System (✅ ЗАВЕРШЕНЫ 2025-11-25/26)

> **Детали:** [docs/roadmap/milestones-3.13-3.15-object-types.md](docs/roadmap/milestones-3.13-3.15-object-types.md)

| Milestone | Ключевой результат |
|-----------|-------------------|
| **3.13 Object-Based Type Comparison** | TypeCompatibility, is_compatible_with(), фасетная совместимость |
| **3.14 Go To Definition** | TypeDefinitionLocation, LSP textDocument/definition |
| **3.15 Lazy Resolution** | OnceCell для кэширования return types |

---

### ✅ Milestone 3.11: Context-Aware Facet Selection

**Статус:** ✅ ЗАВЕРШЁН (2025-11-26)

**Результат:** Реализована фасетная система типов с context-aware выбором фасетов.

**Реализовано:**
- ✅ PropertyAccess → Manager facet (`Справочники.Контрагенты` → `СправочникМенеджер.Контрагенты`)
- ✅ Method facet switching: `СоздатьЭлемент()` → Object, `НайтиПоКоду()` → Reference, `Выбрать()` → Selection
- ✅ RuntimeExecutionContext для server/client контекста (директивы &НаСервере/&НаКлиенте)
- ✅ ContextRequirements в MethodSignature (ServerOnly, ClientOnly, Universal)
- ✅ Hover показывает active facet и context badges

**Тесты:** 55 тестов (context_aware_facets, facet_method_return_type, hover_facets, signature_index_facets)

**Файлы:** `shared/src/domain/runtime_context.rs`, `signature_index.rs`, `backend/src/application/ast_to_ir.rs`

---

### 🔍 Milestone 3.16: Metadata Object Existence Validation (1-2 недели)

**Приоритет:** 🟡 СРЕДНИЙ — улучшение качества валидации типов конфигурации

**Проблема:**
При обращении к менеджерам коллекций (`Документы.ЗаказКлиента`, `Справочники.Контрагенты`) система НЕ проверяет, существует ли объект метаданных в загруженной конфигурации.

**Текущее поведение (некорректное):**
```bsl
ДокМенеджер = Документы.ЗаказКлиента;  // Тип: ДокументМенеджер.ЗаказКлиента (50% Inferred)
                                        // ⚠️ "Детали типа недоступны"
```

**Ожидаемое поведение:**
```bsl
ДокМенеджер = Документы.ЗаказКлиента;  // ❌ Ошибка: Документ "ЗаказКлиента" не найден в конфигурации
                                        // 💡 Возможно, вы имели в виду: "ЗаказНаЭмиссиюКодовМаркировкиСУЗ"
```

#### Задачи:

**Phase 1: Валидация существования объектов метаданных (3-4 дня)**

- [ ] **Task 1.1:** Добавить lookup по имени объекта в MetadataLookup
  - Метод `exists_metadata_object(kind: MetadataKind, name: &str) -> bool`
  - Метод `suggest_similar_names(kind: MetadataKind, name: &str) -> Vec<String>` (Levenshtein distance)

- [ ] **Task 1.2:** Валидация при PropertyAccess к менеджерам коллекций
  - В `semantic_validation_visitor.rs` при обработке `Документы.X`, `Справочники.X`, etc.
  - Проверка: существует ли `X` в загруженных метаданных
  - Генерация `SemanticError::UnknownMetadataObject`

**Phase 2: Semantic Diagnostics (2-3 дня)**

- [ ] **Task 2.1:** Новый тип семантической ошибки
  ```rust
  SemanticError::UnknownMetadataObject {
      kind: MetadataKind,      // Document, Catalog, Register, etc.
      name: String,            // "ЗаказКлиента"
      suggestions: Vec<String>, // ["ЗаказНаЭмиссию...", "ЗаказНаряд"]
      span: Span,
  }
  ```

- [ ] **Task 2.2:** Конвертация в LSP Diagnostic
  - Severity: Error
  - Code: `unknown-metadata-object`
  - Message: `Документ "ЗаказКлиента" не найден в конфигурации`
  - Related Info: Список похожих имён (если есть)

**Phase 3: Hover улучшения (1-2 дня)**

- [ ] **Task 3.1:** Обновить hover для несуществующих объектов
  - Показывать чёткое сообщение об отсутствии объекта
  - НЕ показывать синтезированный тип с 50% уверенностью

#### Критерии успеха:

- [ ] 5+ тестов на валидацию существования объектов
- [ ] Ошибка для несуществующих документов, справочников, регистров
- [ ] Подсказки похожих имён (fuzzy matching)
- [ ] Корректный hover для несуществующих объектов

#### Зависимости:

**Этот Milestone использует:**
- ✅ Milestone 2.17 (Configuration Metadata Parser) — загрузка метаданных
- ✅ Milestone 3.7 (Semantic Diagnostics) — инфраструктура ошибок
- ✅ Milestone 3.15 (Lazy Resolution) — доступ к метаданным

**Используют этот Milestone:**
- Milestone 3.11 (Context-Aware Facets) — более точная валидация

#### Оценка времени:

- **Phase 1:** 3-4 дня
- **Phase 2:** 2-3 дня
- **Phase 3:** 1-2 дня
- **Итого:** 6-9 дней (1-2 недели)

---

### ✅ Milestone 3.18: IR Type Resolution Refactoring (ЗАВЕРШЁН 2025-11-27)

**Статус:** ✅ ЗАВЕРШЁН

**Проблема:**
IR хранит типы как `String`, теряя критичную информацию:
- **Certainty** (Known/Inferred/Unknown) — не сохраняется
- **Facet** (Manager/Object/Reference) — не сохраняется
- **UncertaintyReason** — не передается в диагностики

**Результат бага:** Валидация показывает ошибки для Unknown типов (должна пропускать).

```
Hover: ДокОбъект → Unknown (0%)
Validation: "Метод 'Провести' не существует" ← ОШИБКА!
```

**Корень проблемы:**
- `infer_expression_type()` в ast_to_ir.rs возвращает String, не вызывает TypeResolver
- `simple_resolution()` конвертирует String → TypeResolution с `certainty: Known` (всегда!)

**Решение: TypeResolution как единая точка ответственности**

```rust
// Используем существующий TypeResolution напрямую в IR (не создаём TypeRef)
pub struct TypeResolution {
    pub certainty: Certainty,        // Known/Inferred/Unknown
    pub result: ResolutionResult,    // Resolved/Unknown с причиной
    pub source: ResolutionSource,    // Static/Inferred/Metadata
    pub metadata: ResolutionMetadata,
    pub active_facet: Option<FacetKind>,
    pub available_facets: Vec<FacetKind>,
}
// Размер: ~300 байт × 200 типов × 100 файлов = ~6 MB (допустимо)
```

**Удаляемые структуры:**
- `TypeHint` — BSL не имеет type annotations, всё выводится
- `simple_resolution()` — костыль с багом (всегда Known)

#### Фазы реализации:

| Фаза | Срок | Задачи |
|------|------|--------|
| Phase 1 | 2 дня | Serialize/Deserialize для TypeResolution, конструкторы |
| Phase 2 | 2 дня | Удалить TypeHint, заменить на TypeResolution в SymbolTable |
| Phase 3 | 1 неделя | Мигрировать 24 поля SemanticNodeKind на TypeResolution |
| Phase 4 | 2 дня | Удалить `simple_resolution()`, обновить валидацию |
| Phase 5 | 3 дня | Интеграционное тестирование |

#### Критерии успеха:

- [x] TypeResolution используется напрямую в IR (17+ полей)
- [x] TypeHint полностью удалён
- [x] `simple_resolution()` полностью удалён
- [x] Unknown типы **НЕ** генерируют cascade ошибки
- [x] FlowContext использует TypeResolution
- [x] 162/163 тестов проходят (2 предсуществующих падения)
- [x] IR Cache работает с TypeResolution

**Зависимости:**
- ✅ Milestone 3.13 (TypeCompatibility, is_compatible_with)
- ✅ Milestone 3.16 (UncertaintyReason)

**Разблокирует:**
- Milestone 3.4 (LLM-First Tooling) — корректный Impact Analysis

> **Детали:** [docs/roadmap/milestone-3.18-ir-type-resolution.md](docs/roadmap/milestone-3.18-ir-type-resolution.md)

---

### 🤖 Milestone 3.4: LLM-First Tooling — MCP Server & CLI (2.5-3.5 недели)

**Приоритет:** 🔴 ВЫСОКИЙ — ключевой инструмент для AI-assisted разработки

**Проблема:**

При использовании **gradual typing** одна ошибка в коде может привести к тому, что большая часть последующего кода остаётся **непроверенной**. LLM-агент (Claude, ChatGPT) видит только одну диагностику и не понимает полный "радиус поражения" ошибки.

**Пример проблемы:**
```bsl
// Строка 1: Ошибка - документ не существует
ДокМенеджер = Документы.НесуществующийДокумент;

// Строки 2-5: Весь код НЕ ПРОВЕРЯЕТСЯ!
Ссылка = ДокМенеджер.НайтиПоНомеру("123");  // НЕ проверено
Объект = Ссылка.ПолучитьОбъект();           // НЕ проверено
Объект.НесуществующееСвойство = 123;        // ПРОПУЩЕНА ошибка!
Объект.Записать();                          // НЕ проверено
```

**Что видит LLM:** `Error: Документ "НесуществующийДокумент" не найден`

**Что LLM НЕ понимает:**
- Переменные `ДокМенеджер`, `Ссылка`, `Объект` теперь имеют тип `Unknown`
- Все вызовы методов на Unknown типах НЕ валидировались
- Ошибка `НесуществующееСвойство` была **ПРОПУЩЕНА** из-за gradual typing
- ~80% кода не было проверено

#### Решение: LLM-First диагностики

**Расширенный вывод с Impact Analysis:**
```json
{
  "diagnostics": [{
    "message": "Документ \"НесуществующийДокумент\" не найден",
    "impact_severity": "Critical",
    "affected_variables": [
      {"name": "ДокМенеджер", "usage_count": 3},
      {"name": "Ссылка", "usage_count": 2},
      {"name": "Объект", "usage_count": 2}
    ],
    "unchecked_operations": [
      {"kind": "MethodCall", "line": 2, "method": "НайтиПоНомеру"},
      {"kind": "MethodCall", "line": 3, "method": "ПолучитьОбъект"},
      {"kind": "PropertyAccess", "line": 4, "property": "НесуществующееСвойство"},
      {"kind": "MethodCall", "line": 5, "method": "Записать"}
    ]
  }],
  "coverage": {
    "coverage_percent": 20.0,
    "checked_expressions": 1,
    "unchecked_expressions": 4
  },
  "summary": "Critical error affecting 3 variables and 4 operations. Fix to unlock 80% of validation."
}
```

#### Задачи

**Phase 1: LLM-Friendly Diagnostics (4-5 дней)**

- [ ] **Task 1.1:** Impact Tracking в TypeResolver
  - `ImpactTracker` — отслеживание "радиуса поражения" Unknown типов
  - `AffectedVariable` — переменные, ставшие Unknown из-за ошибки
  - `UncheckedOperation` — операции, не прошедшие валидацию

- [ ] **Task 1.2:** Extended Diagnostics API
  - `LlmDiagnostic` — расширенная структура с impact analysis
  - `ImpactSeverity` — Low/Medium/High/Critical на основе радиуса поражения
  - `validate_for_llm()` в type-system facade

- [ ] **Task 1.3:** Type Coverage Calculator
  - `TypeCoverageReport` — % кода с проверенными типами
  - Breakdown по переменным и операциям
  - Причины непроверенных участков

**Phase 2: MCP Server Implementation (5-6 дней)**

- [ ] **Task 2.1:** MCP Server Core (`bsl-mcp-server` crate)
  - Resources: `bsl://types/platform`, `bsl://types/config`
  - Tools: `validate_code`, `get_type_info`, `get_error_impact`
  - STDIO transport для Claude Desktop

- [ ] **Task 2.2:** MCP Tools
  ```rust
  #[tool(description = "Validate BSL code with impact analysis")]
  async fn validate_code(code: String, include_coverage: bool) -> ValidateResult;

  #[tool(description = "Get type at position")]
  async fn get_type_info(code: String, line: u32, column: u32) -> TypeInfo;

  #[tool(description = "Analyze error blast radius")]
  async fn get_error_impact(code: String, error_line: u32) -> ImpactResult;
  ```

- [ ] **Task 2.3:** MCP Prompts
  - `analyze-bsl` — шаблон для анализа кода
  - `fix-type-errors` — шаблон для исправления с учётом gradual typing

**Phase 3: CLI Implementation (3-4 дня)**

- [ ] **Task 3.1:** CLI Binary (`bsl-types`)
  ```bash
  bsl-types validate <file> [--json] [--sarif] [--coverage] [--impact]
  bsl-types coverage <file>
  bsl-types hover <file> <line> <column>
  bsl-types impact <file> <error_line>
  bsl-types mcp-server [--stdio]
  ```

- [ ] **Task 3.2:** Output Formatters
  - Text (human-readable с emoji для severity)
  - JSON (для LLM и программной обработки)
  - SARIF (для IDE интеграции)

**Phase 4: Documentation & Prompts (2 дня)**

- [ ] **Task 4.1:** System Prompts для Claude/ChatGPT
  - Объяснение gradual typing и "радиуса поражения"
  - Best practices для работы с BSL кодом

- [ ] **Task 4.2:** Usage Guide
  - CLI примеры
  - MCP integration для Claude Desktop
  - Интерпретация результатов

#### Критерии успеха

- [ ] `validate_code` возвращает `affected_variables` и `unchecked_operations`
- [ ] `impact_severity` корректно вычисляется (Low/Medium/High/Critical)
- [ ] Type coverage report показывает % проверенного кода
- [ ] MCP Server работает через STDIO с Claude Desktop
- [ ] CLI выводит в форматах text/json/sarif
- [ ] 30+ unit-тестов для impact tracking
- [ ] 10+ integration-тестов для MCP server
- [ ] Документация с примерами

#### Зависимости

**Использует:**
- ✅ Milestone 2.8 (Semantic IR) — для анализа AST
- ✅ Milestone 3.7 (Semantic Diagnostics) — для базовых ошибок
- ✅ Milestone 3.16 (Metadata Validation) — для UncertaintyReason

**Внешние зависимости:**
```toml
rmcp = "0.3"  # MCP protocol implementation
```

#### Оценка времени

| Phase | Задачи | Оценка |
|-------|--------|--------|
| Phase 1: LLM Diagnostics | 1.1, 1.2, 1.3 | 4-5 дней |
| Phase 2: MCP Server | 2.1, 2.2, 2.3 | 5-6 дней |
| Phase 3: CLI | 3.1, 3.2 | 3-4 дня |
| Phase 4: Documentation | 4.1, 4.2 | 2 дня |
| **Итого** | | **14-17 дней (2.5-3.5 недели)** |

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
- ⏳ MCP Server для интеграции с LLM (Milestone 3.4)
- ⏳ LLM-First диагностики с Impact Analysis (Milestone 3.4)
- ⏳ CLI утилита `bsl-types` для shell (Milestone 3.4)

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
- ⏳ AI-ассистент понимает "радиус поражения" ошибок (Milestone 3.4)
- ⏳ Type Coverage отчёты для LLM (Milestone 3.4)

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

**Интеграция с type-system facade:**
```rust
// backend/src/application/type_system_service.rs
impl TypeSystemFacade {
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

### ✅ Milestone 4.4: MCP Debug Server — AI-Powered Interactive Debugging

**Статус:** ✅ ЗАВЕРШЁН

**Результат:** Создан MCP Debug Server с DAP bridge для интерактивной отладки программ через AI.

**MCP Tools (доступны в Claude Code):**
- `mcp__mcp-debug__debug_create_session` — создание debug сессии
- `mcp__mcp-debug__debug_set_breakpoint` — установка breakpoint
- `mcp__mcp-debug__debug_launch` — запуск программы
- `mcp__mcp-debug__debug_step_in` / `debug_next` / `debug_step_out` — пошаговое выполнение
- `mcp__mcp-debug__debug_continue` — продолжить выполнение
- `mcp__mcp-debug__debug_eval` — показать переменную
- `mcp__mcp-debug__debug_backtrace` — stack trace
- `mcp__mcp-debug__debug_set_conditional_breakpoint` — условный breakpoint
- `mcp__mcp-debug__debug_poll_events` — получение событий отладчика

**Поддерживаемые языки:** Rust (CodeLLDB), C/C++, Go, Python — любой язык с DAP support

**Документация:** См. секцию "Отладка Rust кода через MCP Debug" в CLAUDE.md

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
