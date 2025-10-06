# 🔧 Milestone 2.7: TreeSitterAdapter — Детальный план реализации

**Дата анализа:** 2025-10-06
**Статус:** 🚨 КРИТИЧЕСКИЙ — блокирует все LSP features
**Длительность:** 2 недели (10 рабочих дней)
**Приоритет:** #1

---

## 📊 Текущее состояние

### ✅ Что уже работает (30% готовности)

**Реализованные node types в TreeSitterAdapter:**

| Node Type | Статус | Качество | Комментарий |
|-----------|--------|----------|-------------|
| `function_definition` | ✅ | 80% | Базовая реализация, нет async/export |
| `procedure_definition` | ✅ | 80% | Использует function_definition |
| `var_definition` | ✅ | 90% | Полная реализация |
| `var_statement` | ✅ | 90% | Использует var_definition |
| `if_statement` | ✅ | 70% | Работает, но elseif как вложенный if |
| `assignment_statement` | ✅ | 85% | Работает корректно |
| `call_statement` | ⚠️ | 50% | Преобразуется в Assignment (workaround) |
| `return_statement` | ⚠️ | 40% | Преобразуется в Assignment (workaround) |
| `for_statement` | ⚠️ | 30% | Заглушка — возвращает пустой If |
| `for_each_statement` | ⚠️ | 30% | Использует for_statement (заглушка) |
| | | | |
| **Expressions:** | | | |
| `identifier` | ✅ | 100% | Полная поддержка |
| `const_expression` | ✅ | 90% | Number, Boolean, String |
| `binary_expression` | ✅ | 75% | Базовые операторы (+, -, *) |
| `unary_expression` | ✅ | 70% | NOT, - |
| `call_expression` | ✅ | 80% | Вызов функций работает |
| `method_call` | ✅ | 80% | Использует call_expression |
| `property_access` | ⚠️ | 60% | Преобразуется в Identifier (упрощённо) |

**Текущий AST (backend/src/parsing/bsl/mod.rs):**

```rust
pub enum Statement {
    Assignment { target: Expression, value: Expression },
    VarDeclaration { name: String, type_hint: Option<String> },
    FunctionDecl { name: String, params: Vec<String>, body: Vec<Statement> },
    ProcedureDecl { name: String, params: Vec<String>, body: Vec<Statement> },
    If { condition: Expression, then_body: Vec<Statement>, else_body: Option<Vec<Statement>> },
    For { variable: String, start: Expression, end: Expression, body: Vec<Statement> }, // ✅ Есть!
    While { condition: Expression, body: Vec<Statement> }, // ✅ Есть!
    Return { value: Option<Expression> }, // ✅ Есть!
    Try { try_body: Vec<Statement>, except_body: Vec<Statement> }, // ✅ Есть!
    Call { expression: Expression }, // ✅ Есть!
}

pub enum Expression {
    Identifier(String),
    String(String),
    Number(f64),
    Boolean(bool),
    Call { function: Box<Expression>, args: Vec<Expression> },
    Binary { left: Box<Expression>, operator: String, right: Box<Expression> },
    Unary { operator: String, operand: Box<Expression> },
}
```

**Вывод:** AST структуры УЖЕ ЕСТЬ для большинства конструкций! Adapter просто НЕ ИСПОЛЬЗУЕТ их.

---

### ❌ Что НЕ работает (70% отсутствует)

**Отсутствующие node types (21 штука):**

