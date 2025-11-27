# Milestone 3.18: IR Type Resolution Refactoring

**Статус:** Планируется
**Приоритет:** ВЫСОКИЙ
**Оценка сложности:** 2-3 недели
**Зависимости:** Milestone 3.13 (Object-Based Type Comparison), 3.16 (UncertaintyReason)

---

## Проблема

### Текущее состояние

IR (Intermediate Representation) хранит типы как **String**, что создает критические проблемы:

```rust
// shared/src/ir/mod.rs - Текущая реализация
pub enum SemanticNodeKind {
    Assignment {
        variable: String,
        value_type: String,              // <-- ПРОБЛЕМА: String без Certainty
        value_node: Option<usize>,
    },
    FunctionCall {
        function_name: String,
        object_name: Option<String>,
        object_type: Option<String>,     // <-- ПРОБЛЕМА: String без Certainty
        arg_types: Vec<String>,          // <-- ПРОБЛЕМА: Vec<String> без Certainty
    },
    MemberAccess {
        object_name: Option<String>,
        object_type: String,             // <-- ПРОБЛЕМА: String без Certainty
        member_name: String,
        is_method: bool,
    },
    // ... другие 20+ полей с String типами
}
```

### Корневая причина

**TypeResolver НЕ используется при создании IR:**

```rust
// backend/src/application/ast_to_ir.rs:827
fn infer_expression_type(&self, expr: &Expression) -> String {
    match expr {
        Expression::Number { .. } => "Число".to_string(),    // Просто String!
        Expression::String { .. } => "Строка".to_string(),
        Expression::Identifier { name, .. } => {
            // Эвристика без TypeResolver
            self.lookup_variable_type(name)
                .unwrap_or_else(|| name.clone())
        }
        // ...
    }
}
```

### Костыль simple_resolution() - КРИТИЧЕСКИЙ БАГ

```rust
// backend/src/application/semantic_validation_visitor.rs:286
fn simple_resolution(type_name: &str) -> TypeResolution {
    // КРИТИЧЕСКАЯ ПРОБЛЕМА: Всегда возвращает Certainty::Known!
    TypeResolution {
        certainty: Certainty::Known,  // <-- БАГ: Unknown типы тоже Known
        result: /* ... */,
        source: ResolutionSource::Static,
        // ...
    }
}
```

### Результат проблемы

1. **Ложные ошибки валидации:** Unknown типы показываются как ошибки (должны пропускаться)
2. **Потеря информации:** Certainty, Facet, Metadata теряются при конвертации в String
3. **Дублирование структур:** TypeHint в IR дублирует TypeResolution
4. **Неэффективность:** Повторный резолвинг типов вместо использования закешированных

---

## Решение: TypeResolution как единая точка ответственности

### Архитектурный принцип

**TypeResolution** должен быть **единственным представлением типа** во всей системе:

```
AST → IR (TypeResolution) → Validation (TypeResolution) → Diagnostics
```

Никаких промежуточных конверсий String → TypeResolution → String.

### Оценка memory overhead

| Параметр | Значение |
|----------|----------|
| Размер TypeResolution | ~300 байт |
| Типов на файл (avg) | ~200 |
| Файлов в проекте (avg) | ~100 |
| **Итого памяти** | **~6 MB** |

**6 MB — допустимый overhead** для современных систем.

### Удаляемые структуры

1. **TypeHint** — полностью удалить (BSL не имеет type annotations, всё выводится)
2. **simple_resolution()** — удалить (будет прямое использование TypeResolution)

