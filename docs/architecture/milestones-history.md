# Milestones History

История завершённых ключевых этапов развития BSL Gradual Type System.

## 📋 Навигация

**Актуальный roadmap:** [ROADMAP_2025.md](../../ROADMAP_2025.md)
**Архив всех Milestones:** [ROADMAP_ARCHIVE_2025.md](../../ROADMAP_ARCHIVE_2025.md) (13 завершённых этапов)

---

## ✨ Milestone 2.8: Semantic IR Layer

**Дата завершения:** Август 2024
**Статус:** ✅ Завершён

### Цель

Создание промежуточного представления (IR) программы, независимого от конкретного парсера.

### Ключевые концепции

**Проблема (до 2.8):**
- AnalysisEngine работал напрямую с tree-sitter AST
- Жёсткая зависимость от конкретного парсера
- Невозможно использовать альтернативные парсеры
- CLI (~2 MB) вынужден тянуть tree-sitter (~15 MB)

**Решение (после 2.8):**
```text
Раньше: AST → AnalysisEngine → TypeResolver
Теперь:  AST → IR (SemanticProgram) → AnalysisEngine → TypeResolver
```

### Компоненты

#### 1. SemanticProgram (shared/src/ir/)

Промежуточное представление программы:

```rust
pub struct SemanticProgram {
    pub nodes: Vec<SemanticNode>,       // Упрощённое дерево
    pub symbol_table: SymbolTable,       // Иерархия областей видимости
    pub source_map: SourceMap,           // Связь с исходным кодом
}
```

#### 2. SemanticNode

Упрощённый набор узлов (вместо сложного tree-sitter AST):

```rust
pub enum SemanticNode {
    Variable { name: String, type_hint: Option<TypeHint>, span: Span },
    Function { name: String, params: Vec<Param>, body: Vec<SemanticNode> },
    IfStatement { condition: Expr, then_branch: Vec<SemanticNode>, else_branch: Option<Vec<SemanticNode>> },
    // ... другие узлы
}
```

#### 3. Parser trait (shared/src/parsing/)

Dependency Inversion для парсеров:

```rust
pub trait Parser {
    fn parse(&self, source: &str) -> Result<SemanticProgram, ParseError>;
}
```

**Реализации:**
- `ParserCoordinator` (backend) — TreeSitter + Regex → IR
- `LightweightParser` (cli) — упрощённый парсер → IR (~2-3 MB)

#### 4. AstToIrConverter (backend/src/application/)

Мост между tree-sitter AST и SemanticProgram:

```rust
pub struct AstToIrConverter;

impl AstToIrConverter {
    pub fn convert(&self, ast: &Node) -> Result<SemanticProgram> {
        // Двухпроходная конвертация:
        // 1. Сбор символов в SymbolTable
        // 2. Конвертация узлов в SemanticNode
    }
}
```

### Преимущества

- ✅ **Независимость от парсера** — разные парсеры → единая IR
- ✅ **Лёгкий CLI** — LightweightParser вместо tree-sitter (~2 MB vs ~15 MB)
- ✅ **Переиспользование** — AnalysisEngine работает с любой IR
- ✅ **Тестируемость** — можно создавать SemanticProgram вручную для тестов

### Миграция

**Старый код:**
```rust
let ast = parser.parse(source)?;
let typed_program = engine.analyze_ast(&ast)?;
```

**Новый код:**
```rust
let parser: Box<dyn Parser> = Box::new(ParserCoordinator::new());
let ir = parser.parse(source)?;
let typed_program = engine.analyze_program(&ir)?;
```

### Ссылки

- [shared/src/ir/mod.rs](../../shared/src/ir/mod.rs) — Определение IR
- [backend/src/application/ast_to_ir.rs](../../backend/src/application/ast_to_ir.rs) — Конвертер AST → IR
- [shared/src/parsing/mod.rs](../../shared/src/parsing/mod.rs) — Parser trait

---

