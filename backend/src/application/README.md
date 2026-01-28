# Application Layer

> Бизнес-логика типизации и LSP функций

## Обзор

Application layer содержит высокоуровневую логику системы типов:
- Вывод типов (type inference)
- Семантическая валидация (semantic validation)
- LSP сервисы (hover, completion, diagnostics, etc.)
- Конвертация AST → IR (Intermediate Representation)

**Принцип:** Application layer не знает о деталях парсинга или хранения данных. Работает с абстракциями (IR, Repository).

## Структура компонентов

```
application/
├── type_system/              # Entry points и services для LSP/Web/CLI
│   ├── mod.rs                # re-exports / public API
│   ├── services/             # LSP services (hover, completion, etc.)
│   ├── loaders/              # Загрузчики метаданных
│   ├── extractors/           # Извлечение информации из AST/IR
│   └── formatters/           # Форматирование ответов
│
├── semantic_validation_visitor/  # Семантическая валидация
│   ├── visitor.rs            # SemanticValidationVisitor
│   └── validators/           # Специфичные валидаторы
│
├── ast_to_ir/                # Конвертация AST → IR
│   └── converter.rs          # AstToIrConverter
│
└── type_inference_service.rs # Вывод типов выражений
```

## Ключевые компоненты

### type_system (entrypoints)

**Папка:** `type_system/`

Канонический набор entrypoints и services для LSP/Web/CLI.
В IntelliSense v2 путь вычисления работает на:
- `SemanticProgram` (IR) из `bsl-analysis-v2` (salsa queries),
- `DepsBundleV2` / `SemanticDeps` (атомарный deps snapshot),
- `IndexSnapshot` (snapshot индекса).

**Основные функции:**
- `get_hover_info_with_semantic_program(analysis, file_id, ...) -> Option<String>`
- `get_completion_with_semantic_program_snapshot(..., member_access_owner_type_hint) -> Result<CompletionResult>`
- `web_api_service::*` — helpers для Web API (DTO/semantic tree/etc.)

**Структура:**
- `services/` - hover/completion/web api
- `loaders/` - загрузка TypeRepository/метаданных
- `extractors/` - извлечение данных из source/IR
- `formatters/` - форматирование ответов

### SemanticValidationVisitor

**Файл:** `semantic_validation_visitor/visitor.rs`

Валидация семантических правил языка 1С.

**Проверки:**
- Несоответствие типов параметров
- Вызов несуществующих методов/свойств
- Некорректные операции над типами
- Использование необъявленных переменных

**Пример использования:**

```rust
let mut visitor = SemanticValidationVisitor::new(type_repository);
visitor.visit_program(&ir_program);
let errors = visitor.errors();
```

**Статус:** Milestone 3.7 (MVP) - базовые проверки реализованы.

### AstToIrConverter

**Файл:** `ast_to_ir/converter.rs`

Конвертирует AST (tree-sitter) в IR (Intermediate Representation).

**Зачем IR?**
- Независимость от конкретного парсера (tree-sitter, regex, etc.)
- Упрощённое представление для анализа
- Единая точка для всех анализаторов

**Процесс конвертации:**

```
tree-sitter AST
      ↓
AstToIrConverter
      ↓
SemanticProgram (IR)
  - SemanticNode (упрощённые AST узлы)
  - SymbolTable (таблица символов)
      ↓
AnalysisEngine
```

**Пример:**

```rust
let converter = AstToIrConverter::new();
let ir_program = converter.convert(&ast, source_code)?;

// Теперь работаем с IR, а не с AST
let symbol_table = ir_program.symbol_table();
```

### TypeInferenceService

**Файл:** `type_inference_service.rs`

Вывод типов для выражений.

**Возможности:**
- Вывод типа по контексту (присваивание, вызов функции)
- Поддержка градуальной типизации (`Certainty::Known | Inferred | Unknown`)
- Учёт flow-sensitive анализа

**Пример:**

```rust
let service = TypeInferenceService::new(type_repository);
let expr_type = service.infer_expression_type(&expr_node, &context)?;

match expr_type.certainty {
    Certainty::Known => println!("Точно известен тип"),
    Certainty::Inferred(confidence) => println!("Выведен с уверенностью {}", confidence),
    Certainty::Unknown => println!("Тип неизвестен"),
}
```

## Точки входа

### Для LSP функций

**1. Hover** (`type_system/services/hover_service.rs`):

```
LSP textDocument/hover
      ↓
bin/lsp_server/handlers/hover.rs
      ↓
analysis_v2_runtime.snapshot() + {ir(file_id), type_at_position(file_id)}
      ↓
application::get_hover_info_with_semantic_program()
```

**2. Completion** (`type_system/services/completion_service.rs`):

```
LSP textDocument/completion
      ↓
bin/lsp_server/handlers/completion.rs
      ↓
analysis_v2_runtime.snapshot() + {ir(file_id), type_at_position(file_id)}
      ↓
application::get_completion_with_semantic_program_snapshot()
```

**3. Diagnostics** (v2, queries):

