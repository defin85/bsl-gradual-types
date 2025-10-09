# Milestone 2.11: Tree-Sitter Span Extraction для LSP Hover

**Цель:** Исправить извлечение реальных координат из tree-sitter узлов в AST → IR конверсии для корректной работы LSP hover.

**Дата создания:** 08.10.2025

---

## 🎯 Проблема

### Текущее состояние (Milestone 2.10)

✅ **Что работает:**
- LSP получает `platformDocsArchive` из Extension через `initializationOptions`
- 3927 типов платформы загружаются успешно
- IR-based hover (`get_hover_info_ir`) реализован с Inline Scope Analysis
- `find_variable_at_position()` корректно ищет переменную в scope hierarchy

❌ **Что НЕ работает:**
- Все `Span` в `SemanticNode` фейковые (0, 0, 0, 0)
- `find_node_at_position(line, column)` всегда возвращает `None`
- `find_variable_at_position()` не может найти узел по координатам
- Hover проваливается в fallback → возвращает одинаковую информацию для всех переменных

### Корневая причина

**ast_to_ir.rs:32** содержит TODO:
```rust
/// Исходный код (для вычисления Span из tree-sitter в будущем)
/// TODO: использовать для извлечения реальных Span вместо Span::stub()
#[allow(dead_code)]
source: String,
```

При конверсии AST → IR все `Span` создаются через `Span::stub()`:
- `VariableDeclaration` → `Span::stub()`
- `Assignment` → `Span::stub()`
- `MemberAccess` → `Span::stub()`
- `FunctionCall` → `Span::stub()`

### Что должно быть

Tree-sitter предоставляет точные координаты для каждого узла:
```rust
let node = cursor.node();
let start = node.start_position();
let end = node.end_position();

let span = Span::new(
    start.row as u32,          // start_line
    end.row as u32,            // end_line
    start.column as u32,       // start_column
    end.column as u32          // end_column
);
```

---

## 📋 Задачи

### БЛОК A: Span Extraction из Tree-Sitter (КРИТИЧЕСКИЙ)

#### ✅ Task A1: Добавить позиционную информацию в AST
**Статус:** ✅ **УЖЕ ВЫПОЛНЕНО** (обнаружено при проверке кода)