## 🎯 Milestone 2.9: Inline Scope Analysis

**Дата завершения:** Август 2024
**Статус:** ✅ Завершён

### Цель

Анализ типов локальных переменных "на лету" при hover, без загрузки runtime типов в TypeRepository.

### Концепция

**Ключевая идея:**
```text
LSP hover(file, line, column):
  1. Парсим файл → SemanticProgram (IR)
  2. Вызываем find_variable_at_position(line, column)
  3. Получаем (var_name, TypeHint) из scope
  4. Резолвим тип через TypeRepository (Platform/Config)
  5. Получаем методы/свойства через TypeMetadataLookup
  6. Возвращаем hover text
```

### Реализация

#### 1. find_variable_at_position() (shared/src/ir/)

Поиск переменной в scope hierarchy:

```rust
impl SemanticProgram {
    pub fn find_variable_at_position(&self, line: u32, column: u32)
        -> Option<(String, TypeHint)>
    {
        // Поиск в scope hierarchy (снизу вверх)
    }
}
```

#### 2. get_hover_info_ir() (backend/src/application/)

Inline Scope Analysis flow:

```rust
pub fn get_hover_info_ir(&self, file: &str, line: u32, column: u32)
    -> Result<HoverInfo>
{
    // 1. Парсим файл → SemanticProgram
    let ir = self.parser.parse(source)?;

    // 2. Ищем переменную в scope
    let (var_name, type_hint) = ir.find_variable_at_position(line, column)?;

    // 3. Резолвим тип через TypeRepository
    let type_metadata = self.repository.find_type(&type_hint.name)?;

    // 4. Получаем методы/свойства
    let methods = self.metadata_lookup.get_methods(&type_metadata)?;

    // 5. Формируем hover text
    Ok(HoverInfo { var_name, type_name, methods, certainty })
}
```

### Пример использования

```bsl
Процедура Тест()
    МассивДанных = Новый Массив();
    МассивДанных.Добавить(42);  // ← hover на "МассивДанных"
КонецПроцедуры
```

**Hover показывает:**
```
Переменная: МассивДанных
Тип: Массив
Уверенность: Known (100%)

Методы:
  • Добавить(Значение) — Добавляет элемент в конец массива
  • Количество() → Число — Возвращает количество элементов
  • Очистить() — Удаляет все элементы
  ... (ещё 12 методов)
```

### Преимущества

- ✅ **НЕ нужно управлять жизненным циклом** runtime типов
- ✅ **НЕ нужно** `load_runtime_types()` / `invalidate_runtime_types()`
- ✅ **SemanticProgram всегда актуальная** (парсится на каждый hover)
- ✅ **Работает в пределах одной процедуры/функции** (достаточно для базовой проверки!)

### Ограничения (приемлемы для MVP)

- ❌ НЕ работает межмодульный анализ (`рлф_ОбогащениеДанных.ОбогатитьСтруктуру`)
- ❌ НЕ отслеживается мутабельность (`Структура.Вставить`)
- ❌ НЕ работает flow-sensitive анализ (`Если x <> Неопределено`)

### Тестирование

- [backend/tests/inline_scope_analysis_test.rs](../../backend/tests/inline_scope_analysis_test.rs) — 5 интеграционных тестов
- [cli/test_inline_scope.bsl](../../cli/test_inline_scope.bsl) — тестовый файл для ручной проверки

---

## 🎯 Milestone 2.11: Tree-Sitter Span Extraction

**Дата завершения:** Сентябрь 2024
**Статус:** ✅ Завершён

### Цель

Извлечение реальных координат из tree-sitter узлов для корректной работы LSP hover.

### Проблема (до 2.11)

- Все `Span` в `SemanticNode` были фейковые (0, 0, 0, 0)
- `find_node_at_position(line, column)` всегда возвращал `None`
- Hover проваливался в fallback → одинаковая информация для всех переменных

### Решение (после 2.11)