```
LSP textDocument/didOpen
      ↓
bin/lsp_server/handlers/text_document.rs
      ↓
analysis_v2_runtime.snapshot()
      ↓
analysis.syntax_diagnostics(file_id) + analysis.semantic_diagnostics(file_id)
      ↓
DTO/LSP Diagnostic
```

### Для Web API

**1. POST /api/hover/enhanced**:

```
Web API endpoint
      ↓
presentation/web/handlers.rs
      ↓
web_api_service + v2 snapshot
      ↓
JSON response
```

**2. POST /api/diagnostics**:

```
Web API endpoint
      ↓
presentation/web/handlers.rs
      ↓
v2 snapshot + diagnostics queries
      ↓
JSON response
```

## Взаимодействие со слоями

### С Domain Layer (shared crate)

```rust
use shared::engine::AnalysisEngine;
use shared::domain::{TypeResolver, TypeRepository};

// Application использует domain services
let analysis_engine = AnalysisEngine::new(type_repository.clone());
let result = analysis_engine.analyze_program(&ir_program)?;
```

### С System Layer

```rust
use crate::system::{SystemCoordinator, build_deps_bundle_v2};

// SystemCoordinator инициализирует TypeRepository/TypeResolver (startup),
// затем строится deps snapshot для IntelliSense v2.
let coordinator = SystemCoordinator::new();
coordinator.start().await?;
let deps_bundle = build_deps_bundle_v2(&coordinator, None, None)?;
```

### С Data Layer

```rust
use crate::data::loaders::{ConfigLoader, SyntaxHelperLoader};

// Application НЕ использует Data напрямую
// Data → Domain (TypeRepository) → Application (entrypoints/services)
```

## Ключевые файлы для изучения

**В порядке приоритета:**

1. **`type_system/mod.rs`** - начните здесь
   - Публичные entrypoints и ре-экспорты
   - Ссылки на `services/*`

2. **`semantic_validation_visitor/visitor.rs`** - семантические проверки
   - Как работает валидация
   - Какие ошибки детектируются

3. **`ast_to_ir/converter.rs`** - конвертация AST → IR
   - Как AST превращается в IR
   - Построение SymbolTable

4. **`type_system/services/hover_service.rs`** - пример LSP сервиса
   - Как предоставляется hover информация
   - Интеграция с TypeResolver

5. **`type_inference_service.rs`** - вывод типов
   - Как работает type inference
   - Градуальная типизация

## Добавление новой функции

### Пример: Добавить новый LSP сервис

**1. Создать service:**

```rust
// type_system/services/my_service.rs
pub struct MyService {
    type_repository: Arc<TypeRepository>,
}

impl MyService {
    pub fn provide_my_feature(&self, params: MyParams) -> Result<MyResult> {
        // Логика
    }
}
```

**2. Добавить публичный entrypoint:**

```rust
// type_system/mod.rs (ре-экспорт) или services/<...>.rs
pub async fn my_feature(/* deps + ir + params */) -> Result<MyResult> {
    // ...
}
```

**3. Создать LSP handler:**

```rust
// bin/lsp_server/handlers/my_feature.rs
pub fn handle_my_feature(
    params: lsp_types::MyParams
) -> Result<lsp_types::MyResult> {
    // Конвертация LSP types → internal types
    let internal_params = convert_params(params);

    // Вызов entrypoint (на v2 снапшоте)
    let result = my_feature(internal_params)?;

    // Конвертация internal types → LSP types
    Ok(convert_result(result))
}
```

### Пример: Добавить новую семантическую проверку

**1. Добавить ошибку:**

```rust
// shared/src/domain/semantic_error.rs
pub enum SemanticErrorKind {
    // ... existing errors
    MyNewError { details: String },
}
```

**2. Расширить visitor:**

```rust
// semantic_validation_visitor/visitor.rs
impl SemanticValidationVisitor {
    fn visit_my_node(&mut self, node: &SemanticNode) {
        if self.is_invalid(node) {
            self.errors.push(SemanticError {
                kind: SemanticErrorKind::MyNewError {
                    details: "...".to_string()
                },
                range: node.range,
            });
        }
    }
}
```

**3. Добавить тест:**

```rust
// backend/tests/context_diagnostics_lsp_test.rs
#[tokio::test]
async fn test_my_new_error() {
    let diagnostics = validate_code(r#"
        // Код, который должен вызвать ошибку
    "#);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("MyNewError"));
}
```

## Связанные документы

- [Backend README](../README.md) - обзор всего backend
- [Type System Architecture](../../../docs/architecture/type_system_architecture.md) - архитектура
- [Development Workflow](../../../docs/guides/development-workflow.md) - workflow разработки

## Статус

**Application Layer статус:**
- ✅ type_system entrypoints (v2-only path)
- ✅ SemanticValidationVisitor (MVP, Milestone 3.7)
- ✅ AstToIrConverter (реализован, Milestone 2.8)
- ✅ LSP Services: hover, completion, diagnostics, signature help
- 🚧 TypeInferenceService (в процессе улучшения)