#### 1️⃣ Statements (11 пропущенных):
- ❌ `while_statement` — **AST ЕСТЬ!** Нужно только подключить
- ❌ `try_statement` — **AST ЕСТЬ!** Нужно только подключить
- ❌ `break_statement` — AST нет, нужно добавить `Statement::Break`
- ❌ `continue_statement` — AST нет, нужно добавить `Statement::Continue`
- ❌ `goto_statement` — AST нет, нужно добавить `Statement::Goto { label: String }`
- ❌ `label_statement` — AST нет, нужно добавить `Statement::Label { name: String }`
- ❌ `execute_statement` — AST нет, нужно добавить `Statement::Execute { code: Expression }`
- ❌ `rise_error_statement` — AST нет, нужно добавить `Statement::RaiseError { message: Expression }`
- ❌ `add_handler_statement` — AST нет, нужно добавить `Statement::AddHandler { event, handler }`
- ❌ `remove_handler_statement` — AST нет, нужно добавить `Statement::RemoveHandler { event, handler }`
- ❌ `await_statement` — AST нет, нужно добавить `Statement::Await { expression }`

#### 2️⃣ Expressions (3 пропущенных):
- ❌ `ternary_expression` — AST нет, нужно добавить `Expression::Ternary { condition, then_expr, else_expr }`
- ❌ `new_expression` — AST нет, нужно добавить `Expression::New { type_name, args }`
- ❌ `await_expression` — AST нет, нужно добавить `Expression::Await { expression }`

#### 3️⃣ Literals (2 пропущенных):
- ❌ `date` — AST нет, нужно добавить `Expression::Date(String)`
- ❌ `string` (multiline) — Текущий `Expression::String` может не различать обычные и многострочные

#### 4️⃣ Вспомогательные структуры (5 пропущенных):
- ❌ `else_clause` — **РЕАЛИЗОВАНО!** Но нужна проверка
- ❌ `elseif_clause` — Реализовано как вложенный if (можно улучшить)
- ❌ `parameters` — **РЕАЛИЗОВАНО!**
- ❌ `parameter` — **РЕАЛИЗОВАНО!**
- ❌ `preprocessor` — Пропускается (корректно для MVP)

---

## 🎯 Детальный план реализации

### 📅 День 1-2: Расширение AST структур (2 дня)

**Задача:** Добавить недостающие варианты в enum Statement и Expression

**Файл:** `backend/src/parsing/bsl/mod.rs`

**Новые Statement варианты:**

```rust
pub enum Statement {
    // ✅ Уже есть
    Assignment { target: Expression, value: Expression },
    VarDeclaration { name: String, type_hint: Option<String> },
    FunctionDecl { name: String, params: Vec<String>, body: Vec<Statement> },
    ProcedureDecl { name: String, params: Vec<String>, body: Vec<Statement> },
    If { condition: Expression, then_body: Vec<Statement>, else_body: Option<Vec<Statement>> },
    For { variable: String, start: Expression, end: Expression, body: Vec<Statement> },
    While { condition: Expression, body: Vec<Statement> },
    Return { value: Option<Expression> },
    Try { try_body: Vec<Statement>, except_body: Vec<Statement> },
    Call { expression: Expression },

    // ➕ ДОБАВИТЬ:
    ForEach {
        variable: String,
        collection: Expression,
        body: Vec<Statement>
    },
    Break,
    Continue,
    Goto {
        label: String
    },
    Label {
        name: String
    },
    Execute {
        code: Expression
    },
    RaiseError {
        message: Option<Expression>
    },
    AddHandler {
        event: Expression,
        handler: Expression
    },
    RemoveHandler {
        event: Expression,
        handler: Expression
    },
    Await {
        expression: Expression
    },
}
```

**Новые Expression варианты:**

```rust
pub enum Expression {
    // ✅ Уже есть
    Identifier(String),
    String(String),
    Number(f64),
    Boolean(bool),
    Call { function: Box<Expression>, args: Vec<Expression> },
    Binary { left: Box<Expression>, operator: String, right: Box<Expression> },
    Unary { operator: String, operand: Box<Expression> },

    // ➕ ДОБАВИТЬ:
    Ternary {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>
    },
    New {
        type_name: String,
        args: Vec<Expression>
    },
    PropertyAccess {
        object: Box<Expression>,
        property: String
    },
    IndexAccess {
        object: Box<Expression>,
        index: Box<Expression>
    },
    Date(String), // Сохраняем как строку для простоты
    Await {
        expression: Box<Expression>
    },
}
```

