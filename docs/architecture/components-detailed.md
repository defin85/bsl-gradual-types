# Components Detailed

Детальное описание ключевых компонентов BSL Gradual Type System.

## 📚 Обзор

**См. также:**
- **[Type System Architecture](type_system_architecture.md)** — общая архитектура системы типов
- **[Milestones History](milestones-history.md)** — история развития компонентов

---

## 🎯 System Layer (backend/src/system/)

### SystemCoordinator

**Назначение:** Composition Root, управление жизненным циклом приложения

**Структура:**
```rust
pub struct SystemCoordinator {
    disk_cache: Arc<DiskCache>,
    parser_coordinator: Arc<ParserCoordinator>,
    observability: Arc<BasicObservability>,
    analysis_host_v2: Arc<AnalysisHostV2>,
}
```

**Ответственность:**
- Инициализация всех компонентов системы
- Управление зависимостями (DI container)
- Координация жизненного цикла сервисов
- Предоставление единой точки входа

**Пример использования:**
```rust
let coordinator = SystemCoordinator::new();
let engine = coordinator.analysis_engine();
```

---

### DiskCache

**Назначение:** LRU-кэш для результатов анализа файлов

**Структура:**
```rust
pub struct DiskCache {
    cache: Arc<RwLock<LruCache<FileHash, CachedAnalysis>>>,
    ttl: Duration,
}
```

**Ключевые методы:**
- `get(file_hash: &FileHash) -> Option<CachedAnalysis>`
- `insert(file_hash: FileHash, analysis: CachedAnalysis)`
- `invalidate(file_hash: &FileHash)`
- `clear()`

**Стратегия кэширования:**
- LRU (Least Recently Used) с максимальным размером
- TTL (Time To Live) для автоматической инвалидации
- File hash для быстрого lookup

**Файл:** [backend/src/system/disk_cache.rs](../../backend/src/system/disk_cache.rs)

---

### ParserCoordinator

**Назначение:** TreeSitter (primary) + Regex (fallback), конвертация AST → IR

**Структура:**
```rust
pub struct ParserCoordinator {
    tree_sitter: TreeSitterAdapter,
    regex_parser: RegexParser,
    converter: AstToIrConverter,
}
```

**Реализует:** `Parser` trait из `shared/src/parsing/`

**Логика:**
```text
1. Попытка парсинга через TreeSitter
2. Если ошибка → fallback на Regex parser
3. Конвертация AST → IR (SemanticProgram)
4. Возврат SemanticProgram
```

**Файл:** [backend/src/system/parser_coordinator.rs](../../backend/src/system/parser_coordinator.rs)

---

### BasicObservability

**Назначение:** Структурированное логирование и базовые метрики

**Функции:**
- Логирование событий (INFO, WARN, ERROR, DEBUG)
- Метрики производительности
- Health endpoint для мониторинга
- Трассировка запросов

**Файл:** [backend/src/system/observability.rs](../../backend/src/system/observability.rs)

---

## 🔧 Application Layer

### AnalysisEngine (shared/src/engine/)

**Назначение:** Чистый оркестратор анализа, работает с IR вместо AST

**Структура:**
```rust
pub struct AnalysisEngine {
    resolver: TypeResolver,
    metadata_lookup: TypeMetadataLookup,
}
```

**Ключевой метод:**
```rust
pub fn analyze_program(&self, program: &SemanticProgram)
    -> Result<TypedProgram, AnalysisError>
{
    // 1. Обход SemanticNode дерева
    // 2. Резолв типов через TypeResolver
    // 3. Построение TypedProgram с аннотациями типов
}
```

**Особенности:**
- ✅ Не зависит от backend (находится в shared)
- ✅ Не зависит от конкретного парсера (работает с IR)
- ✅ Переиспользуется LSP, Web API, CLI

**Файл:** [shared/src/engine/mod.rs](../../shared/src/engine/mod.rs)

---

### TypeSystemFacade (backend/src/application/)

**Назначение:** Высокоуровневый API для LSP/Web с кэшированием

**Структура:**
```rust
pub struct TypeSystemFacade {
    engine: AnalysisEngine,
    disk_cache: Arc<DiskCache>,
    parser: Arc<dyn Parser>,
    converter: AstToIrConverter,
}
```

**Ключевые методы:**

#### get_hover_info_ir() — IR-based hover

```rust
pub fn get_hover_info_ir(&self, file: &str, line: u32, column: u32)
    -> Result<HoverInfo>
{
    // 1. Парсим файл → SemanticProgram
    let ir = self.parser.parse(source)?;

    // 2. Inline Scope Analysis: find_variable_at_position
    let (var_name, type_hint) = ir.find_variable_at_position(line, column)?;

    // 3. Резолвим тип через TypeRepository
    let type_metadata = self.repository.find_type(&type_hint.name)?;

    // 4. Получаем методы/свойства
    let methods = self.metadata_lookup.get_methods(&type_metadata)?;

    // 5. Формируем hover text
    Ok(HoverInfo { var_name, type_name, methods, certainty })
}
```

