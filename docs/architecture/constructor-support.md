# Constructor Support in BSL Type System

## Overview

Поддержка конструкторов в BSL Gradual Type System позволяет анализировать выражения создания объектов через ключевое слово `Новый`.

## IR Node: NewExpression

### Определение

```rust
SemanticNodeKind::NewExpression {
    /// Имя типа для создания
    type_name: String,

    /// Типы аргументов конструктора
    arg_types: Vec<String>,

    /// Динамический конструктор через строку
    is_dynamic: bool,

    /// Результирующий тип
    result_type: String,

    /// Generic параметры для коллекций
    generic_params: Option<Vec<String>>,
}
```

### Примеры использования

#### Простой конструктор без параметров

```bsl
МассивДанных = Новый Массив;
```

Представление в IR:
```rust
SemanticNodeKind::NewExpression {
    type_name: "Массив",
    arg_types: vec![],
    is_dynamic: false,
    result_type: "Массив",
    generic_params: None,
}
```

#### Конструктор с параметрами

```bsl
МассивФиксированный = Новый Массив(10);
```

Представление в IR:
```rust
SemanticNodeKind::NewExpression {
    type_name: "Массив",
    arg_types: vec!["Число"],
    is_dynamic: false,
    result_type: "Массив",
    generic_params: None,
}
```

#### Динамический конструктор

```bsl
Ссылка = Новый("СправочникСсылка.Номенклатура");
```

Представление в IR:
```rust
SemanticNodeKind::NewExpression {
    type_name: "СправочникСсылка.Номенклатура",
    arg_types: vec![],
    is_dynamic: true,
    result_type: "СправочникСсылка.Номенклатура",
    generic_params: None,
}
```

#### Generic конструктор с явным параметром

```bsl
МассивЧисел = Новый Массив<Число>;
```

Представление в IR:
```rust
SemanticNodeKind::NewExpression {
    type_name: "Массив",
    arg_types: vec![],
    is_dynamic: false,
    result_type: "Массив<Число>",
    generic_params: Some(vec!["Число"]),
}
```

## Семантика полей

### type_name

Имя типа, который создаётся конструктором. Может быть:
- Базовый тип платформы: `Массив`, `Строка`, `ТаблицаЗначений`
- Прикладной тип: `СправочникСсылка.Номенклатура`, `ДокументОбъект.ЗаказКлиента`
- Полный путь к типу: `ОбщиеМодули.МойМодуль.МойКласс`

### arg_types

Типы аргументов конструктора в порядке их передачи. Используется для:
- Валидации корректности вызова конструктора
- Type inference для Generic типов
- Проверки соответствия сигнатуре конструктора

### is_dynamic

Флаг динамического конструктора. `true` для выражений вида:
```bsl
Объект = Новый("ИмяТипаИзСтроки");
```

Динамические конструкторы требуют runtime проверки типа.

### result_type

Результирующий тип после выполнения конструктора. Может отличаться от `type_name` для Generic типов:
- `type_name = "Массив"` → `result_type = "Массив<Число>"` (если выведено)
- `type_name = "Соответствие"` → `result_type = "Соответствие<Строка, Число>"`

### generic_params

Generic параметры для коллекций. `Some(vec![...])` если тип Generic, `None` если обычный тип.

#### Примеры:
- `Массив` → `None` (inference будет выполнен позже)
- `Массив<Число>` → `Some(vec!["Число"])`
- `Соответствие<Строка, Число>` → `Some(vec!["Строка", "Число"])`

## Flow в Type System

### 1. Parsing (Backend)

Parser распознаёт выражение `Новый` и создаёт AST узел.

### 2. AST → IR Conversion (Backend)

AstToIrConverter преобразует AST в `SemanticNodeKind::NewExpression`:
```rust
// Pseudo-code
fn convert_new_expression(ast_node: &AstNode) -> SemanticNode {
    let type_name = extract_type_name(ast_node);
    let arg_types = extract_arg_types(ast_node);
    let is_dynamic = check_if_dynamic(ast_node);

    SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: type_name.clone(),
            arg_types,
            is_dynamic,
            result_type: type_name, // Может быть обновлено позже
            generic_params: None,    // Заполняется в AnalysisEngine
        },
        span: ast_node.span(),
        scope_id: current_scope,
    }
}
```