**Метрика успеха:**
- ✅ `cargo check` проходит без ошибок
- ✅ Все существующие тесты компилируются (даже если не проходят)

---

### 📅 День 3-5: Реализация конвертации простых statements (3 дня)

**Задача:** Подключить уже существующие AST варианты и добавить простые новые

**Файл:** `backend/src/system/tree_sitter_adapter.rs`

#### 3.1. Исправить существующие workarounds

**ПРИОРИТЕТ #1:** Использовать реальные AST варианты вместо заглушек

```rust
// ❌ БЫЛО (строка 207):
fn convert_for_statement(_node: &Node, _source: &str) -> Result<Statement, String> {
    Ok(Statement::If { // ← ЗАГЛУШКА!
        condition: Expression::Boolean(true),
        then_body: vec![],
        else_body: None,
    })
}

// ✅ ДОЛЖНО БЫТЬ:
fn convert_for_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut variable = String::new();
    let mut start = Expression::Number(0.0);
    let mut end = Expression::Number(0.0);
    let mut body = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if variable.is_empty() {
                    variable = Self::node_text(&child, source);
                }
            }
            "expression" => {
                // Первое expression = start, второе = end
                if let Some(expr) = Self::convert_expression(&child, source)? {
                    if start == Expression::Number(0.0) {
                        start = expr;
                    } else {
                        end = expr;
                    }
                }
            }
            _ => {
                // Собираем тело цикла
                if let Some(stmt) = Self::convert_statement(&child, source)? {
                    body.push(stmt);
                }
            }
        }
    }

    Ok(Statement::For { variable, start, end, body })
}
```

```rust
// ❌ БЫЛО (строка 240):
fn convert_return(_node: &Node, _source: &str) -> Result<Statement, String> {
    Ok(Statement::Assignment { // ← ЗАГЛУШКА!
        target: Expression::Identifier("__return".to_string()),
        value: Expression::Boolean(true),
    })
}

// ✅ ДОЛЖНО БЫТЬ:
fn convert_return(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut value = None;

    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            value = Some(expr);
            break;
        }
    }

    Ok(Statement::Return { value })
}
```

```rust
// ❌ БЫЛО (строка 250):
fn convert_call_statement(node: &Node, source: &str) -> Result<Statement, String> {
    // ...
    Ok(Statement::Assignment { // ← ЗАГЛУШКА!
        target: Expression::Identifier("__call".to_string()),
        value: expr,
    })
}

// ✅ ДОЛЖНО БЫТЬ:
fn convert_call_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            return Ok(Statement::Call { expression: expr });
        }
    }

    Err("call_statement without expression".to_string())
}
```

#### 3.2. Добавить простые statements

**while_statement:**
```rust
"while_statement" => Ok(Some(Self::convert_while_statement(node, source)?)),

fn convert_while_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut condition = Expression::Boolean(true);
    let mut body = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "expression" => {
                if let Some(expr) = Self::convert_expression(&child, source)? {
                    condition = expr;
                }
            }
            _ => {
                if let Some(stmt) = Self::convert_statement(&child, source)? {
                    body.push(stmt);
                }
            }
        }
    }

    Ok(Statement::While { condition, body })
}
```

**try_statement:**
```rust
"try_statement" => Ok(Some(Self::convert_try_statement(node, source)?)),

fn convert_try_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut try_body = Vec::new();
    let mut except_body = Vec::new();
    let mut in_except = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "EXCEPT_KEYWORD" => in_except = true,
            _ => {
                if let Some(stmt) = Self::convert_statement(&child, source)? {
                    if in_except {
                        except_body.push(stmt);
                    } else {
                        try_body.push(stmt);
                    }
                }
            }
        }
    }

    Ok(Statement::Try { try_body, except_body })
}
```