```rust
// Tree-sitter предоставляет точные координаты
let span = Span::new(
    node.start_position().row as u32,        // start_line
    node.start_position().column as u32,     // start_column
    node.end_position().row as u32,          // end_line
    node.end_position().column as u32        // end_column
);
```

### Ключевые компоненты

#### 1. node_to_span() (backend/src/system/tree_sitter_adapter.rs)

Извлекает реальные координаты из tree-sitter:

```rust
fn node_to_span(node: &tree_sitter::Node) -> Span {
    Span::new(
        node.start_position().row as u32,
        node.start_position().column as u32,
        node.end_position().row as u32,
        node.end_position().column as u32,
    )
}
```

#### 2. ast_span_to_ir_span() (backend/src/application/ast_to_ir.rs)

Передаёт координаты в IR:

```rust
fn ast_span_to_ir_span(ast_span: &AstSpan) -> IrSpan {
    IrSpan::new(
        ast_span.start_line,
        ast_span.start_column,
        ast_span.end_line,
        ast_span.end_column,
    )
}
```

#### 3. get_hover_info() (backend/src/application/type_system_service.rs)

Использует реальные Span для поиска узлов:

```rust
pub fn get_hover_info(&self, file: &str, line: u32, column: u32)
    -> Result<HoverInfo>
{
    let ir = self.parser.parse(source)?;

    // Использует реальные Span для поиска
    let node = ir.find_node_at_position(line, column)?;

    // ...
}
```

### Результат

- ✅ **0 использований** `Span::stub()` в production коде (только в тестовых данных)
- ✅ **`find_node_at_position()` корректно находит** узлы по позиции курсора
- ✅ **Hover показывает разную информацию** для разных переменных
- ✅ **DEBUG логи отслеживают** Span extraction в реальном времени

### Тестирование

[backend/tests/hover_with_spans_test.rs](../../backend/tests/hover_with_spans_test.rs) — 6 интеграционных тестов:

1. ✅ Hover на переменной в объявлении
2. ✅ Hover на переменной при использовании
3. ✅ Hover показывает разную информацию для разных переменных
4. ✅ Hover на параметре функции
5. ✅ Hover на имени метода
6. ✅ Корректность `Span.contains(line, column)`

### Отладка

DEBUG логи:
- `tree_sitter_adapter.rs` — извлечённые Span из tree-sitter
- `ast_to_ir.rs` — конвертация AST Span → IR Span
- `type_system_service.rs` — результат поиска узлов

---

## 🎯 Configuration Type Certainty (Исправление 2025-01-18)

**Дата:** 18 января 2025
**Статус:** ✅ Исправлено

### Проблема

Функция `resolve_member_access()` в TypeResolver **ВСЕГДА возвращала** `Certainty::Inferred(0.8)` (80%) для конфигурационных типов (Справочники.*, Документы.*), даже если метаданные конфигурации не загружены.

### Пример проблемы

```bsl
СправочникКонтрагенты = Справочники.Контрагенты;
// ❌ БЫЛО: Hover показывал "🟡 Inferred (80%)"
// Но метаданных конфигурации нет → ложная уверенность!
```

### Решение

Честная оценка certainty на основе наличия метаданных в TypeRepository:

| Ситуация | Certainty | Пример |
|----------|-----------|--------|
| **Metadata найдена** | `Known (100%)` | Типы платформы из Syntax Helper |
| **Только синтаксис** | `Inferred (50%)` | Справочники.* без загруженной конфигурации |
| **Неизвестный тип** | `Unknown (0%)` | Опечатки, несуществующие типы |

### Код исправления

[shared/src/domain/resolver.rs:124-136](../../shared/src/domain/resolver.rs):