#### get_hover_info() — AST-based fallback

```rust
pub fn get_hover_info(&self, file: &str, line: u32, column: u32)
    -> Result<HoverInfo>
{
    // Старый метод для совместимости
    // Использует find_node_at_position() с реальными Span
}
```

**Файл:** [backend/src/application/type_system_service.rs](../../backend/src/application/type_system_service.rs)

---

### AstToIrConverter (backend/src/application/)

**Назначение:** Мост между tree-sitter AST и SemanticProgram

**Процесс конвертации:**

#### Двухпроходная конвертация

**Проход 1: Сбор символов**
```rust
fn collect_symbols(&mut self, node: &tree_sitter::Node) {
    // Обход AST и сбор всех определений переменных/функций
    // Построение SymbolTable с иерархией scope
}
```

**Проход 2: Конвертация узлов**
```rust
fn convert_node(&self, node: &tree_sitter::Node) -> SemanticNode {
    match node.kind() {
        "variable_declaration" => SemanticNode::Variable { ... },
        "function_declaration" => SemanticNode::Function { ... },
        "if_statement" => SemanticNode::IfStatement { ... },
        // ... другие узлы
    }
}
```

**Особенности:**
- Извлекает реальные Span из tree-sitter (Milestone 2.11)
- Строит SymbolTable для Inline Scope Analysis (Milestone 2.9)
- Упрощает сложное дерево tree-sitter → компактная SemanticNode структура

**Файл:** [backend/src/application/ast_to_ir.rs](../../backend/src/application/ast_to_ir.rs)

---

## 🌟 Semantic Layer (shared/src/ir/)

### SemanticProgram

**Назначение:** Промежуточное представление программы, независимое от парсера

**Структура:**
```rust
pub struct SemanticProgram {
    pub nodes: Vec<SemanticNode>,       // Упрощённое дерево
    pub symbol_table: SymbolTable,       // Иерархия областей видимости
    pub source_map: SourceMap,           // Связь с исходным кодом
}
```

**Ключевые методы:**

#### find_variable_at_position()

```rust
pub fn find_variable_at_position(&self, line: u32, column: u32)
    -> Option<(String, TypeHint)>
{
    // Поиск в scope hierarchy (снизу вверх)
    // Используется в Inline Scope Analysis (Milestone 2.9)
}
```

#### find_node_at_position()

```rust
pub fn find_node_at_position(&self, line: u32, column: u32)
    -> Option<&SemanticNode>
{
    // Поиск узла по позиции курсора
    // Используется в LSP hover (Milestone 2.11)
}
```

**Файл:** [shared/src/ir/mod.rs](../../shared/src/ir/mod.rs)

---

### SemanticNode

**Назначение:** Упрощённый набор узлов вместо сложного tree-sitter AST

**Enum:**
```rust
pub enum SemanticNode {
    Variable {
        name: String,
        type_hint: Option<TypeHint>,
        span: Span,
    },
    Function {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeHint>,
        body: Vec<SemanticNode>,
        span: Span,
    },
    IfStatement {
        condition: Box<Expr>,
        then_branch: Vec<SemanticNode>,
        else_branch: Option<Vec<SemanticNode>>,
        span: Span,
    },
    // ... другие узлы (ForLoop, WhileLoop, Assignment, MethodCall, etc.)
}
```

**Преимущества:**
- ✅ Гораздо проще tree-sitter AST (~10 типов узлов vs ~50)
- ✅ Содержит только семантически значимые узлы
- ✅ Легко создавать вручную для тестов
- ✅ Независим от синтаксических деталей

**Файл:** [shared/src/ir/semantic_node.rs](../../shared/src/ir/semantic_node.rs)

---

### SymbolTable

**Назначение:** Иерархия областей видимости с символами

**Структура:**
```rust
pub struct SymbolTable {
    scopes: Vec<Scope>,          // Стек областей видимости
    current_scope_id: ScopeId,   // Текущая область
}

pub struct Scope {
    id: ScopeId,
    parent: Option<ScopeId>,
    symbols: HashMap<String, Symbol>,  // Имя → Символ
}

pub struct Symbol {
    name: String,
    type_hint: TypeHint,
    kind: SymbolKind,  // Variable | Function | Parameter
    span: Span,
}
```

**Использование:**
- Inline Scope Analysis (Milestone 2.9)
- Разрешение имён переменных
- Обнаружение shadowing и конфликтов имён

**Файл:** [shared/src/ir/symbol_table.rs](../../shared/src/ir/symbol_table.rs)

---

### Parser trait

**Назначение:** Dependency Inversion для разных парсеров

**Trait:**
```rust
pub trait Parser {
    fn parse(&self, source: &str) -> Result<SemanticProgram, ParseError>;
}
```

**Реализации:**

#### ParserCoordinator (backend)
- TreeSitter + Regex → IR
- Полнофункциональный парсер (~15 MB с tree-sitter)

#### LightweightParser (cli)
- Упрощённый regex-based парсер (~2-3 MB)
- Достаточно для базовых задач CLI