**for_each_statement:**
```rust
"for_each_statement" => Ok(Some(Self::convert_for_each_statement(node, source)?)),

fn convert_for_each_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut variable = String::new();
    let mut collection = Expression::Identifier("unknown".to_string());
    let mut body = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if variable.is_empty() {
                    variable = Self::node_text(&child, source);
                }
            }
            "expression" => {
                if let Some(expr) = Self::convert_expression(&child, source)? {
                    collection = expr;
                }
            }
            _ => {
                if let Some(stmt) = Self::convert_statement(&child, source)? {
                    body.push(stmt);
                }
            }
        }
    }

    Ok(Statement::ForEach { variable, collection, body })
}
```

**break_statement, continue_statement:**
```rust
"break_statement" => Ok(Some(Statement::Break)),
"continue_statement" => Ok(Some(Statement::Continue)),
```

**Метрика успеха:**
- ✅ For/While/Try/ForEach корректно парсятся из реального BSL кода
- ✅ Return и Call больше не используют Assignment workaround
- ✅ Unit-тесты для каждого statement type

---

### 📅 День 6-7: Реализация сложных statements (2 дня)

**Задача:** Добавить специфичные для 1С конструкции

#### 6.1. goto_statement и label_statement

```rust
"goto_statement" => Ok(Some(Self::convert_goto_statement(node, source)?)),
"label_statement" => Ok(Some(Self::convert_label_statement(node, source)?)),

fn convert_goto_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let label = Self::node_text(&child, source);
            return Ok(Statement::Goto { label });
        }
    }
    Err("goto_statement without label".to_string())
}

fn convert_label_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let name = Self::node_text(&child, source);
            return Ok(Statement::Label { name });
        }
    }
    Err("label_statement without name".to_string())
}
```

#### 6.2. execute_statement

```rust
"execute_statement" => Ok(Some(Self::convert_execute_statement(node, source)?)),

fn convert_execute_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            return Ok(Statement::Execute { code: expr });
        }
    }
    Err("execute_statement without code".to_string())
}
```

#### 6.3. rise_error_statement

```rust
"rise_error_statement" => Ok(Some(Self::convert_raise_error_statement(node, source)?)),

fn convert_raise_error_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    let mut message = None;

    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            message = Some(expr);
            break;
        }
    }

    Ok(Statement::RaiseError { message })
}
```

#### 6.4. add_handler_statement и remove_handler_statement

```rust
"add_handler_statement" => Ok(Some(Self::convert_add_handler_statement(node, source)?)),
"remove_handler_statement" => Ok(Some(Self::convert_remove_handler_statement(node, source)?)),

fn convert_add_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let (event, handler) = Self::extract_event_handler_pair(node, source)?;
    Ok(Statement::AddHandler { event, handler })
}

fn convert_remove_handler_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let (event, handler) = Self::extract_event_handler_pair(node, source)?;
    Ok(Statement::RemoveHandler { event, handler })
}

fn extract_event_handler_pair(node: &Node, source: &str) -> Result<(Expression, Expression), String> {
    let mut cursor = node.walk();
    let mut event = None;
    let mut handler = None;

    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            if event.is_none() {
                event = Some(expr);
            } else {
                handler = Some(expr);
                break;
            }
        }
    }

    match (event, handler) {
        (Some(e), Some(h)) => Ok((e, h)),
        _ => Err("handler statement requires event and handler".to_string()),
    }
}
```

#### 6.5. await_statement

```rust
"await_statement" => Ok(Some(Self::convert_await_statement(node, source)?)),

fn convert_await_statement(node: &Node, source: &str) -> Result<Statement, String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            return Ok(Statement::Await { expression: expr });
        }
    }
    Err("await_statement without expression".to_string())
}
```