### 3. Type Analysis (Shared)

AnalysisEngine анализирует `NewExpression`:
1. Проверяет существование типа в TypeRepository
2. Валидирует аргументы конструктора
3. Для Generic типов выполняет inference параметров
4. Обновляет `result_type` и `generic_params`

### 4. Type Resolution (Shared)

TypeResolver резолвит тип конструктора:
```rust
// Pseudo-code
fn resolve_constructor(new_expr: &NewExpression) -> TypeResolution {
    match &new_expr.generic_params {
        Some(params) => {
            // Generic тип с известными параметрами
            TypeResolution::Generic {
                base_type: new_expr.type_name.clone(),
                type_params: params.clone(),
            }
        }
        None => {
            // Обычный тип или Generic без параметров
            resolve_type(&new_expr.result_type)
        }
    }
}
```

## Integration Points

### Backend (AstToIrConverter)

**Responsibility:** Парсинг BSL кода и создание IR узлов

**Tasks:**
- Распознавание конструкторов `Новый Тип` и `Новый("Тип")`
- Извлечение аргументов конструктора
- Определение динамических конструкторов
- Создание `SemanticNodeKind::NewExpression`

### Shared (AnalysisEngine)

**Responsibility:** Семантический анализ конструкторов

**Tasks:**
- Валидация существования типа
- Проверка корректности аргументов
- Generic inference для коллекций
- Обновление `result_type` и `generic_params`

### Shared (TypeResolver)

**Responsibility:** Резолюция типов конструкторов

**Tasks:**
- Резолюция базового типа через TypeRepository
- Применение Generic параметров
- Резолюция facets для типов платформы
- Резолюция прикладных типов

## Testing

### Unit Tests (shared/src/ir/mod.rs)

```rust
#[test]
fn test_new_expression_simple() { /* ... */ }

#[test]
fn test_new_expression_with_args() { /* ... */ }

#[test]
fn test_new_expression_dynamic() { /* ... */ }

#[test]
fn test_new_expression_with_generics() { /* ... */ }

#[test]
fn test_new_expression_to_dto() { /* ... */ }
```

### Integration Tests (планируется)

```bsl
// Test: Constructor без параметров
МассивДанных = Новый Массив;
// Expected: result_type = "Массив", generic_params = None

// Test: Constructor с параметром
МассивФиксированный = Новый Массив(10);
// Expected: arg_types = ["Число"]

// Test: Generic inference
МассивСтрок = Новый Массив;
МассивСтрок.Добавить("текст");
// Expected: generic_params = Some(["Строка"])

// Test: Динамический конструктор
Ссылка = Новый("СправочникСсылка.Номенклатура");
// Expected: is_dynamic = true
```

## Future Enhancements

### 1. Constructor Signature Validation

Валидация соответствия аргументов конструктора ожидаемой сигнатуре:
```bsl
Массив = Новый Массив("неправильный аргумент"); // ❌ Ошибка
```

### 2. Generic Inference from Constructor Args

Вывод Generic параметров из типов аргументов:
```bsl
Соответствие = Новый Соответствие<Строка, Число>;
// Автоматически выводим: K = Строка, V = Число
```

### 3. Platform Type Constructor Database

База данных конструкторов типов платформы:
```rust
struct ConstructorSignature {
    type_name: String,
    parameters: Vec<ParameterInfo>,
    result_type: String,
}
```

### 4. Prикладные Type Constructors

Поддержка конструкторов прикладных типов с параметрами:
```bsl
Документ = Новый("Документ.Заказ", Дата);
```

## Version History

- **0.4.2** (2025-11-05): Initial implementation of `NewExpression` IR node
  - Added `SemanticNodeKind::NewExpression` variant
  - Implemented DTO conversion
  - Added 5 unit tests
  - Documentation created

## Related Documentation

- [Type System Architecture](type_system_architecture.md)
- [Components Detailed](components-detailed.md)
- [Milestones History](milestones-history.md)

---

**Last Updated:** 2025-11-05
**Version:** 0.4.2