```rust
// ✅ ИСПРАВЛЕНИЕ: Проверяем наличие метаданных для честного certainty
let type_name = format!("{}.{}", prefix, member);
let has_metadata = self.repository.find_type(&type_name).is_some();

// Определяем уровень уверенности:
// - Known (100%) - тип найден в метаданных конфигурации
// - Inferred (50%) - только синтаксис распарсили, метаданных нет
let (certainty, source) = if has_metadata {
    (Certainty::Known, ResolutionSource::Static)
} else {
    (Certainty::Inferred(0.5), ResolutionSource::Inferred)
};
```

### Теперь hover показывает

```
Переменная: СправочникКонтрагенты
Тип: Справочники.Контрагенты
Уверенность: 🟡 Inferred (50%)  ← ЧЕСТНО!
⚠️ **Детали типа недоступны**

💡 Возможные причины:
• Тип не загружен из Syntax Helper
• Требуется парсинг документации платформы
```

### Затронутые компоненты

- `shared/src/domain/resolver.rs` — логика TypeResolver
- `backend/tests/hover_unknown_type_test.rs` — обновлены assertions
- LSP hover, Web API, CLI — все используют новую логику certainty

### Тестирование

```bash
# Все тесты проходят
cargo test -p bsl-backend --test hover_unknown_type_test  # 3/3 passed
cargo test -p bsl-shared resolver                         # 43/43 passed
```

### Code Review

Reviewer оценил изменение на **9.2/10** ⭐
Архитектурная корректность: **10/10**

---

## 🚨 Milestone 2.18: LSP Syntax Error Diagnostics

**Дата:** Планируется
**Статус:** ⚠️ В разработке

### Текущее состояние (2025-01-18)

Tree-sitter парсер **УСПЕШНО обнаруживает** синтаксические ошибки, но они **НЕ отображаются** пользователю в VSCode.

### Что работает

- ✅ `tree_sitter_adapter.rs` обнаруживает ERROR узлы
- ✅ `tree_sitter_adapter.rs` обнаруживает отсутствующие токены (`node.is_missing()`)
- ✅ `ParseResult.syntax_errors` содержит список ошибок с координатами
- ✅ Логирование ошибок в `rust_lsp_server.log`

### Что НЕ работает

- ❌ LSP Server НЕ передаёт ошибки в `publish_diagnostics`
- ❌ Пользователь НЕ видит красные волнистые линии в VSCode
- ❌ Diagnostics panel пуст

### Пример обнаруженной ошибки

[backend/tests/syntax_error_detection_test.rs](../../backend/tests/syntax_error_detection_test.rs):

```rust
let source = r#"
Функция Тест()
    Если Истина Тогда
        Сообщить("Привет");
    // Отсутствует КонецЕсли!
    Возврат;
КонецФункции
"#;

let parse_result = parser.parse(source).unwrap();

// ✅ РАБОТАЕТ: Ошибка обнаружена
assert!(parse_result.has_errors());
assert_eq!(parse_result.syntax_errors.len(), 1);

// ✅ РАБОТАЕТ: Детали ошибки доступны
let error = &parse_result.syntax_errors[0];
assert_eq!(error.error_type, ErrorType::MissingToken);
assert!(error.message.contains("ENDIF_KEYWORD"));

// Позиция: строка 8, колонка 38
```

### Типы синтаксических ошибок

[backend/src/parsing/bsl/mod.rs:48-56](../../backend/src/parsing/bsl/mod.rs):

```rust
pub enum ErrorType {
    UnexpectedToken,  // Неожиданный токен
    MissingToken,     // Отсутствующий токен (незакрытая конструкция)
    InvalidSyntax,    // Неверная структура
    ParseError,       // Общая ошибка парсинга
}
```

### Компоненты обнаружения ошибок

| Компонент | Файл | Статус | Функция |
|-----------|------|--------|---------|
| **TreeSitterAdapter** | `backend/src/system/tree_sitter_adapter.rs` | ✅ Работает | Обнаруживает ERROR узлы и missing tokens |
| **ParseResult** | `backend/src/parsing/bsl/mod.rs` | ✅ Работает | Хранит синтаксические ошибки |
| **ParserCoordinator** | `backend/src/system/parser_coordinator.rs` | ✅ Работает | Логирует ошибки в файл |
| **LSP diagnostics** | `backend/src/bin/lsp_server.rs` | ❌ НЕ работает | `publish_diagnostics` получает пустой массив |