**Метрика успеха:**
- ✅ Все 21 statement types реализованы
- ✅ "Skipping unknown statement type" больше не появляется для стандартных конструкций
- ✅ Unit-тесты для каждого нового statement

---

### 📅 День 8: Реализация недостающих expressions (1 день)

**Задача:** Добавить ternary, new, await expressions

#### 8.1. ternary_expression

```rust
"ternary_expression" => Self::convert_ternary_expression(node, source),

fn convert_ternary_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let mut cursor = node.walk();
    let mut condition = None;
    let mut then_expr = None;
    let mut else_expr = None;

    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            if condition.is_none() {
                condition = Some(expr);
            } else if then_expr.is_none() {
                then_expr = Some(expr);
            } else {
                else_expr = Some(expr);
            }
        }
    }

    match (condition, then_expr, else_expr) {
        (Some(c), Some(t), Some(e)) => {
            Ok(Some(Expression::Ternary {
                condition: Box::new(c),
                then_expr: Box::new(t),
                else_expr: Box::new(e),
            }))
        }
        _ => Ok(None),
    }
}
```

#### 8.2. new_expression

```rust
"new_expression" | "new_expression_method" => Self::convert_new_expression(node, source),

fn convert_new_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let mut cursor = node.walk();
    let mut type_name = String::new();
    let mut args = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "property_access" => {
                type_name = Self::node_text(&child, source);
            }
            _ => {
                if let Some(expr) = Self::convert_expression(&child, source)? {
                    args.push(expr);
                }
            }
        }
    }

    Ok(Some(Expression::New { type_name, args }))
}
```

#### 8.3. await_expression

```rust
"await_expression" => Self::convert_await_expression(node, source),

fn convert_await_expression(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            return Ok(Some(Expression::Await {
                expression: Box::new(expr),
            }));
        }
    }
    Ok(None)
}
```

#### 8.4. Улучшение property_access

```rust
// ❌ БЫЛО (строка 298):
"property_access" => {
    Ok(Some(Expression::Identifier(Self::node_text(node, source))))
}

// ✅ ДОЛЖНО БЫТЬ:
"property_access" => Self::convert_property_access(node, source),

fn convert_property_access(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let mut cursor = node.walk();
    let mut object = None;
    let mut property = String::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if object.is_none() {
                    object = Some(Expression::Identifier(Self::node_text(&child, source)));
                } else {
                    property = Self::node_text(&child, source);
                }
            }
            "property_access" => {
                // Вложенный property access (a.b.c)
                if let Some(expr) = Self::convert_expression(&child, source)? {
                    object = Some(expr);
                }
            }
            "property" => {
                property = Self::node_text(&child, source);
            }
            _ => {}
        }
    }

    match object {
        Some(obj) if !property.is_empty() => {
            Ok(Some(Expression::PropertyAccess {
                object: Box::new(obj),
                property,
            }))
        }
        _ => Ok(Some(Expression::Identifier(Self::node_text(node, source)))),
    }
}
```

#### 8.5. index_access (доступ по индексу: arr[0])

```rust
"index" | "access" => Self::convert_index_access(node, source),

fn convert_index_access(node: &Node, source: &str) -> Result<Option<Expression>, String> {
    let mut cursor = node.walk();
    let mut object = None;
    let mut index = None;

    for child in node.children(&mut cursor) {
        if let Some(expr) = Self::convert_expression(&child, source)? {
            if object.is_none() {
                object = Some(expr);
            } else {
                index = Some(expr);
            }
        }
    }

    match (object, index) {
        (Some(obj), Some(idx)) => {
            Ok(Some(Expression::IndexAccess {
                object: Box::new(obj),
                index: Box::new(idx),
            }))
        }
        _ => Ok(None),
    }
}
```

#### 8.6. date literal

```rust
"date" => {
    let text = Self::node_text(node, source);
    Ok(Some(Expression::Date(text)))
}
```