**Преимущество:**
- ✅ AnalysisEngine не зависит от конкретного парсера
- ✅ Можно подменить парсер без изменения анализатора
- ✅ Упрощает тестирование (mock parser)

**Файл:** [shared/src/parsing/mod.rs](../../shared/src/parsing/mod.rs)

---

## 🧠 Domain Layer (shared/src/domain/)

### TypeResolver

**Назначение:** Центральная логика анализа типов

**Структура:**
```rust
pub struct TypeResolver {
    repository: Arc<TypeRepository>,
    facet_rules: FacetRules,
}
```

**Ключевые методы:**

#### resolve_type()

```rust
pub fn resolve_type(&self, hint: &TypeHint) -> TypeResolution {
    match hint {
        TypeHint::Primitive(name) => self.resolve_primitive(name),
        TypeHint::Configuration(prefix, member) => self.resolve_member_access(prefix, member),
        TypeHint::Inferred(expr) => self.infer_from_expression(expr),
    }
}
```

#### resolve_member_access()

```rust
pub fn resolve_member_access(&self, prefix: &str, member: &str) -> TypeResolution {
    let type_name = format!("{}.{}", prefix, member);

    // ✅ ИСПРАВЛЕНИЕ (2025-01-18): Честная оценка certainty
    let has_metadata = self.repository.find_type(&type_name).is_some();

    let (certainty, source) = if has_metadata {
        (Certainty::Known, ResolutionSource::Static)
    } else {
        (Certainty::Inferred(0.5), ResolutionSource::Inferred)
    };

    TypeResolution { certainty, result, source }
}
```

**Особенности:**
- Flow-sensitive анализ (планируется)
- Фасетная система (Manager | Object | Reference)
- Градуальная типизация (Known | Inferred | Unknown)

**Файл:** [shared/src/domain/resolver.rs](../../shared/src/domain/resolver.rs)

---

### TypeRepository

**Назначение:** Абстракция для работы с метаданными типов

**Структура:**
```rust
pub struct TypeRepository {
    types: HashMap<String, TypeMetadata>,
    categories: HashMap<String, Vec<String>>,
}
```

**Данные:**
- **3927 типов платформы** из Syntax Helper
- **276 категорий** с правильными названиями
- **6975 методов** объектов
- **13357 свойств** объектов
- **476 глобальных функций**
- **712 системных перечислений**

**Методы:**
- `find_type(name: &str) -> Option<&TypeMetadata>`
- `get_methods(type_name: &str) -> Vec<Method>`
- `get_properties(type_name: &str) -> Vec<Property>`
- `load_platform_types(path: &Path) -> Result<()>`

**Файл:** [shared/src/domain/repository.rs](../../shared/src/domain/repository.rs)

---

### TypeMetadataLookup

**Назначение:** Получение детальной информации о типах

**Функции:**
- Получение методов с описаниями
- Получение свойств с типами
- Фильтрация по активному фасету
- Поиск перегруженных методов

**Использование:**
```rust
let methods = metadata_lookup.get_methods(&type_metadata)?;
let properties = metadata_lookup.get_properties(&type_metadata)?;
```

**Файл:** [shared/src/domain/metadata_lookup.rs](../../shared/src/domain/metadata_lookup.rs)

---

## 🌐 Presentation Layer

### LSP Server (backend/src/presentation/lsp_server.rs)

**Назначение:** Language Server Protocol для VSCode Extension

**Поддерживаемые LSP возможности:**
- `textDocument/hover` — информация о типах
- `textDocument/completion` — автодополнение
- `textDocument/definition` — переход к определению
- `textDocument/diagnostics` — ошибки компиляции (планируется Milestone 2.18)

**Custom requests:**
- `bsl/getSemanticHtml` — Semantic Tree Visualization (Milestone 2.16)

---

### Web Server (backend/src/presentation/web_server.rs)

**Назначение:** Axum web server для API endpoints

**Endpoints:**
- `GET /api/health` — health check
- `GET /api/types?search=<query>` — поиск типов
- `POST /api/analyze` — анализ кода
- `GET /api/semantic/:file_path` — semantic visualization (Milestone 2.16)

**Интеграция:** Отдаёт статические WASM файлы frontend

---

### SemanticRoutes (backend/src/presentation/semantic_routes.rs)

**Назначение:** HTTP endpoint для Semantic Tree Visualization

**Endpoint:**
```
GET /api/semantic/:file_path?format=json|html&theme=dark|light&compact=false
```

**Статус:** ⚠️ MVP stub (заглушка для тестирования API контракта)

**Файл:** [backend/src/presentation/semantic_routes.rs](../../backend/src/presentation/semantic_routes.rs)

---

## 🔗 Связанные документы

- **[Type System Architecture](type_system_architecture.md)** — общая архитектура
- **[Milestones History](milestones-history.md)** — история развития
- **[ROADMAP_2025.md](../../ROADMAP_2025.md)** — актуальный план развития
- **[Development Workflow](../guides/development-workflow.md)** — команды разработки