### Архитектурная диаграмма

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ТЕКУЩАЯ АРХИТЕКТУРА                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  AST (tree-sitter)                                                  │
│       │                                                             │
│       ▼                                                             │
│  ┌─────────────────────┐                                           │
│  │  AstToIrConverter   │  infer_expression_type() → String         │
│  │  (ast_to_ir.rs)     │                                           │
│  └─────────────────────┘                                           │
│       │                                                             │
│       ▼                                                             │
│  ┌─────────────────────┐                                           │
│  │  SemanticProgram    │  value_type: String (без Certainty!)      │
│  │  (IR) + TypeHint    │  object_type: String + TypeHint           │
│  └─────────────────────┘                                           │
│       │                                                             │
│       ▼                                                             │
│  ┌─────────────────────┐                                           │
│  │  SemanticValidation │  simple_resolution(String)                │
│  │  Visitor            │  → TypeResolution {Certainty::Known} !!   │
│  └─────────────────────┘                                           │
│       │                                                             │
│       ▼                                                             │
│  ПРОБЛЕМА: Unknown типы валидируются как Known → Ложные ошибки     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                         ЦЕЛЕВАЯ АРХИТЕКТУРА                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  AST (tree-sitter)                                                  │
│       │                                                             │
│       ▼                                                             │
│  ┌─────────────────────┐    ┌─────────────────────┐                │
│  │  AstToIrConverter   │───▶│    TypeResolver     │                │
│  │  (ast_to_ir.rs)     │    │  resolve_expression │                │
│  └─────────────────────┘    └─────────────────────┘                │
│       │                              │                              │
│       │                              ▼                              │
│       │                      TypeResolution                         │
│       │                      (полная информация)                    │
│       │                              │                              │
│       ▼                              │                              │
│  ┌─────────────────────────────────────────────────┐               │
│  │  SemanticProgram (IR)                           │               │
│  │  value_type: TypeResolution                     │               │
│  │  object_type: TypeResolution                    │               │
│  │  arg_types: Vec<TypeResolution>                 │               │
│  └─────────────────────────────────────────────────┘               │
│       │                                                             │
│       ▼                                                             │
│  ┌─────────────────────────────────────────────────┐               │
│  │  SemanticValidationVisitor                      │               │
│  │  → if certainty == Unknown: skip validation     │               │
│  │  → Прямое использование TypeResolution          │               │
│  └─────────────────────────────────────────────────┘               │
│       │                                                             │
│       ▼                                                             │
│  РЕЗУЛЬТАТ: Unknown типы пропускаются → Нет ложных ошибок          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## План реализации

### Phase 1: Подготовка TypeResolution (2 дня)

**Задачи:**

1. **Добавить Serialize/Deserialize для TypeResolution:**
```rust
// shared/src/domain/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeResolution {
    pub certainty: Certainty,
    pub result: ResolutionResult,
    pub source: ResolutionSource,
    pub metadata: ResolutionMetadata,
    pub active_facet: Option<FacetKind>,
    pub available_facets: Vec<FacetKind>,
}
```

2. **Добавить конструкторы для удобства:**
```rust
impl TypeResolution {
    /// Известный примитивный тип
    pub fn primitive(type_name: &str) -> Self {
        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Resolved(TypeKind::Primitive(type_name.into())),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Неизвестный тип с причиной
    pub fn unknown(reason: UncertaintyReason) -> Self {
        Self {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Unknown(reason.clone()),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Выведенный тип с уверенностью
    pub fn inferred(type_name: &str, confidence: f32) -> Self {
        Self {
            certainty: Certainty::Inferred(confidence),
            result: ResolutionResult::Resolved(TypeKind::Primitive(type_name.into())),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Тип метаданных (справочник, документ и т.д.)
    pub fn metadata_type(kind: MetadataKind, name: &str, facet: FacetKind) -> Self {
        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Resolved(TypeKind::MetadataObject {
                kind,
                name: name.to_string(),
            }),
            source: ResolutionSource::Metadata,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(facet),
            available_facets: vec![facet],
        }
    }

    /// Проверка: тип Unknown?
    pub fn is_unknown(&self) -> bool {
        matches!(self.certainty, Certainty::Unknown)
    }

    /// Получить имя типа как String (для совместимости)
    pub fn type_name(&self) -> String {
        // ... extract from result
    }
}
```

3. **Добавить Serialize/Deserialize для зависимых типов:**
```rust
// Убедиться что сериализуются:
// - Certainty
// - ResolutionResult
// - ResolutionSource
// - ResolutionMetadata
// - FacetKind
// - UncertaintyReason
// - TypeKind
```

**Файлы для изменения:**
- `shared/src/domain/types.rs` — Serialize/Deserialize + конструкторы

**Критерии успеха Phase 1:**
- [ ] TypeResolution сериализуется в JSON
- [ ] Конструкторы работают корректно
- [ ] Unit тесты проходят

---

### Phase 2: Удаление TypeHint (2 дня)

**Задачи:**

1. **Найти все использования TypeHint:**
```bash
grep -r "TypeHint" shared/src/
grep -r "TypeHint" backend/src/
```

2. **Заменить TypeHint на TypeResolution в SymbolTable:**
```rust
// shared/src/ir/mod.rs

// БЫЛО:
pub struct SymbolInfo {
    pub name: String,
    pub type_hint: TypeHint,
    // ...
}

// СТАЛО:
pub struct SymbolInfo {
    pub name: String,
    pub resolved_type: TypeResolution,  // <-- Полная информация о типе
    // ...
}
```