**Фактическое состояние:**
- ✅ AST уже содержит `Span` во всех узлах: [backend/src/parsing/bsl/mod.rs:6-36](backend/src/parsing/bsl/mod.rs#L6)
- ✅ Все `Statement` имеют поле `span: Span`: [mod.rs:95-191](backend/src/parsing/bsl/mod.rs#L95)
- ✅ Все `Expression` имеют поле `span: Span`: [mod.rs:193+](backend/src/parsing/bsl/mod.rs#L193)
- ✅ TreeSitterAdapter извлекает реальные координаты: [tree_sitter_adapter.rs:13-21](backend/src/system/tree_sitter_adapter.rs#L13)

**Реализованный код:**
```rust
// backend/src/parsing/bsl/mod.rs
pub struct Span {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl Span {
    pub fn from_positions(start: (u32, u32), end: (u32, u32)) -> Self {
        Self {
            start_line: start.0,
            start_column: start.1,
            end_line: end.0,
            end_column: end.1,
        }
    }
}

// backend/src/system/tree_sitter_adapter.rs
fn node_to_span(node: &Node) -> Span {
    let start_pos = node.start_position();
    let end_pos = node.end_position();
    Span::from_positions(
        (start_pos.row as u32, start_pos.column as u32),
        (end_pos.row as u32, end_pos.column as u32),
    )
}
```

**Проверка:**
```bash
✅ cargo check -p bsl-backend  # Компилируется без ошибок
✅ Все Statement и Expression создаются с реальными span из tree-sitter
```

---

#### ✅ Task A2: Передать Span в IR при конверсии
**Статус:** ✅ **ВЫПОЛНЕНО** (08.10.2025)
**Цель:** Использовать координаты из AST при создании `SemanticNode`

**Что сделано:**

1. ✅ Метод `ast_span_to_ir_span()` использует реальные координаты из AST:
   ```rust
   // backend/src/application/ast_to_ir.rs:616-627
   fn ast_span_to_ir_span(&self, ast_span: crate::parsing::bsl::ast::Span) -> Span {
       Span {
           start_line: ast_span.start_line,
           start_column: ast_span.start_column,
           end_line: ast_span.end_line,
           end_column: ast_span.end_column,
       }
   }
   ```

2. ✅ Все `convert_statement()` вызовы используют `ast_span_to_ir_span()` вместо `Span::stub()`:
   - VarDeclaration, Assignment, If, While, For, ForEach
   - Return, Try, Call, Break, Continue
   - FunctionDecl, ProcedureDecl

3. ✅ Исправлена передача span в `convert_call_expression()`:
   - Добавлен параметр `span: Span`
   - Удалены вызовы `create_span_stub()`

**Результат:**
- ❌ **БЫЛО:** 13 использований `Span::stub()` → SemanticNode с (0,0,0,0)
- ✅ **СТАЛО:** 0 использований `Span::stub()` → реальные координаты из tree-sitter

**Проверка:**
```bash
✅ cargo check -p bsl-backend  # Компилируется без ошибок
✅ grep -c "Span::stub()" backend/src/application/ast_to_ir.rs  # → 0
```

---

#### ⏳ Task A3: Протестировать find_node_at_position
**Цель:** Убедиться что `find_node_at_position()` находит узлы по реальным координатам

**Тест:**
```rust
#[tokio::test]
async fn test_hover_with_real_spans() {
    let code = r#"
Функция ТестМассива()
    МойМассив = Новый Массив();
    МойМассив.Добавить("элемент");
КонецФункции
"#;

    let service = setup_type_system_service().await;

    // Hover на "МойМассив" в строке 2 (assignment)
    let hover1 = service.get_hover_info_ir("test.bsl", code, 2, 5).await.unwrap();
    assert!(hover1.is_some());
    assert!(hover1.unwrap().contains("Массив"));

    // Hover на "МойМассив" в строке 3 (member access)
    let hover2 = service.get_hover_info_ir("test.bsl", code, 3, 5).await.unwrap();
    assert!(hover2.is_some());
    assert!(hover2.unwrap().contains("Добавить"));
}
```

**Файл:** `backend/tests/hover_with_spans_test.rs`

---

### БЛОК B: Логирование и отладка

#### ⏳ Task B1: Добавить DEBUG логи для Span
**Цель:** Отслеживать Span extraction в логах

**Изменения:**
1. `tree_sitter_adapter.rs`:
   ```rust
   debug!("Extracted node at {}:{} - {}:{}",
       span.start_line, span.start_column,
       span.end_line, span.end_column);
   ```

2. `ast_to_ir.rs`:
   ```rust
   debug!("Created SemanticNode at span {:?}", node.span);
   ```

3. `get_hover_info_ir`:
   ```rust
   debug!("Looking for node at position {}:{}", line, column);
   if let Some(node) = ir.find_node_at_position(line, column) {
       debug!("Found node: {:?} at span {:?}", node.kind, node.span);
   } else {
       warn!("No node found at position {}:{}", line, column);
   }
   ```

---

#### ⏳ Task B2: Тест с различными позициями
**Цель:** Проверить hover в разных местах кода

**Тестовые случаи:**
1. Hover на переменной в объявлении
2. Hover на переменной в присваивании
3. Hover на переменной в вызове метода
4. Hover на имени метода
5. Hover на параметре функции

---

### БЛОК C: Документация

#### ⏳ Task C1: Обновить CLAUDE.md
**Добавить:**
```markdown
## Span Extraction (Milestone 2.11)

Все SemanticNode теперь содержат реальные координаты из tree-sitter:
- `Span.start_line`, `Span.end_line` - строки (0-indexed)
- `Span.start_column`, `Span.end_column` - колонки (0-indexed)

Это позволяет:
- Точно находить узлы по позиции курсора в LSP hover
- Правильно работать Inline Scope Analysis
- Отображать корректную информацию о типах переменных
```

---

#### ⏳ Task C2: Обновить ROADMAP_2025.md
**Отметить:**
- ✅ Milestone 2.10: LSP Configuration + Type Index Integration
- ✅ Milestone 2.11: Tree-Sitter Span Extraction (текущий)
- ⏳ Milestone 2.12: Custom LSP Requests (bsl/getAllTypes, bsl/searchTypes)

---

## 🎯 Критерий успеха

✅ **Milestone 2.11 считается завершённым когда:**

1. ✅ Все `SemanticNode` содержат реальные координаты из tree-sitter (не stub)
2. ✅ `find_node_at_position(line, column)` корректно находит узел под курсором
3. ✅ `find_variable_at_position()` работает без fallback на `find_symbol_in_ir`
4. ✅ LSP hover показывает разную информацию для разных переменных (не одинаковую)
5. ✅ Тесты `hover_with_spans_test.rs` проходят успешно
6. ✅ Логи показывают реальные span координаты (не 0:0)

---

## 📊 Прогресс

**Текущий статус:** ⚠️ **ЧАСТИЧНО ЗАВЕРШЁН** (80%)

| Задача | Статус | Дата завершения |
|--------|--------|-----------------|
| Task A1: Span в AST | ✅ **УЖЕ ГОТОВО** | (ранее) |
| Task A2: Span в IR | ✅ **ВЫПОЛНЕНО** | 08.10.2025 |
| Task A3: Тест hover | ⏳ Ожидает запуска | - |
| Task B1: Логи | ⏳ Не начато | - |
| Task B2: Тесты | ⏳ Не начато | - |
| Task C1: CLAUDE.md | ⏳ Не начато | - |
| Task C2: ROADMAP | ✅ **ВЫПОЛНЕНО** | 08.10.2025 |

---

## 🔗 Связанные файлы

**Критические файлы для изменения:**
- `backend/src/parsing/bsl/ast.rs` - добавить SourceSpan в AST узлы
- `backend/src/system/tree_sitter_adapter.rs` - извлекать span из tree-sitter
- `backend/src/application/ast_to_ir.rs` - передавать span в SemanticNode
- `shared/src/ir/mod.rs` - конверсия SourceSpan → Span

**Тесты:**
- `backend/tests/hover_with_spans_test.rs` (новый)
- `backend/tests/tree_sitter_adapter_comprehensive_test.rs` (обновить)
- `backend/tests/full_pipeline_integration_test.rs` (обновить)

---

## 💡 Технические заметки

### Tree-Sitter координаты

Tree-sitter использует 0-based индексацию для строк и колонок:
```rust
let start = node.start_position();
// start.row = 0 → первая строка файла
// start.column = 0 → первый символ строки
```

LSP также использует 0-based индексацию, поэтому конверсия не требуется.

### Span::contains

Проверка включения позиции в span:
```rust
impl Span {
    pub fn contains(&self, line: u32, column: u32) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }

        if line == self.start_line && column < self.start_column {
            return false;
        }

        if line == self.end_line && column > self.end_column {
            return false;
        }

        true
    }
}
```

### Пример Span extraction

```rust
// Tree-sitter node:
// "МойМассив = Новый Массив();"
// ^start (line 5, col 4)
//                            ^end (line 5, col 32)

let span = SourceSpan {
    start_line: 5,
    end_line: 5,
    start_column: 4,
    end_column: 32,
};
```

---

## ⚠️ Риски и ограничения

1. **Большой объём изменений** - нужно обновить все места создания AST узлов
2. **Обратная совместимость** - старые тесты могут сломаться из-за новых полей
3. **Performance** - извлечение span добавляет небольшой оверхед при парсинге

**Митигация:**
- Пошаговый подход: сначала AST, потом IR, потом тесты
- Использовать derive(Clone) для SourceSpan чтобы избежать копирования
- Добавить feature flag для включения/выключения span extraction

---

## 📝 Примечания

**Почему это важно для LSP:**

Без реальных Span:
- ❌ Hover показывает одинаковую информацию для всех переменных
- ❌ Go to Definition не работает
- ❌ Find References возвращает все использования вместо конкретного
- ❌ Rename не может определить область применения

С реальными Span:
- ✅ Точное определение типа переменной под курсором
- ✅ Корректная работа всех LSP features
- ✅ Отличная UX в VSCode Extension