**Метрика успеха:**
- ✅ Все expression types из grammar.js поддерживаются
- ✅ Property access работает корректно (a.b.c → PropertyAccess вложенные)
- ✅ Ternary, New, Await expressions парсятся

---

### 📅 День 9: Тестирование на реальных BSL файлах (1 день)

**Задача:** Создать comprehensive test suite

#### 9.1. Unit-тесты для каждого node type

**Файл:** `backend/tests/tree_sitter_adapter_test.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_bsl(code: &str) -> Program {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_bsl::language()).unwrap();
        let tree = parser.parse(code, None).unwrap();
        TreeSitterAdapter::convert_tree(&tree, code).unwrap()
    }

    #[test]
    fn test_procedure_declaration() {
        let code = "Процедура Тест(Параметр1, Параметр2)\n    Возврат;\nКонецПроцедуры";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::ProcedureDecl { name, params, body } => {
                assert_eq!(name, "Тест");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], "Параметр1");
                assert_eq!(params[1], "Параметр2");
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected ProcedureDecl"),
        }
    }

    #[test]
    fn test_function_declaration() {
        let code = "Функция Сумма(А, Б)\n    Возврат А + Б;\nКонецФункции";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::FunctionDecl { name, params, body } => {
                assert_eq!(name, "Сумма");
                assert_eq!(params.len(), 2);
                assert!(body.len() > 0);
            }
            _ => panic!("Expected FunctionDecl"),
        }
    }

    #[test]
    fn test_if_statement() {
        let code = "Если Условие Тогда\n    А = 1;\nИначе\n    А = 2;\nКонецЕсли";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::If { condition, then_body, else_body } => {
                assert_eq!(then_body.len(), 1);
                assert!(else_body.is_some());
            }
            _ => panic!("Expected If statement"),
        }
    }

    #[test]
    fn test_for_statement() {
        let code = "Для Индекс = 1 По 10 Цикл\n    Сообщить(Индекс);\nКонецЦикла";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::For { variable, start, end, body } => {
                assert_eq!(variable, "Индекс");
                assert!(matches!(start, Expression::Number(1.0)));
                assert!(body.len() > 0);
            }
            _ => panic!("Expected For statement"),
        }
    }

    #[test]
    fn test_while_statement() {
        let code = "Пока Условие Цикл\n    Счетчик = Счетчик + 1;\nКонецЦикла";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::While { condition, body } => {
                assert!(body.len() > 0);
            }
            _ => panic!("Expected While statement"),
        }
    }

    #[test]
    fn test_try_except() {
        let code = "Попытка\n    ВызватьМетод();\nИсключение\n    Сообщить(ОписаниеОшибки());\nКонецПопытки";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Try { try_body, except_body } => {
                assert!(try_body.len() > 0);
                assert!(except_body.len() > 0);
            }
            _ => panic!("Expected Try statement"),
        }
    }

    #[test]
    fn test_binary_expression() {
        let code = "А = 1 + 2 * 3";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Assignment { value, .. } => {
                assert!(matches!(value, Expression::Binary { .. }));
            }
            _ => panic!("Expected Assignment with Binary expression"),
        }
    }

    #[test]
    fn test_method_call() {
        let code = "Объект.Метод(Параметр1, Параметр2)";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        // Проверка что это Call statement с корректным expression
    }

    #[test]
    fn test_new_expression() {
        let code = "Массив = Новый Массив";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Assignment { value, .. } => {
                assert!(matches!(value, Expression::New { .. }));
            }
            _ => panic!("Expected Assignment with New expression"),
        }
    }

    #[test]
    fn test_ternary_expression() {
        let code = "Результат = ?(Условие, Истина, Ложь)";
        let program = parse_bsl(code);

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Statement::Assignment { value, .. } => {
                assert!(matches!(value, Expression::Ternary { .. }));
            }
            _ => panic!("Expected Assignment with Ternary expression"),
        }
    }
}
```