3. **Удалить enum TypeHint:**
```rust
// УДАЛИТЬ из shared/src/ir/mod.rs:
pub enum TypeHint {
    Explicit(String),
    Inferred(String),
    Generic { ... },
    Unknown,
}
```

4. **Обновить все места создания SymbolInfo:**
```rust
// БЫЛО:
let symbol = SymbolInfo {
    name: var_name.clone(),
    type_hint: TypeHint::Inferred(type_name),
    // ...
};

// СТАЛО:
let symbol = SymbolInfo {
    name: var_name.clone(),
    resolved_type: TypeResolution::inferred(&type_name, 0.8),
    // ...
};
```

**Файлы для изменения:**
- `shared/src/ir/mod.rs` — удалить TypeHint, обновить SymbolInfo
- `backend/src/application/ast_to_ir.rs` — обновить создание symbols
- Все файлы использующие TypeHint

**Критерии успеха Phase 2:**
- [ ] TypeHint удалён полностью
- [ ] SymbolTable использует TypeResolution
- [ ] `cargo build --workspace` без ошибок

---

### Phase 3: Миграция SemanticNodeKind (1 неделя)

**Задачи:**

1. **Обновить SemanticNodeKind поэтапно:**

**Этап 3.1: FunctionCall**
```rust
FunctionCall {
    function_name: String,
    object_name: Option<String>,
    object_type: Option<TypeResolution>,  // ИЗМЕНЕНО: было Option<String>
    arg_types: Vec<TypeResolution>,       // ИЗМЕНЕНО: было Vec<String>
}
```

**Этап 3.2: MemberAccess**
```rust
MemberAccess {
    object_name: Option<String>,
    object_type: TypeResolution,          // ИЗМЕНЕНО: было String
    member_name: String,
    is_method: bool,
}
```

**Этап 3.3: Assignment**
```rust
Assignment {
    variable: String,
    value_type: TypeResolution,           // ИЗМЕНЕНО: было String
    value_node: Option<usize>,
}
```

**Этап 3.4: NewExpression**
```rust
NewExpression {
    type_name: String,
    result_type: TypeResolution,          // ИЗМЕНЕНО
    arg_types: Vec<TypeResolution>,       // ИЗМЕНЕНО
    generic_params: Option<Vec<String>>,
    is_dynamic: bool,
}
```

**Этап 3.5: Остальные узлы**
```rust
IfStatement {
    condition_type: TypeResolution,       // ИЗМЕНЕНО
    then_branch: Vec<usize>,
    else_branch: Option<Vec<usize>>,
}

ForStatement {
    iterator_type: TypeResolution,        // ИЗМЕНЕНО
    collection_type: TypeResolution,      // ИЗМЕНЕНО
    body: Vec<usize>,
}

ReturnStatement {
    value_type: Option<TypeResolution>,   // ИЗМЕНЕНО
    value_node: Option<usize>,
}

// ... и т.д. (24 поля)
```