### Тестирование

[backend/tests/syntax_error_detection_test.rs](../../backend/tests/syntax_error_detection_test.rs) — 4 интеграционных теста:

1. ✅ Незакрытый Если
2. ✅ Отсутствующий КонецЦикла
3. ✅ Множественные ошибки
4. ✅ Корректная обработка ERROR узлов

Все тесты проходят — ошибки обнаруживаются корректно.

### Пример лога

`rust_lsp_server.log`:

```
WARN tree_sitter_adapter: ⚠️ Обнаружены синтаксические ошибки при парсинге:
WARN tree_sitter_adapter:   - [8:38-8:38] Отсутствует обязательный элемент: ENDIF_KEYWORD
```

### Планируемое исправление

**Milestone 2.18 (в roadmap):**
Добавить конвертацию `ParseError` → LSP `Diagnostic` для отображения красных волнистых линий в VSCode.

**Ожидаемый результат:**
```
Пользователь увидит в VSCode:
  | 4 | // Отсутствует КонецЕсли!
  | 5 | Возврат;
        ~~~~~~~~~~ ❌ Ошибка: Отсутствует обязательный элемент: КонецЕсли
```

### Ссылки

- [ROADMAP_2025.md:500-714](../../ROADMAP_2025.md) — Milestone 2.18 описание
- [backend/tests/syntax_error_detection_test.rs](../../backend/tests/syntax_error_detection_test.rs) — тесты обнаружения

---

## 🎯 Milestone 2.16: Semantic Tree Visualization

**Дата завершения:** Январь 2025
**Статус:** ✅ MVP завершён (заглушка API контракта)

### Цель

HTTP endpoint для получения семантического дерева BSL модулей в JSON/HTML форматах.

### Компоненты

#### 1. SemanticRoutes (backend/src/presentation/)

Axum router для semantic visualization API:

```rust
pub fn semantic_routes() -> Router {
    Router::new()
        .route("/api/semantic/:file_path", get(get_semantic_tree))
}
```

#### 2. Endpoints

**GET** `/api/semantic/:file_path?format=json|html&theme=dark|light&compact=false`

**Параметры:**
- `file_path` — путь к BSL файлу
- `format` — `json` (по умолчанию) или `html`
- `theme` — `dark` (по умолчанию) или `light` (только для HTML)
- `compact` — `false` (по умолчанию) или `true` (упрощённый вывод)

### Интеграция

- **Web Server:** прямой вызов через маршрутизацию
- **LSP Server:** custom request `bsl/getSemanticHtml` для VSCode Extension

### Статус

⚠️ **MVP stub** — заглушка для тестирования API контракта.
Возвращает заготовленные данные, реальный парсинг планируется в будущем.

### Тестирование

[backend/tests/semantic_visualization_test.rs](../../backend/tests/semantic_visualization_test.rs) — 3 интеграционных теста:

1. ✅ JSON формат возвращается корректно
2. ✅ HTML формат возвращается корректно
3. ✅ Параметры theme и compact обрабатываются

### Использование

```bash
# JSON формат
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=json" | jq '.'

# HTML формат с темной темой
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=html&theme=dark" > semantic_tree.html
start semantic_tree.html
```

---

## 📚 Полный список Milestones

**См. также:**
- **[ROADMAP_2025.md](../../ROADMAP_2025.md)** — актуальный план с компактным списком завершённых этапов
- **[ROADMAP_ARCHIVE_2025.md](../../ROADMAP_ARCHIVE_2025.md)** — полный архив всех 13 завершённых Milestones с детальными описаниями

**Прогресс Версии 2.0:** ~65% завершено (13 Milestones из ~20 планируемых)