#### 9.2. Integration тесты с реальными BSL файлами

**Создать:** `backend/tests/fixtures/` с реальными модулями 1С

**Примеры файлов:**
- `document_object_module.bsl` — модуль объекта документа
- `common_module.bsl` — общий модуль
- `form_module.bsl` — модуль формы
- `manager_module.bsl` — модуль менеджера справочника

**Тесты:**
```rust
#[test]
fn test_parse_real_document_module() {
    let code = include_str!("fixtures/document_object_module.bsl");
    let program = parse_bsl(code);

    // Проверяем что парсинг прошёл без ошибок
    assert!(program.statements.len() > 0);

    // Проверяем отсутствие "Skipping unknown" в логах
    // (можно через capture logs в тестах)
}
```

**Метрика успеха:**
- ✅ 20+ unit-тестов для всех node types
- ✅ 5+ integration тестов на реальных BSL модулях
- ✅ Все тесты проходят (`cargo test --workspace`)
- ✅ Парсинг реальных модулей без "Skipping unknown" warnings

---

### 📅 День 10: Бенчмарки производительности и документация (1 день)

#### 10.1. Бенчмарки производительности

**Файл:** `backend/benches/tree_sitter_adapter_bench.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parse_small_file(c: &mut Criterion) {
    let code = include_str!("../tests/fixtures/small_module.bsl"); // ~100 строк

    c.bench_function("parse small BSL file (100 lines)", |b| {
        b.iter(|| {
            parse_bsl(black_box(code))
        });
    });
}

fn bench_parse_medium_file(c: &mut Criterion) {
    let code = include_str!("../tests/fixtures/medium_module.bsl"); // ~1000 строк

    c.bench_function("parse medium BSL file (1000 lines)", |b| {
        b.iter(|| {
            parse_bsl(black_box(code))
        });
    });
}

fn bench_parse_large_file(c: &mut Criterion) {
    let code = include_str!("../tests/fixtures/large_module.bsl"); // ~10000 строк

    c.bench_function("parse large BSL file (10000 lines)", |b| {
        b.iter(|| {
            parse_bsl(black_box(code))
        });
    });
}

criterion_group!(benches, bench_parse_small_file, bench_parse_medium_file, bench_parse_large_file);
criterion_main!(benches);
```

**Целевые показатели (из ROADMAP Milestone 2.1):**
- ✅ Парсинг 10000 строк < 200ms
- ✅ Инкрементальный парсинг < 10ms

#### 10.2. Документация

**Файл:** `docs/TREE_SITTER_ADAPTER_IMPLEMENTATION.md`

**Содержание:**
1. Маппинг всех tree-sitter node kinds → AST Statement/Expression
2. Таблица соответствия с примерами BSL кода
3. Архитектурные решения (почему property_access стал PropertyAccess)
4. Checklist для добавления новых node kinds
5. Примеры использования adapter

**Таблица маппинга:**

| Tree-sitter node kind | AST тип | Пример BSL кода | Статус |
|----------------------|---------|-----------------|--------|
| `procedure_definition` | `Statement::ProcedureDecl` | `Процедура Тест() КонецПроцедуры` | ✅ |
| `function_definition` | `Statement::FunctionDecl` | `Функция Сумма() КонецФункции` | ✅ |
| `if_statement` | `Statement::If` | `Если ... Тогда ... КонецЕсли` | ✅ |
| `for_statement` | `Statement::For` | `Для И = 1 По 10 Цикл ... КонецЦикла` | ✅ |
| `while_statement` | `Statement::While` | `Пока Условие Цикл ... КонецЦикла` | ✅ |
| `try_statement` | `Statement::Try` | `Попытка ... Исключение ... КонецПопытки` | ✅ |
| `binary_expression` | `Expression::Binary` | `А + Б * В` | ✅ |
| `call_expression` | `Expression::Call` | `Функция(Параметр)` | ✅ |
| `property_access` | `Expression::PropertyAccess` | `Объект.Свойство.Подсвойство` | ✅ |
| `new_expression` | `Expression::New` | `Новый Массив` | ✅ |
| `ternary_expression` | `Expression::Ternary` | `?(Условие, Да, Нет)` | ✅ |
| ... | ... | ... | ... |