2. **Создать метод infer_type_resolution() в AstToIrConverter:**
```rust
// backend/src/application/ast_to_ir.rs

impl AstToIrConverter {
    /// Вывод типа с полной информацией
    fn infer_type_resolution(&self, expr: &Expression) -> TypeResolution {
        match expr {
            Expression::Number { .. } => TypeResolution::primitive("Число"),
            Expression::String { .. } => TypeResolution::primitive("Строка"),
            Expression::Boolean { .. } => TypeResolution::primitive("Булево"),
            Expression::Date { .. } => TypeResolution::primitive("Дата"),
            Expression::Undefined { .. } => TypeResolution::primitive("Неопределено"),
            Expression::Null { .. } => TypeResolution::primitive("Null"),

            Expression::Identifier { name, .. } => {
                // Поиск в scope
                if let Some(symbol) = self.lookup_symbol(name) {
                    symbol.resolved_type.clone()
                } else {
                    TypeResolution::unknown(UncertaintyReason::VariableNotDeclared {
                        name: name.clone(),
                    })
                }
            }

            Expression::New { type_name, args, .. } => {
                self.resolve_new_expression(type_name, args)
            }

            Expression::PropertyAccess { object, property, .. } => {
                let base = self.infer_type_resolution(object);
                self.resolve_property_access(base, property)
            }

            Expression::Call { function, args, .. } => {
                self.resolve_call_expression(function, args)
            }

            _ => TypeResolution::inferred("Dynamic", 0.3),
        }
    }

    /// Резолвинг Новый Type()
    fn resolve_new_expression(&self, type_name: &str, args: &[Expression]) -> TypeResolution {
        // Проверяем в TypeRepository
        if let Some(type_info) = self.repository.find_type(type_name) {
            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Resolved(type_info.kind.clone()),
                source: ResolutionSource::Constructor,
                metadata: ResolutionMetadata::default(),
                active_facet: Some(FacetKind::Object),
                available_facets: vec![FacetKind::Object],
            }
        } else {
            // Тип не найден - Unknown
            TypeResolution::unknown(UncertaintyReason::TypeNotFound {
                type_name: type_name.to_string(),
            })
        }
    }

    /// Резолвинг доступа к свойству (Справочники.Контрагенты и т.д.)
    fn resolve_property_access(&self, base: TypeResolution, property: &str) -> TypeResolution {
        // Логика фасетной трансформации
        match base.type_name().as_str() {
            "Справочники" | "Catalogs" => {
                if self.repository.has_catalog(property) {
                    TypeResolution::metadata_type(
                        MetadataKind::Catalog,
                        property,
                        FacetKind::Manager
                    )
                } else {
                    TypeResolution::unknown(UncertaintyReason::MetadataObjectNotFound {
                        kind: MetadataKind::Catalog,
                        name: property.to_string(),
                    })
                }
            }
            "Документы" | "Documents" => {
                if self.repository.has_document(property) {
                    TypeResolution::metadata_type(
                        MetadataKind::Document,
                        property,
                        FacetKind::Manager
                    )
                } else {
                    TypeResolution::unknown(UncertaintyReason::MetadataObjectNotFound {
                        kind: MetadataKind::Document,
                        name: property.to_string(),
                    })
                }
            }
            // ... другие коллекции
            _ => {
                // Неизвестное свойство
                TypeResolution::inferred(
                    &format!("{}.{}", base.type_name(), property),
                    0.5
                )
            }
        }
    }
}
```

3. **Обновить старый метод как wrapper:**
```rust
/// Обратная совместимость (deprecated)
#[deprecated(note = "Use infer_type_resolution() instead")]
fn infer_expression_type(&self, expr: &Expression) -> String {
    self.infer_type_resolution(expr).type_name()
}
```

**Файлы для изменения:**
- `shared/src/ir/mod.rs` — SemanticNodeKind с TypeResolution
- `backend/src/application/ast_to_ir.rs` — infer_type_resolution()
- `backend/src/domain/flow_analyzer.rs` — TypeResolution вместо String
- `backend/src/domain/flow_analyzer_simple.rs` — TypeResolution вместо String
- `shared/src/ir/visitor.rs` — обновить visitor pattern

**Критерии успеха Phase 3:**
- [ ] Все 24 поля типов мигрированы на TypeResolution
- [ ] `cargo build --workspace` без ошибок
- [ ] `cargo test --workspace` — 100% pass

---

### Phase 4: Удаление simple_resolution() (2 дня)

**Задачи:**

1. **Обновить SemanticValidationVisitor:**
```rust
// backend/src/application/semantic_validation_visitor.rs

impl<'a> SemanticVisitor for SemanticValidationVisitor<'a> {
    fn visit_node(&mut self, node: &SemanticNode, _context: &mut FlowContext) {
        match &node.kind {
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                object_type: Some(obj_type),  // obj_type теперь TypeResolution!
                arg_types,
                ..
            } => {
                // ✅ КЛЮЧЕВОЕ ИЗМЕНЕНИЕ: Пропускаем валидацию для Unknown типов
                if obj_type.is_unknown() {
                    // Graceful degradation: не показываем cascade ошибки
                    return;
                }

                // Пропускаем если низкая уверенность
                if let Certainty::Inferred(confidence) = &obj_type.certainty {
                    if *confidence < 0.5 {
                        return;
                    }
                }

                // Прямое использование TypeResolution для валидации
                self.validate_method_call(obj_type, function_name, arg_types);
            }

            SemanticNodeKind::MemberAccess {
                object_name,
                object_type,  // TypeResolution!
                member_name,
                is_method: false,
                ..
            } => {
                // ✅ Пропускаем валидацию для Unknown типов
                if object_type.is_unknown() {
                    return;
                }

                self.validate_property_access(object_type, member_name);
            }

            // ... другие cases
            _ => {}
        }
    }

    fn validate_method_call(
        &mut self,
        object_type: &TypeResolution,
        method_name: &str,
        arg_types: &[TypeResolution]
    ) {
        // Прямое использование TypeResolution - никакого simple_resolution!
        let type_info = self.repository.find_by_resolution(object_type);

        if let Some(info) = type_info {
            if !info.has_method(method_name, object_type.active_facet) {
                self.add_error(SemanticError::MethodNotFound {
                    type_name: object_type.type_name(),
                    method_name: method_name.to_string(),
                    facet: object_type.active_facet,
                });
            }
        }
    }
}
```