**Checklist для добавления нового node kind:**
1. Добавить вариант в `Statement` или `Expression` enum
2. Добавить case в `convert_statement()` или `convert_expression()`
3. Реализовать `convert_xxx()` функцию
4. Написать unit-тест
5. Добавить пример в документацию
6. Обновить таблицу маппинга

**Метрика успеха:**
- ✅ Бенчмарки показывают < 200ms для 10000 строк
- ✅ Документация полная и понятная
- ✅ Contributor может добавить новый node kind за 30 минут

---

## 📊 Итоговые метрики Milestone 2.7

### Технические метрики

| Метрика | Цель | Текущее | После реализации |
|---------|------|---------|------------------|
| **Поддерживаемые node types** | 90% | 30% (11/36) | 95% (34/36) |
| **Парсинг 10000 строк** | < 200ms | N/A | < 150ms |
| **"Skipping unknown" warnings** | 0 для стандартных конструкций | Много | 0 |
| **Test coverage** | 80% | ~20% | 85% |
| **Все workspace тесты** | 100% pass | Некоторые сломаны | 100% pass |

### Функциональные метрики

| Функция | Текущее | После реализации |
|---------|---------|------------------|
| **Hover показывает типы** | "Dynamic type" | Реальные типы (80%+ случаев) |
| **Completion работает** | Только regex fallback | Tree-sitter AST + type inference |
| **Diagnostics** | Не работает | Работает (валидация по AST) |
| **Flow-sensitive analysis** | БЛОКИРОВАН | Разблокирован (базовый CFG) |
| **Semantic highlighting** | Не работает | Работает (подсветка по типам) |

---

## 🚀 Что разблокируется после Milestone 2.7

### ✅ Немедленно доступно:

1. **Milestone 2.2 — VSCode Extension оптимизация**
   - Все команды через LSP requests (используют полный AST)
   - Hover/Completion работают через tree-sitter
   - Diagnostics показывают реальные ошибки

2. **Milestone 2.3 — Advanced Type System**
   - Type inference из AST (Generic types из `arr.Add("text")`)
   - Null safety через flow-sensitive analysis
   - Union/Intersection types с контекстом

3. **Milestone 2.4 — Performance & Caching**
   - Кеш AST деревьев (инкрементальное обновление)
   - Параллельный анализ файлов

### ✅ В будущем (v3.0):

4. **Code Intelligence**
   - Goto Definition (на основе AST)
   - Find References (точный поиск)
   - Rename Symbol (безопасный рефакторинг)

5. **Static Analysis**
   - Анализ сложности функций (по AST)
   - Поиск дублирования кода
   - Security rules (SQL injection, XSS)

---

## 📝 Резюме

**Milestone 2.7 — это ФУНДАМЕНТ для всех дальнейших фич.**

**Текущая ситуация:**
- ❌ TreeSitterAdapter пропускает 70% конструкций BSL
- ❌ Hover показывает "Dynamic type" вместо реальных типов
- ❌ Flow-sensitive analysis БЛОКИРОВАН

**После реализации:**
- ✅ 95% конструкций BSL поддерживаются
- ✅ Hover показывает реальные типы (80%+ случаев)
- ✅ Flow-sensitive analysis РАЗБЛОКИРОВАН
- ✅ Все LSP features работают на полном AST

**Приоритет:** 🚨 КРИТИЧЕСКИЙ #1
**Длительность:** 10 дней
**Блокирует:** Milestone 2.2, 2.3, 2.4 и всю v3.0

**Следующий шаг:** Начать с расширения AST структур (День 1-2)