2. **Удалить simple_resolution():**
```rust
// УДАЛИТЬ полностью из semantic_validation_visitor.rs:
// fn simple_resolution(type_name: &str) -> TypeResolution { ... }
// (строки 286-356)
```

**Файлы для изменения:**
- `backend/src/application/semantic_validation_visitor.rs` — удалить `simple_resolution()`

**Критерии успеха Phase 4:**
- [ ] `simple_resolution()` удален
- [ ] Валидация Unknown типов пропускается
- [ ] `cargo test --workspace` — 100% pass

---

### Phase 5: Интеграционное тестирование (3 дня)

**Задачи:**

1. **Тесты на Unknown типы:**
```rust
#[test]
fn test_unknown_type_skips_validation() {
    // Тип "НесуществующийТип" не найден → Unknown → нет ошибки
    let code = r#"
        Перем x;
        x = Новый НесуществующийТип();
        x.НесуществующийМетод();  // НЕ должно быть ошибки!
    "#;

    let diagnostics = validate_code(code);

    // Нет ошибок про "метод не найден" для Unknown типов
    assert!(diagnostics.is_empty(),
        "Unknown type should not produce validation errors");
}

#[test]
fn test_known_type_validates() {
    let code = r#"
        МассивДанных = Новый Массив();
        МассивДанных.НесуществующийМетод();
    "#;

    let diagnostics = validate_code(code);

    // Есть ошибка: метод НесуществующийМетод не найден для Массив
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("не найден"));
}

#[test]
fn test_metadata_not_loaded_graceful_degradation() {
    // Конфигурация не загружена → Unknown → нет каскадных ошибок
    let code = r#"
        Док = Документы.ЗаказКлиента.СоздатьДокумент();
        Док.Провести();  // НЕ должно быть ошибки если ЗаказКлиента unknown
    "#;

    let diagnostics = validate_code_without_config(code);

    // Graceful degradation: нет каскадных ошибок
    assert!(diagnostics.is_empty());
}

#[test]
fn test_inferred_low_confidence_skips_validation() {
    // Тип выведен с низкой уверенностью → пропускаем валидацию
    let code = r#"
        Результат = СложнаяФункция();  // Inferred(0.3)
        Результат.КакойТоМетод();       // НЕ должно быть ошибки
    "#;

    let diagnostics = validate_code(code);
    assert!(diagnostics.is_empty());
}
```

2. **Тесты сериализации IR Cache:**
```rust
#[test]
fn test_ir_cache_with_type_resolution() {
    let program = create_test_program();

    // Сериализация
    let json = serde_json::to_string(&program).unwrap();

    // Десериализация
    let restored: SemanticProgram = serde_json::from_str(&json).unwrap();

    // Проверка TypeResolution сохранился
    assert_eq!(
        program.nodes[0].value_type().certainty,
        restored.nodes[0].value_type().certainty
    );
}
```

3. **Regression тесты:**
```rust
#[test]
fn test_hover_works_with_type_resolution() {
    // Проверяем, что hover не сломался
}

#[test]
fn test_diagnostics_work_with_type_resolution() {
    // Проверяем, что диагностики работают
}
```

**Файлы для создания:**
- `backend/tests/type_resolution_ir_test.rs` — новые тесты
- `backend/tests/unknown_type_graceful_test.rs` — тесты graceful degradation

**Критерии успеха Phase 5:**
- [ ] Все новые тесты проходят
- [ ] Все существующие тесты проходят (332+ regression)
- [ ] LSP функционирует корректно
- [ ] IR Cache сериализуется/десериализуется

---

## Список файлов для изменения

### Удаляемые структуры
| Структура | Файл | Причина удаления |
|-----------|------|------------------|
| `TypeHint` | `shared/src/ir/mod.rs` | BSL не имеет type annotations |
| `simple_resolution()` | `backend/src/application/semantic_validation_visitor.rs` | Костыль с багом |

### Изменяемые файлы (Core)
| Файл | Изменения |
|------|-----------|
| `shared/src/domain/types.rs` | Serialize/Deserialize, конструкторы |
| `shared/src/ir/mod.rs` | SemanticNodeKind → TypeResolution, удалить TypeHint |
| `shared/src/ir/visitor.rs` | Обновить visitor pattern |
| `backend/src/application/ast_to_ir.rs` | `infer_type_resolution()` |
| `backend/src/application/semantic_validation_visitor.rs` | Удалить `simple_resolution()` |

### Изменяемые файлы (Consumers)
| Файл | Изменения |
|------|-----------|
| `backend/src/domain/flow_analyzer.rs` | TypeResolution вместо String |
| `backend/src/domain/flow_analyzer_simple.rs` | TypeResolution вместо String |
| `backend/src/bin/lsp_server.rs` | Обновить hover/diagnostics |
| `backend/src/presentation/web/handlers.rs` | Обновить API responses |

### Новые тесты
| Файл | Описание |
|------|----------|
| `backend/tests/type_resolution_ir_test.rs` | Интеграционные тесты |
| `backend/tests/unknown_type_graceful_test.rs` | Тесты graceful degradation |

---

## Критерии успеха Milestone 3.18

### Функциональные
- [ ] **F1:** TypeResolution используется напрямую в IR (вместо String)
- [ ] **F2:** TypeHint полностью удалён
- [ ] **F3:** `simple_resolution()` полностью удалён
- [ ] **F4:** Unknown типы пропускаются при валидации
- [ ] **F5:** Certainty.Inferred с низкой уверенностью пропускается
- [ ] **F6:** UncertaintyReason сохраняется в IR

### Качественные
- [ ] **Q1:** IR Cache работает с TypeResolution (сериализация)
- [ ] **Q2:** Hover показывает полную информацию TypeResolution
- [ ] **Q3:** Diagnostics не показывают cascade ошибки для Unknown

### Метрики производительности
| Метрика | До | После | Допустимо |
|---------|-----|-------|-----------|
| Размер IR на файл | ~10 KB | ~60 KB | < 100 KB |
| IR Cache память (100 файлов) | ~1 MB | ~6 MB | < 50 MB |
| Hover latency | <5ms | <7ms | <15ms |
| Validation latency | <10ms | <12ms | <25ms |

### Тесты
- [ ] **T1:** 100% существующих тестов проходят (332+)
- [ ] **T2:** 10+ новых тестов для TypeResolution в IR
- [ ] **T3:** 5+ тестов на graceful degradation
- [ ] **T4:** `cargo test --workspace` — 0 failures

---

## Оценка рисков

### Высокие риски
| Риск | Вероятность | Влияние | Митигация |
|------|-------------|---------|-----------|
| Breaking change в IR Cache | Высокая | Высокое | Версионирование кеша, миграция |
| Регрессия в LSP | Средняя | Высокое | Обширные regression тесты |

### Средние риски
| Риск | Вероятность | Влияние | Митигация |
|------|-------------|---------|-----------|
| Увеличение памяти IR | Высокая | Среднее | ~6MB допустимо |
| Сложность сериализации вложенных enum | Средняя | Среднее | serde flatten/tag attributes |

### Низкие риски
| Риск | Вероятность | Влияние | Митигация |
|------|-------------|---------|-----------|
| Performance degradation | Низкая | Низкое | Benchmarks |

---

## Связанные Milestones

- **3.13 Object-Based Type Comparison** — базовая инфраструктура `is_compatible_with()`
- **3.16 UncertaintyReason** — структура для причин Unknown
- **3.19 (будущий)** — Full TypeResolution в FlowAnalysisContext
- **3.20 (будущий)** — Type narrowing с учетом Certainty

---

## Заключение

Milestone 3.18 решает критическую архитектурную проблему:

**Было:**
- IR хранит типы как String → потеря Certainty
- TypeHint дублирует информацию
- `simple_resolution()` — костыль с багом (всегда Known)
- Unknown типы показывают cascade ошибки

**Станет:**
- **TypeResolution = единая точка ответственности**
- Никаких промежуточных конверсий
- Unknown типы пропускаются при валидации
- Graceful degradation когда config не загружен

**Memory overhead:** ~6MB — допустимо для современных систем.
