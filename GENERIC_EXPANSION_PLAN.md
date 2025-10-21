# План расширения Generic функциональности

**Дата:** 2025-10-21
**Автор:** Architect Agent
**Контекст:** Reviewer предложил 3 направления расширения Generic типов после успешной реализации `ТабличнаяЧасть<СтрокаРаботы>`

---

## 📊 Executive Summary

### Рекомендуемая приоритизация

| Приоритет | Направление | Обоснование | Трудозатраты | Риски |
|-----------|-------------|-------------|--------------|-------|
| **1** | HoverFormatter (Direction 3) | Quick win, улучшает maintainability, 0 breaking changes | **1-2 дня** | LOW |
| **2** | Generic Collections (Direction 2) | Высокая ценность для пользователя, простая реализация | **3-5 дней** | MEDIUM |
| **3** | Nested Generics (Direction 1) | Низкая практическая ценность для 1С, высокая сложность | **5-7 дней** | HIGH |

### Value/Effort Ratio

```
HoverFormatter:        ████████████ (12/10) ⭐ HIGHEST
Generic Collections:   ██████████   (10/10) ⭐ HIGH
Nested Generics:       ████         (4/10)  ⚠️ LOW
```

### Ключевые выводы

1. **HoverFormatter** — начать ПЕРВЫМ, т.к. упростит реализацию направлений 1 и 2
2. **Generic Collections** — основная ценность для пользователей 1С
3. **Nested Generics** — рассмотреть как Future Scope (ПОСЛЕ Milestone 3.x)

---

## 🔍 Направление 1: Вложенные Generic типы

### Найденные решения

#### 1. Rust Generic Type Recursion

**Источник:** [Stack Overflow - Recursive Generic Types](https://stackoverflow.com/questions/53968543/how-to-write-a-recursive-generic-type-in-rust)

**Ключевая идея:** Rust требует `Box<T>` для рекурсивных типов, чтобы избежать infinite size types:

```rust
struct Node<T> {
    value: T,
    // ✅ ПРАВИЛЬНО: Box даёт известный размер
    children: Vec<Box<Node<T>>>,
}

// ❌ ОШИБКА: infinite size
// children: Vec<Node<T>>,
```

**Применимость к BSL:** Наша система НЕ нуждается в Box, т.к. `GenericType` уже содержит `Vec<ConcreteType>`, а не вложенные `GenericType` напрямую.

#### 2. TypeScript Nested Array Inference

**Источник:** [GitHub TypeScript Issue #15659](https://github.com/Microsoft/TypeScript/issues/15659)

**Ключевая идея:** TypeScript использует conditional types для извлечения типа элемента:

```typescript
type ElementType<T> = T extends Array<infer U> ? U : T;

// Пример:
type A = Array<string>;        // string[]
type B = Array<Array<string>>; // string[][]
type C = ElementType<B>;       // string[] ← извлекли внутренний тип
```

**Применимость к BSL:** Мы можем использовать рекурсивную функцию `extract_innermost_type()` для разрешения вложенных Generic.

#### 3. Java Generic Type Inference

**Источник:** [Oracle Java Generics Tutorial](https://docs.oracle.com/javase/tutorial/java/generics/genTypeInference.html)

**Ключевая идея:** Java выводит Generic параметры из аргументов метода:

```java
static <T> List<T> emptyList() { return new ArrayList<>(); }

List<String> list = emptyList(); // T = String inferred from target type
```

**Применимость к BSL:** В 1С НЕТ target type inference (нет объявлений типов переменных). Мы должны выводить тип из вызовов методов.

---

### Архитектурные варианты

#### Вариант A: Рекурсивная резолюция в `resolve_generic()`

**Идея:** Разрешать вложенные Generic рекурсивно при парсинге типа.

```rust
// shared/src/domain/resolver.rs

pub fn resolve_generic(&self, generic_str: &str) -> TypeResolution {
    // Парсим: "Массив<Массив<Строка>>"
    if let Some((base_type, params_str)) = self.parse_generic_syntax(generic_str) {
        let type_params: Vec<&str> = self.split_generic_params(params_str);

        let mut concrete_params = Vec::new();
        for param in type_params {
            // ✅ РЕКУРСИЯ: если параметр — Generic, разрешаем его
            if param.contains('<') && param.contains('>') {
                let nested_resolution = self.resolve_generic(param);

                // Конвертируем GenericType обратно в ConcreteType
                let nested_concrete = match nested_resolution.result {
                    ResolutionResult::Generic(gt) => {
                        // ⚠️ ПРОБЛЕМА: GenericType != ConcreteType!
                        // Нужно обернуть в новый вариант ConcreteType::Generic
                        ConcreteType::NestedGeneric(gt) // ← НОВЫЙ ВАРИАНТ!
                    }
                    ResolutionResult::Concrete(ct) => ct,
                    _ => continue,
                };

                concrete_params.push(nested_concrete);
            } else {
                // Обычное разрешение
                let resolved = self.resolve_expression_sync(param);
                if let ResolutionResult::Concrete(ct) = resolved.result {
                    concrete_params.push(ct);
                }
            }
        }

        TypeResolution::generic(GenericType {
            base_type: base_type.to_string(),
            type_params: concrete_params,
        })
    } else {
        TypeResolution::unknown()
    }
}
```

**Плюсы:**
- ✅ Поддержка произвольной глубины вложенности
- ✅ Логически простая рекурсия

**Минусы:**
- ❌ Требует расширения `ConcreteType` новым вариантом `NestedGeneric(GenericType)`
- ❌ Усложнение всех match patterns на `ConcreteType`
- ❌ Парсинг вложенных `<>` нетривиален (нужен счётчик скобок)

---

#### Вариант B: Flattening — преобразование в цепочку

**Идея:** Вместо хранения вложенных Generic, представлять их как цепочку применений типов.

```rust
// Вместо:
// Массив<Массив<Строка>>  ← вложенный Generic

// Храним:
// "Массив применённый к (Массив применённый к Строка)"  ← flat chain

pub struct GenericTypeChain {
    base_type: String,
    type_params: Vec<TypeNode>,
}

pub enum TypeNode {
    Concrete(ConcreteType),
    Generic(GenericTypeChain), // ← рекурсивный вариант
}
```

**Плюсы:**
- ✅ Явное моделирование вложенности
- ✅ Не нужно менять `ConcreteType`

**Минусы:**
- ❌ Дублирование структур (`GenericType` vs `GenericTypeChain`)
- ❌ Два пути обработки Generic типов

---

#### Вариант C: Ограничение глубины вложенности

**Идея:** Поддерживать только 1 уровень вложенности (99% случаев в 1С).

```rust
pub fn resolve_generic(&self, generic_str: &str) -> TypeResolution {
    // Проверяем глубину вложенности
    let nesting_depth = self.count_nesting_depth(generic_str);

    if nesting_depth > 1 {
        // ⚠️ ЧЕСТНОСТЬ: говорим, что не поддерживаем
        return TypeResolution::inferred(
            0.3, // 30% certainty — синтаксис распарсили, но вложенность не поддерживаем
            ConcreteType::Platform(PlatformType {
                name: format!("{}...", &generic_str[..20]) // обрезаем
            })
        ).with_note("Nested generics depth > 1 not supported yet");
    }

    // Обычная резолюция для depth <= 1
    // ...
}
```

**Плюсы:**
- ✅ Простота реализации — 0 изменений в `ConcreteType`
- ✅ Честная градуальная типизация (Low certainty = не поддерживаем)
- ✅ Покрывает 99% реальных случаев в 1С

**Минусы:**
- ❌ Ограничение функциональности

---

### Рекомендация

**🎯 Вариант C — Ограничение глубины вложенности**

**Обоснование:**
1. **Практическая ценность для 1С:** В коде 1С крайне редко встречаются вложенные Generic типы
2. **Честность:** Градуальная типизация позволяет возвращать `Inferred(0.3)` для неподдерживаемых случаев
3. **Простота:** 0 breaking changes, минимальная модификация кода
4. **Future-proof:** При необходимости можно расширить до Варианта A в будущем

---

### Примеры кода

#### 1. Парсинг вложенных Generic с учётом вложенности

```rust
// shared/src/domain/resolver.rs

impl TypeResolver {
    /// Подсчёт глубины вложенности Generic типов
    fn count_nesting_depth(&self, generic_str: &str) -> usize {
        let mut depth = 0;
        let mut max_depth = 0;

        for ch in generic_str.chars() {
            match ch {
                '<' => {
                    depth += 1;
                    max_depth = max_depth.max(depth);
                }
                '>' => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth
    }

    /// Улучшенная резолюция Generic с проверкой вложенности
    pub fn resolve_generic(&self, generic_str: &str) -> TypeResolution {
        use crate::domain::types::{ResolutionResult, GenericType, Certainty, ResolutionSource};

        // 1. Проверяем глубину вложенности
        let nesting_depth = self.count_nesting_depth(generic_str);

        if nesting_depth > 1 {
            tracing::warn!(
                "⚠️ Nested generic depth {} exceeds supported limit (1): {}",
                nesting_depth,
                generic_str
            );

            // Возвращаем Unknown с пояснением
            return TypeResolution {
                certainty: Certainty::Inferred(0.3),
                result: ResolutionResult::Concrete(
                    ConcreteType::Platform(PlatformType {
                        name: generic_str.to_string(),
                    })
                ),
                source: ResolutionSource::Inferred,
                metadata: ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec![format!(
                        "Nested generic types (depth {}) are not fully supported yet. \
                         Consider flattening the structure.",
                        nesting_depth
                    )],
                },
                active_facet: None,
                available_facets: vec![],
            };
        }

        // 2. Обычная резолюция для простых Generic (depth <= 1)
        if let Some((base_type, params_str)) = self.parse_generic_syntax(generic_str) {
            // ... существующий код ...
        }

        TypeResolution::unknown()
    }
}
```

#### 2. Тесты для вложенных Generic

```rust
// shared/src/domain/resolver/resolver_generic_tests.rs

#[test]
fn test_nested_generic_depth_limit() {
    let resolver = create_test_resolver();

    // Depth 1 — должно работать
    let simple = resolver.resolve_generic("Массив<Строка>");
    assert!(matches!(simple.certainty, Certainty::Known));

    // Depth 2 — НЕ поддерживается (честно возвращаем Inferred)
    let nested = resolver.resolve_generic("Массив<Массив<Строка>>");

    match nested.certainty {
        Certainty::Inferred(val) => {
            assert!(val <= 0.5, "Nested generics должны иметь низкую certainty");
        }
        _ => panic!("Expected Inferred certainty for nested generics"),
    }

    assert!(!nested.metadata.notes.is_empty());
    assert!(nested.metadata.notes[0].contains("not fully supported"));
}

#[test]
fn test_count_nesting_depth() {
    let resolver = create_test_resolver();

    assert_eq!(resolver.count_nesting_depth("Массив<Строка>"), 1);
    assert_eq!(resolver.count_nesting_depth("Массив<Массив<Строка>>"), 2);
    assert_eq!(resolver.count_nesting_depth("Соответствие<Строка, Число>"), 1);
    assert_eq!(resolver.count_nesting_depth("Массив<Соответствие<Строка, Массив<Число>>>"), 3);
}
```

---

### Оценка

| Параметр | Значение |
|----------|----------|
| **Трудозатраты** | 2-3 дня (с тестами) |
| **Сложность** | MEDIUM (парсинг вложенных `<>`) |
| **Риски** | LOW (ограничение функциональности приемлемо) |
| **Ценность** | LOW (редко используется в 1С) |

**Рекомендация:** ⚠️ **Future Scope** — реализовать ПОСЛЕ Milestone 3.x, когда появится реальная потребность.

---

## 🎨 Направление 2: Специализация Generic коллекций

### Найденные решения

#### 1. Java Generic Method Type Inference

**Источник:** [Oracle - Type Inference](https://docs.oracle.com/javase/tutorial/java/generics/genTypeInference.html)

**Ключевая идея:** Компилятор Java выводит тип из аргументов метода:

```java
public static <T> void addToList(List<T> list, T element) {
    list.add(element);
}

List<String> list = new ArrayList<>();
addToList(list, "text"); // T = String inferred from "text" argument
```

**Применимость к BSL:**

```bsl
МассивСтрок = Новый Массив();
МассивСтрок.Добавить("текст");  // ← выводим T = Строка из аргумента "текст"
```

Мы можем отслеживать вызовы методов `Добавить()`, `Вставить()` для inference типа элемента.

#### 2. C# Generic Collections

**Источник:** [Microsoft - Generic Methods](https://learn.microsoft.com/en-us/dotnet/csharp/programming-guide/generics/generic-methods)

**Ключевая идея:** Generic constraints и type inference работают вместе:

```csharp
public void Add<T>(List<T> list, T item) where T : IComparable {
    list.Add(item);
}
```

**Применимость к BSL:** В 1С нет constraints, но мы можем использовать flow-sensitive анализ для отслеживания типа элементов.

#### 3. Scala Collections Optimization

**Источник:** [Interflow - Flow-Sensitive Type Inference](https://www.researchgate.net/publication/327735826_Interflow_interprocedural_flow-sensitive_type_inference_and_method_duplication)

**Ключевая идея:** Interprocedural flow-sensitive анализ отслеживает Generic параметры через вызовы методов:

```scala
val list = new mutable.ListBuffer[Int]()
list += 42         // infer element type = Int
list += "text"     // ERROR: type mismatch
```

**Применимость к BSL:** Наша система уже имеет `FlowSensitiveVisitor` (Milestone 2.8), можем расширить для Generic inference.

---

### Архитектурные варианты

#### Вариант A: Flow-Sensitive Generic Inference

**Идея:** Отслеживать вызовы `Добавить()`, `Вставить()` для вывода типа элементов коллекции.

```rust
// shared/src/ir/flow_sensitive_visitor.rs

impl FlowSensitiveVisitor {
    fn visit_method_call(&mut self, method_call: &MethodCall) {
        let receiver_type = self.resolve_expression(&method_call.receiver);

        // Обнаружили вызов метода Добавить на коллекции
        if self.is_collection_add_method(&method_call.method_name) {
            self.infer_collection_element_type(
                &method_call.receiver_name,
                &method_call.arguments[0]
            );
        }
    }

    fn infer_collection_element_type(&mut self, collection_var: &str, element_expr: &Expression) {
        // Выводим тип элемента из аргумента
        let element_type = self.resolve_expression(element_expr);

        // Обновляем тип переменной на Generic с параметром
        let current_type = self.symbol_table.get_type(collection_var);

        if let Some(ConcreteType::Platform(pt)) = current_type {
            if pt.name == "Массив" {
                // Обновляем: Массив → Массив<element_type>
                self.symbol_table.update_type(
                    collection_var,
                    TypeHint::Inferred(format!("Массив<{}>", element_type))
                );
            }
        }
    }
}
```

**Плюсы:**
- ✅ Автоматический inference без аннотаций типов
- ✅ Интеграция с существующим flow-sensitive анализом
- ✅ Работает для простых случаев (один тип элементов)

**Минусы:**
- ❌ Не работает для пустых коллекций
- ❌ Сложность при multiple types (Union inference нужен)
- ❌ Требует межпроцедурный анализ для передачи коллекций

---

#### Вариант B: Explicit Generic Metadata для Platform Types

**Идея:** Добавить в `RawTypeData` информацию о Generic параметрах для коллекций платформы.

```rust
// shared/src/domain/types.rs

pub struct RawTypeData {
    pub name: String,
    pub methods: Vec<RawMethodData>,

    // ✅ НОВОЕ: Generic метаданные для коллекций
    pub generic_info: Option<GenericMetadata>,
}

pub struct GenericMetadata {
    pub base_type: String,           // "Массив", "Соответствие"
    pub type_param_count: usize,     // 1 для Массив, 2 для Соответствие
    pub inference_methods: Vec<InferenceMethod>,
}

pub struct InferenceMethod {
    pub method_name: String,         // "Добавить", "Вставить"
    pub inferred_param_index: usize, // Какой параметр метода определяет Generic тип
}
```

**Пример использования:**

```rust
// При загрузке Platform Types из Syntax Helper
let array_metadata = GenericMetadata {
    base_type: "Массив".to_string(),
    type_param_count: 1,
    inference_methods: vec![
        InferenceMethod {
            method_name: "Добавить".to_string(),
            inferred_param_index: 0, // первый параметр определяет тип элемента
        },
        InferenceMethod {
            method_name: "Вставить".to_string(),
            inferred_param_index: 1, // второй параметр (индекс + значение)
        },
    ],
};
```

**Плюсы:**
- ✅ Явная декларация Generic поведения
- ✅ Легко расширяется для новых коллекций
- ✅ Не требует сложного flow-sensitive анализа

**Минусы:**
- ❌ Требует ручной конфигурации для каждой коллекции
- ❌ Не обнаруживает Generic автоматически

---

#### Вариант C: Гибридный подход (РЕКОМЕНДАЦИЯ)

**Идея:** Комбинация Варианта A (flow-sensitive inference) + Вариант B (metadata).

1. **Загрузка метаданных:** Используем Вариант B для известных Platform Types
2. **Runtime inference:** Используем Вариант A для вывода конкретных типов

```rust
// Двухэтапный процесс:

// ЭТАП 1: Загрузка Generic метаданных (statically)
// backend/src/data/loaders/platform_types_loader.rs

impl PlatformTypesLoader {
    fn enrich_collection_types(&mut self) {
        // Массив
        self.add_generic_metadata("Массив", GenericMetadata {
            type_param_count: 1,
            inference_methods: vec![
                InferenceMethod { method_name: "Добавить", param_index: 0 },
            ],
        });

        // Соответствие
        self.add_generic_metadata("Соответствие", GenericMetadata {
            type_param_count: 2,
            inference_methods: vec![
                InferenceMethod { method_name: "Вставить", param_index: 0 }, // ключ
                InferenceMethod { method_name: "Вставить", param_index: 1 }, // значение
            ],
        });

        // ФиксированныйМассив
        self.add_generic_metadata("ФиксированныйМассив", GenericMetadata {
            type_param_count: 1,
            inference_methods: vec![], // Неизменяемый — нет методов добавления
        });

        // СписокЗначений
        self.add_generic_metadata("СписокЗначений", GenericMetadata {
            type_param_count: 1,
            inference_methods: vec![
                InferenceMethod { method_name: "Добавить", param_index: 0 },
            ],
        });
    }
}

// ЭТАП 2: Flow-sensitive inference (runtime)
// shared/src/ir/flow_sensitive_visitor.rs

impl FlowSensitiveVisitor {
    fn visit_method_call(&mut self, call: &MethodCall) {
        let receiver_type = self.get_variable_type(&call.receiver);

        // Проверяем: есть ли у типа Generic метаданные?
        if let Some(generic_meta) = self.repository.get_generic_metadata(&receiver_type) {
            // Ищем метод в inference_methods
            for inference_method in &generic_meta.inference_methods {
                if call.method_name == inference_method.method_name {
                    // Выводим тип параметра
                    let param_type = self.resolve_expression(
                        &call.arguments[inference_method.param_index]
                    );

                    // Обновляем тип коллекции на Generic
                    self.specialize_collection_type(
                        &call.receiver,
                        &receiver_type,
                        param_type
                    );
                }
            }
        }
    }

    fn specialize_collection_type(
        &mut self,
        var_name: &str,
        base_type: &str,
        element_type: String,
    ) {
        // Массив → Массив<Строка>
        let specialized = format!("{}<{}>", base_type, element_type);

        self.symbol_table.update_type(
            var_name,
            TypeHint::Inferred(specialized)
        );
    }
}
```

**Плюсы:**
- ✅ Декларативное описание Generic коллекций
- ✅ Автоматический inference на основе flow-анализа
- ✅ Расширяемость для новых коллекций

**Минусы:**
- ❌ Требует модификации `RawTypeData` структуры
- ❌ Дополнительная логика в FlowSensitiveVisitor

---

### Рекомендация

**🎯 Вариант C — Гибридный подход**

**Обоснование:**
1. **Практическая ценность:** Коллекции (Массив, Соответствие) — самые частые типы в 1С
2. **Масштабируемость:** Легко добавить новые коллекции через метаданные
3. **Точность:** Flow-sensitive inference обеспечивает точный вывод типов
4. **Готовая инфраструктура:** FlowSensitiveVisitor уже реализован (Milestone 2.8)

---

### Приоритизация коллекций для MVP

| Приоритет | Коллекция | Частота использования | Generic параметры | Сложность inference |
|-----------|-----------|----------------------|-------------------|---------------------|
| **1** | Массив | ⭐⭐⭐⭐⭐ Очень высокая | 1 (элемент) | LOW |
| **2** | Соответствие | ⭐⭐⭐⭐ Высокая | 2 (ключ, значение) | MEDIUM |
| **3** | СписокЗначений | ⭐⭐⭐ Средняя | 1 (элемент) | LOW |
| **4** | ФиксированныйМассив | ⭐⭐ Низкая | 1 (элемент) | LOW (из конструктора) |

**Рекомендация для MVP:**
1. Реализовать Массив + СписокЗначений (простые, 1 параметр)
2. Затем Соответствие (2 параметра, сложнее inference)
3. ФиксированныйМассив — бонус (создаётся из Массива)

---

### Примеры кода

#### 1. Расширение RawTypeData

```rust
// shared/src/domain/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTypeData {
    pub name: String,
    pub category: String,
    pub description: String,
    pub methods: Vec<RawMethodData>,
    pub properties: Vec<RawPropertyData>,
    pub attributes: Vec<RawAttributeData>,
    pub tabular_sections: Vec<RawTabularSectionData>,
    pub facets: Vec<FacetKind>,
    pub enum_values: Vec<String>,

    // ✅ НОВОЕ: Generic метаданные для коллекций
    #[serde(default)]
    pub generic_info: Option<GenericInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericInfo {
    /// Базовый тип (например, "Массив")
    pub base_type: String,

    /// Количество типовых параметров (1 для Массив, 2 для Соответствие)
    pub type_param_count: usize,

    /// Методы, которые позволяют вывести тип параметра
    pub inference_methods: Vec<InferenceMethodInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceMethodInfo {
    /// Имя метода (например, "Добавить")
    pub method_name: String,

    /// Индекс параметра, который определяет тип (0-based)
    pub param_index: usize,

    /// Какой Generic параметр этот аргумент определяет (0 для T, 1 для K в Map<K,V>)
    pub determines_type_param: usize,
}
```

#### 2. Инициализация Generic метаданных

```rust
// backend/src/data/loaders/platform_types_loader.rs

impl PlatformTypesLoader {
    /// Обогащение Platform Types информацией о Generic коллекциях
    pub fn enrich_collection_generics(&mut self) {
        // 1. Массив
        if let Some(array_type) = self.types.get_mut("Массив") {
            array_type.generic_info = Some(GenericInfo {
                base_type: "Массив".to_string(),
                type_param_count: 1,
                inference_methods: vec![
                    InferenceMethodInfo {
                        method_name: "Добавить".to_string(),
                        param_index: 0,
                        determines_type_param: 0, // T в Массив<T>
                    },
                    InferenceMethodInfo {
                        method_name: "Вставить".to_string(),
                        param_index: 1, // индекс + значение
                        determines_type_param: 0,
                    },
                ],
            });
        }

        // 2. Соответствие
        if let Some(map_type) = self.types.get_mut("Соответствие") {
            map_type.generic_info = Some(GenericInfo {
                base_type: "Соответствие".to_string(),
                type_param_count: 2,
                inference_methods: vec![
                    InferenceMethodInfo {
                        method_name: "Вставить".to_string(),
                        param_index: 0, // ключ
                        determines_type_param: 0, // K в Соответствие<K, V>
                    },
                    InferenceMethodInfo {
                        method_name: "Вставить".to_string(),
                        param_index: 1, // значение
                        determines_type_param: 1, // V в Соответствие<K, V>
                    },
                ],
            });
        }

        // 3. СписокЗначений
        if let Some(list_type) = self.types.get_mut("СписокЗначений") {
            list_type.generic_info = Some(GenericInfo {
                base_type: "СписокЗначений".to_string(),
                type_param_count: 1,
                inference_methods: vec![
                    InferenceMethodInfo {
                        method_name: "Добавить".to_string(),
                        param_index: 0,
                        determines_type_param: 0,
                    },
                ],
            });
        }

        // 4. ФиксированныйМассив (создаётся из Массива, нет методов добавления)
        if let Some(fixed_array_type) = self.types.get_mut("ФиксированныйМассив") {
            fixed_array_type.generic_info = Some(GenericInfo {
                base_type: "ФиксированныйМассив".to_string(),
                type_param_count: 1,
                inference_methods: vec![], // Inference из конструктора
            });
        }
    }
}
```

#### 3. Flow-Sensitive Generic Inference

```rust
// shared/src/ir/flow_sensitive_visitor.rs

use crate::domain::types::{GenericInfo, TypeResolution};

impl FlowSensitiveVisitor {
    /// Обработка вызова метода для Generic inference
    fn visit_method_call(&mut self, call: &MethodCall) {
        let receiver_name = &call.receiver;

        // Получаем текущий тип переменной
        let current_type = self.symbol_table.get_variable(receiver_name)
            .map(|v| &v.type_hint)
            .cloned()
            .unwrap_or(TypeHint::Unknown);

        // Резолвим базовый тип
        let base_type_name = match current_type {
            TypeHint::Explicit(ref t) | TypeHint::Inferred(ref t) => t.clone(),
            TypeHint::Unknown => return,
        };

        // Проверяем: есть ли у типа Generic метаданные?
        let raw_type = self.repository.find_type(&base_type_name);
        let generic_info = raw_type.and_then(|rt| rt.generic_info.clone());

        if let Some(generic_info) = generic_info {
            // Проверяем каждый inference method
            for inference_method in &generic_info.inference_methods {
                if call.method_name == inference_method.method_name {
                    // Выводим тип из аргумента
                    if let Some(arg_expr) = call.arguments.get(inference_method.param_index) {
                        let arg_type = self.resolve_expression(arg_expr);

                        tracing::debug!(
                            "🔍 Generic inference: {}.{}({}) → тип параметра {}",
                            receiver_name,
                            call.method_name,
                            arg_type,
                            inference_method.determines_type_param
                        );

                        // Специализируем Generic тип
                        self.specialize_generic_collection(
                            receiver_name,
                            &generic_info.base_type,
                            inference_method.determines_type_param,
                            arg_type,
                        );
                    }
                }
            }
        }
    }

    /// Специализация Generic коллекции
    fn specialize_generic_collection(
        &mut self,
        var_name: &str,
        base_type: &str,
        param_index: usize,
        element_type: String,
    ) {
        // Получаем текущий тип (может быть уже Generic)
        let current_var = self.symbol_table.get_variable_mut(var_name);

        if let Some(var) = current_var {
            match &var.type_hint {
                // Простой тип → Generic
                TypeHint::Explicit(t) | TypeHint::Inferred(t) if !t.contains('<') => {
                    // Массив → Массив<Строка>
                    var.type_hint = TypeHint::Inferred(
                        format!("{}<{}>", base_type, element_type)
                    );
                }

                // Уже Generic → обновляем параметр (для Соответствие)
                TypeHint::Inferred(t) if t.contains('<') => {
                    // Соответствие<Строка, ?> + Вставить(key, 42) → Соответствие<Строка, Число>
                    // TODO: Реализовать merge Generic параметров
                }

                _ => {}
            }
        }
    }
}
```

#### 4. Тесты для Generic Collections

```rust
// backend/tests/generic_collections_inference_test.rs

use bsl_backend::application::type_system_service::TypeSystemService;
use bsl_backend::system::{AnalysisCache, IrCache, ParserCoordinator, SystemCoordinator};
use std::sync::Arc;

#[tokio::test]
async fn test_array_generic_inference_from_add() {
    let coordinator = SystemCoordinator::new().await.unwrap();
    let service = coordinator.type_system_service();

    let source = r#"
    Процедура Тест()
        МассивСтрок = Новый Массив();
        МассивСтрок.Добавить("текст");
        МассивСтрок.Добавить("другой текст");
    КонецПроцедуры
    "#;

    // Hover на переменной после вызова Добавить
    let hover = service.get_hover_info(source, 3, 10).await.unwrap();

    assert!(hover.is_some());
    let text = hover.unwrap();

    // Проверяем: тип должен быть Массив<Строка>
    assert!(text.contains("Массив<Строка>"), "Hover text: {}", text);
    assert!(text.contains("100%"), "Should be Known certainty");
}

#[tokio::test]
async fn test_map_generic_inference_from_insert() {
    let coordinator = SystemCoordinator::new().await.unwrap();
    let service = coordinator.type_system_service();

    let source = r#"
    Процедура Тест()
        Словарь = Новый Соответствие();
        Словарь.Вставить("ключ", 42);
        Словарь.Вставить("другой", 100);
    КонецПроцедуры
    "#;

    let hover = service.get_hover_info(source, 3, 10).await.unwrap();
    let text = hover.unwrap();

    // Проверяем: тип должен быть Соответствие<Строка, Число>
    assert!(text.contains("Соответствие<Строка, Число>"), "Hover text: {}", text);
}

#[tokio::test]
async fn test_fixed_array_inference_from_constructor() {
    let source = r#"
    Процедура Тест()
        МассивЧисел = Новый Массив();
        МассивЧисел.Добавить(1);
        МассивЧисел.Добавить(2);

        ФиксМассив = Новый ФиксированныйМассив(МассивЧисел);
    КонецПроцедуры
    "#;

    let coordinator = SystemCoordinator::new().await.unwrap();
    let service = coordinator.type_system_service();
    let hover = service.get_hover_info(source, 6, 10).await.unwrap();
    let text = hover.unwrap();

    // ФиксированныйМассив должен наследовать тип элементов из Массива
    assert!(text.contains("ФиксированныйМассив<Число>"));
}
```

---

### Оценка

| Параметр | Значение |
|----------|----------|
| **Трудозатраты** | 3-5 дней (включая тесты для 4 коллекций) |
| **Сложность** | MEDIUM (flow-sensitive inference + метаданные) |
| **Риски** | MEDIUM (нужно учесть Union types для разных элементов) |
| **Ценность** | ⭐⭐⭐⭐⭐ VERY HIGH (Массив/Соответствие — основа 1С) |

**Рекомендация:** ✅ **Высокий приоритет** — реализовать ПОСЛЕ HoverFormatter.

---

## 🎨 Направление 3: Выделение HoverFormatter

### Найденные решения

#### 1. Visitor Pattern для форматирования

**Источник:** [Software Patterns Lexicon - Visitor Pattern](https://softwarepatternslexicon.com/object-oriented/5/12/)

**Ключевая идея:** Visitor pattern разделяет алгоритм от структуры данных:

```rust
trait TypeFormatter {
    fn format_concrete(&self, ct: &ConcreteType) -> String;
    fn format_generic(&self, gt: &GenericType) -> String;
    fn format_union(&self, ut: &Vec<WeightedType>) -> String;
}

struct MarkdownFormatter;
impl TypeFormatter for MarkdownFormatter {
    fn format_concrete(&self, ct: &ConcreteType) -> String {
        format!("**Type:** `{:?}`", ct)
    }
}
```

**Применимость к BSL:** Отлично подходит для расширяемости (Markdown, HTML, JSON форматы).

#### 2. Strategy Pattern для форматирования

**Источник:** [LeedrickDotNet - Strategy vs Visitor](http://leedrickdotnet.blogspot.com/2007/01/strategy-pattern-vs-visitor-pattern.html)

**Ключевая идея:** Strategy инкапсулирует семейство алгоритмов:

```rust
enum FormatStrategy {
    Markdown,
    HTML,
    PlainText,
}

struct HoverFormatter {
    strategy: FormatStrategy,
}

impl HoverFormatter {
    fn format(&self, resolution: &TypeResolution) -> String {
        match self.strategy {
            FormatStrategy::Markdown => self.format_markdown(resolution),
            FormatStrategy::HTML => self.format_html(resolution),
            FormatStrategy::PlainText => self.format_plain(resolution),
        }
    }
}
```

**Применимость к BSL:** Хорошо для поддержки разных форматов вывода (LSP, Web, CLI).

#### 3. Builder Pattern для составных hover текстов

**Источник:** Стандартный паттерн проектирования

**Ключевая идея:** Пошаговое конструирование сложных объектов:

```rust
struct HoverBuilder {
    sections: Vec<String>,
}

impl HoverBuilder {
    fn add_title(&mut self, text: &str) -> &mut Self {
        self.sections.push(format!("**{}**", text));
        self
    }

    fn add_methods(&mut self, methods: Vec<RawMethodData>) -> &mut Self {
        // ...
        self
    }

    fn build(&self) -> String {
        self.sections.join("\n\n")
    }
}
```

**Применимость к BSL:** Идеально для структурированного hover текста с секциями.

---

### Архитектурные варианты

#### Вариант A: Simple Formatter в Application Layer

**Идея:** Минимальный рефакторинг — просто выделить функции форматирования в отдельный модуль.

```rust
// backend/src/application/hover_formatter.rs

use bsl_shared::domain::types::{TypeResolution, GenericType, ConcreteType};
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;

/// Форматирование hover текста для LSP/Web
pub struct HoverFormatter {
    metadata_lookup: TypeMetadataLookup,
}

impl HoverFormatter {
    pub fn new(metadata_lookup: TypeMetadataLookup) -> Self {
        Self { metadata_lookup }
    }

    /// Форматирование переменной с типом
    pub fn format_variable(
        &self,
        var_name: &str,
        type_hint: &bsl_shared::ir::TypeHint,
    ) -> String {
        // Переносим логику из TypeSystemService::format_variable_hover
        // ...
    }

    /// Форматирование Generic типа
    pub fn format_generic(
        &self,
        var_name: &str,
        generic_type: &GenericType,
        resolution: &TypeResolution,
    ) -> String {
        // Переносим логику из TypeSystemService::format_generic_hover
        // ...
    }

    /// Форматирование ConcreteType имени
    pub fn format_type_name(&self, concrete: &ConcreteType) -> String {
        // Переносим логику из TypeSystemService::format_concrete_type_name
        // ...
    }
}

// backend/src/application/type_system_service.rs

impl TypeSystemService {
    pub async fn get_hover_info(...) -> Result<Option<String>> {
        // ...

        // ✅ Используем HoverFormatter вместо self.format_variable_hover
        let formatter = HoverFormatter::new(self.metadata_lookup.clone());
        let hover_text = formatter.format_variable(&var_name, &type_hint);

        Ok(Some(hover_text))
    }
}
```

**Плюсы:**
- ✅ Минимальный рефакторинг
- ✅ Не меняет публичные API
- ✅ Легко тестировать изолированно

**Минусы:**
- ❌ Всё ещё в Application Layer (не переиспользуется в CLI)
- ❌ Один формат вывода (Markdown)

---

#### Вариант B: Trait-based Formatter с поддержкой форматов

**Идея:** Используем trait для поддержки разных форматов (Markdown, HTML, JSON).

```rust
// shared/src/presentation/formatter.rs  ← НОВЫЙ модуль в shared!

use crate::domain::types::{TypeResolution, GenericType, ConcreteType};
use crate::domain::metadata_lookup::TypeMetadataLookup;

/// Trait для форматирования hover информации
pub trait TypeFormatter {
    fn format_variable(&self, var_name: &str, resolution: &TypeResolution) -> String;
    fn format_generic(&self, var_name: &str, generic: &GenericType, resolution: &TypeResolution) -> String;
    fn format_type_name(&self, concrete: &ConcreteType) -> String;
}

/// Markdown formatter для LSP hover
pub struct MarkdownFormatter {
    metadata_lookup: TypeMetadataLookup,
}

impl TypeFormatter for MarkdownFormatter {
    fn format_variable(&self, var_name: &str, resolution: &TypeResolution) -> String {
        use crate::domain::types::{Certainty, ResolutionResult};

        let mut output = format!("**Переменная:** `{}`\n", var_name);

        // Certainty badge
        let certainty_badge = match resolution.certainty {
            Certainty::Known => "🟢 Known (100%)",
            Certainty::Inferred(v) => &format!("🟡 Inferred ({:.0}%)", v * 100.0),
            Certainty::Unknown => "⚪ Unknown (0%)",
        };
        output.push_str(&format!("*Уверенность:* {}\n\n", certainty_badge));

        // Методы/свойства через TypeMetadataLookup
        let methods = self.metadata_lookup.get_methods(resolution);
        if !methods.is_empty() {
            output.push_str("📚 **Методы:**\n");
            for method in methods.iter().take(10) {
                output.push_str(&format!("- `{}`\n", method.name));
            }
        }

        output
    }

    fn format_generic(&self, var_name: &str, generic: &GenericType, resolution: &TypeResolution) -> String {
        // Generic-специфичное форматирование
        // ...
    }

    fn format_type_name(&self, concrete: &ConcreteType) -> String {
        match concrete {
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => format!("{}.{}", ct.kind.to_prefix(), ct.name),
            // ...
        }
    }
}

/// HTML formatter для Web UI
pub struct HtmlFormatter {
    metadata_lookup: TypeMetadataLookup,
}

impl TypeFormatter for HtmlFormatter {
    fn format_variable(&self, var_name: &str, resolution: &TypeResolution) -> String {
        format!(
            r#"<div class="hover">
                <h3>{}</h3>
                <span class="certainty">{:?}</span>
               </div>"#,
            var_name,
            resolution.certainty
        )
    }

    // ...
}
```

**Использование:**

```rust
// backend/src/application/type_system_service.rs

use bsl_shared::presentation::formatter::{TypeFormatter, MarkdownFormatter};

impl TypeSystemService {
    pub async fn get_hover_info(...) -> Result<Option<String>> {
        // ...

        let formatter = MarkdownFormatter::new(self.metadata_lookup.clone());
        let hover_text = formatter.format_variable(&var_name, &resolution);

        Ok(Some(hover_text))
    }
}
```

**Плюсы:**
- ✅ Поддержка разных форматов (Markdown, HTML, JSON)
- ✅ В shared крейте — переиспользуется везде (LSP, Web, CLI)
- ✅ Trait позволяет добавлять новые форматы без изменений в TypeSystemService

**Минусы:**
- ❌ Больше кода для реализации trait для каждого формата
- ❌ Нужна координация между форматами (одинаковая структура)

---

#### Вариант C: Builder Pattern для композиции hover

**Идея:** Используем Builder для пошагового конструирования hover текста.

```rust
// shared/src/presentation/hover_builder.rs

pub struct HoverBuilder {
    sections: Vec<String>,
    format: OutputFormat,
}

pub enum OutputFormat {
    Markdown,
    Html,
    PlainText,
}

impl HoverBuilder {
    pub fn new(format: OutputFormat) -> Self {
        Self {
            sections: Vec::new(),
            format,
        }
    }

    pub fn add_title(&mut self, var_name: &str) -> &mut Self {
        let title = match self.format {
            OutputFormat::Markdown => format!("**Переменная:** `{}`", var_name),
            OutputFormat::Html => format!("<h3>{}</h3>", var_name),
            OutputFormat::PlainText => format!("Переменная: {}", var_name),
        };
        self.sections.push(title);
        self
    }

    pub fn add_certainty(&mut self, certainty: &Certainty) -> &mut Self {
        let badge = match self.format {
            OutputFormat::Markdown => format!("*Уверенность:* {:?}", certainty),
            OutputFormat::Html => format!("<span class='badge'>{:?}</span>", certainty),
            OutputFormat::PlainText => format!("Уверенность: {:?}", certainty),
        };
        self.sections.push(badge);
        self
    }

    pub fn add_methods(&mut self, methods: Vec<RawMethodData>) -> &mut Self {
        if methods.is_empty() {
            return self;
        }

        let header = match self.format {
            OutputFormat::Markdown => "📚 **Методы:**",
            OutputFormat::Html => "<h4>Методы:</h4><ul>",
            OutputFormat::PlainText => "Методы:",
        };

        self.sections.push(header.to_string());

        for method in methods.iter().take(10) {
            let item = match self.format {
                OutputFormat::Markdown => format!("- `{}`", method.name),
                OutputFormat::Html => format!("<li>{}</li>", method.name),
                OutputFormat::PlainText => format!("  - {}", method.name),
            };
            self.sections.push(item);
        }

        if self.format == OutputFormat::Html {
            self.sections.push("</ul>".to_string());
        }

        self
    }

    pub fn build(&self) -> String {
        match self.format {
            OutputFormat::Markdown | OutputFormat::PlainText => {
                self.sections.join("\n")
            }
            OutputFormat::Html => {
                format!("<div class='hover'>{}</div>", self.sections.join("\n"))
            }
        }
    }
}
```

**Использование:**

```rust
// backend/src/application/type_system_service.rs

use bsl_shared::presentation::hover_builder::{HoverBuilder, OutputFormat};

impl TypeSystemService {
    pub async fn get_hover_info(...) -> Result<Option<String>> {
        // ...

        let methods = self.metadata_lookup.get_methods(&resolution);
        let properties = self.metadata_lookup.get_properties(&resolution);

        let hover_text = HoverBuilder::new(OutputFormat::Markdown)
            .add_title(&var_name)
            .add_certainty(&resolution.certainty)
            .add_methods(methods)
            .add_properties(properties)
            .build();

        Ok(Some(hover_text))
    }
}
```

**Плюсы:**
- ✅ Композиция секций hover — очень гибкий API
- ✅ Поддержка разных форматов в одном месте
- ✅ Читаемый код использования (fluent API)

**Минусы:**
- ❌ Builder pattern может быть overkill для простых случаев
- ❌ Дублирование логики форматирования внутри методов builder

---

### Рекомендация

**🎯 Вариант B — Trait-based Formatter**

**Обоснование:**
1. **Separation of Concerns:** Форматирование полностью отделено от бизнес-логики
2. **Расширяемость:** Легко добавить новые форматы (JSON для API, HTML для Web UI)
3. **Переиспользование:** В `shared` крейте — доступен для LSP, Web, CLI
4. **Тестируемость:** Каждый formatter тестируется изолированно
5. **Type Safety:** Trait гарантирует единообразие API

**Альтернатива:** Если нужна максимальная гибкость — комбинировать Вариант B (trait) + Вариант C (builder внутри каждого formatter).

---

### Примеры кода

#### 1. Определение Formatter trait

```rust
// shared/src/presentation/mod.rs

pub mod formatter;

// shared/src/presentation/formatter.rs

use crate::domain::types::{TypeResolution, GenericType, ConcreteType, Certainty};
use crate::domain::metadata_lookup::TypeMetadataLookup;

/// Trait для форматирования hover информации о типах
///
/// Разные реализации поддерживают разные форматы вывода:
/// - MarkdownFormatter — для LSP hover (VSCode)
/// - HtmlFormatter — для Web UI
/// - JsonFormatter — для API endpoints
pub trait TypeFormatter {
    /// Форматирование переменной с типом
    fn format_variable(
        &self,
        var_name: &str,
        resolution: &TypeResolution,
    ) -> String;

    /// Форматирование Generic типа (например, Массив<Строка>)
    fn format_generic(
        &self,
        var_name: &str,
        generic: &GenericType,
        resolution: &TypeResolution,
    ) -> String;

    /// Форматирование имени ConcreteType
    fn format_type_name(&self, concrete: &ConcreteType) -> String;

    /// Форматирование certainty badge
    fn format_certainty(&self, certainty: &Certainty) -> String;
}

/// Markdown formatter для LSP hover
pub struct MarkdownFormatter {
    metadata_lookup: TypeMetadataLookup,
}

impl MarkdownFormatter {
    pub fn new(metadata_lookup: TypeMetadataLookup) -> Self {
        Self { metadata_lookup }
    }
}

impl TypeFormatter for MarkdownFormatter {
    fn format_variable(
        &self,
        var_name: &str,
        resolution: &TypeResolution,
    ) -> String {
        use crate::domain::types::ResolutionResult;

        let mut output = String::new();

        // Заголовок
        output.push_str(&format!("**Переменная:** `{}`\n", var_name));

        // Тип
        let type_name = match &resolution.result {
            ResolutionResult::Concrete(ct) => self.format_type_name(ct),
            ResolutionResult::Generic(gt) => {
                return self.format_generic(var_name, gt, resolution);
            }
            ResolutionResult::Union(_) => "Union тип".to_string(),
            ResolutionResult::Dynamic => "Динамический".to_string(),
        };
        output.push_str(&format!("*Тип:* `{}`\n", type_name));

        // Certainty
        output.push_str(&format!("*Уверенность:* {}\n\n", self.format_certainty(&resolution.certainty)));

        // Методы
        let methods = self.metadata_lookup.get_methods(resolution);
        if !methods.is_empty() {
            output.push_str("📚 **Методы:**\n");
            for method in methods.iter().take(10) {
                let params = method.params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.param_type))
                    .collect::<Vec<_>>()
                    .join(", ");

                if !method.return_type.is_empty() {
                    output.push_str(&format!("- `{}({})` → `{}`\n", method.name, params, method.return_type));
                } else {
                    output.push_str(&format!("- `{}({})`\n", method.name, params));
                }
            }
            if methods.len() > 10 {
                output.push_str(&format!("- ... и ещё {} методов\n", methods.len() - 10));
            }
            output.push('\n');
        }

        // Свойства
        let properties = self.metadata_lookup.get_properties(resolution);
        if !properties.is_empty() {
            output.push_str("📦 **Свойства:**\n");
            for prop in properties.iter().take(10) {
                output.push_str(&format!("- `{}`: `{}`\n", prop.name, prop.prop_type));
            }
            if properties.len() > 10 {
                output.push_str(&format!("- ... и ещё {} свойств\n", properties.len() - 10));
            }
        }

        output
    }

    fn format_generic(
        &self,
        var_name: &str,
        generic: &GenericType,
        resolution: &TypeResolution,
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!("**Переменная:** `{}`\n", var_name));

        // Полное Generic имя
        let full_name = if let Some(param) = generic.type_params.first() {
            format!("{}<{}>", generic.base_type, self.format_type_name(param))
        } else {
            generic.base_type.clone()
        };
        output.push_str(&format!("*Тип:* `{}`\n", full_name));
        output.push_str(&format!("*Уверенность:* {}\n\n", self.format_certainty(&resolution.certainty)));

        // Специфичная информация для табличных частей
        if generic.base_type == "ТабличнаяЧасть" {
            if let Some(ConcreteType::TabularRow(row_type)) = generic.type_params.first() {
                output.push_str(&format!("📋 *Табличная часть:* `{}`\n", row_type.tabular_section_name));
                output.push_str(&format!("📄 *Родительский объект:* `{}`\n\n", row_type.parent_type));

                // Атрибуты строки
                if !row_type.attributes.is_empty() {
                    output.push_str("📦 **Атрибуты строки:**\n");
                    for attr in row_type.attributes.iter().take(15) {
                        if !attr.attr_type.is_empty() {
                            output.push_str(&format!("- `{}`: `{}`\n", attr.name, attr.attr_type));
                        } else {
                            output.push_str(&format!("- `{}`\n", attr.name));
                        }
                    }
                }
            }
        }

        // Методы коллекции
        let methods = self.metadata_lookup.get_methods(resolution);
        if !methods.is_empty() {
            output.push_str("📚 **Методы коллекции:**\n");
            for method in methods.iter().take(10) {
                output.push_str(&format!("- `{}`\n", method.name));
            }
        }

        output
    }

    fn format_type_name(&self, concrete: &ConcreteType) -> String {
        match concrete {
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => {
                format!("{}.{}", ct.kind.to_prefix(), ct.name)
            }
            ConcreteType::Primitive(prim) => format!("{:?}", prim),
            ConcreteType::Special(spec) => format!("{:?}", spec),
            ConcreteType::GlobalFunction(gf) => gf.name.clone(),
            ConcreteType::TabularRow(tr) => tr.get_full_name(),
        }
    }

    fn format_certainty(&self, certainty: &Certainty) -> String {
        match certainty {
            Certainty::Known => "🟢 Known (100%)".to_string(),
            Certainty::Inferred(val) => format!("🟡 Inferred ({:.0}%)", val * 100.0),
            Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
        }
    }
}
```

#### 2. Использование в TypeSystemService

```rust
// backend/src/application/type_system_service.rs

use bsl_shared::presentation::formatter::{TypeFormatter, MarkdownFormatter};

impl TypeSystemService {
    pub async fn get_hover_info(
        &self,
        file_content: &str,
        line: u32,
        column: u32,
    ) -> Result<Option<String>> {
        // ... существующая логика парсинга и поиска переменной ...

        // Создаём formatter
        let formatter = MarkdownFormatter::new(self.metadata_lookup.clone());

        // Резолвим тип
        let resolution = self.analysis_engine.resolve_type(&type_name);

        // Форматируем hover
        let hover_text = formatter.format_variable(&var_name, &resolution);

        Ok(Some(hover_text))
    }
}
```

#### 3. Тесты для HoverFormatter

```rust
// shared/src/presentation/formatter/tests.rs

use crate::domain::types::{TypeResolution, Certainty, ResolutionResult, ConcreteType, PlatformType};
use crate::domain::repository::InMemoryTypeRepository;
use crate::domain::metadata_lookup::TypeMetadataLookup;
use crate::presentation::formatter::{TypeFormatter, MarkdownFormatter};
use std::sync::Arc;

#[test]
fn test_markdown_formatter_simple_type() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let lookup = TypeMetadataLookup::new(repo.clone());
    let formatter = MarkdownFormatter::new(lookup);

    let resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(
            ConcreteType::Platform(PlatformType { name: "Строка".to_string() })
        ),
        source: Default::default(),
        metadata: Default::default(),
        active_facet: None,
        available_facets: vec![],
    };

    let output = formatter.format_variable("ИмяПеременной", &resolution);

    assert!(output.contains("**Переменная:** `ИмяПеременной`"));
    assert!(output.contains("*Тип:* `Строка`"));
    assert!(output.contains("🟢 Known (100%)"));
}

#[test]
fn test_markdown_formatter_certainty_badges() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let lookup = TypeMetadataLookup::new(repo);
    let formatter = MarkdownFormatter::new(lookup);

    assert_eq!(formatter.format_certainty(&Certainty::Known), "🟢 Known (100%)");
    assert_eq!(formatter.format_certainty(&Certainty::Inferred(0.8)), "🟡 Inferred (80%)");
    assert_eq!(formatter.format_certainty(&Certainty::Unknown), "⚪ Unknown (0%)");
}
```

---

### Оценка

| Параметр | Значение |
|----------|----------|
| **Трудозатраты** | 1-2 дня (рефакторинг + тесты) |
| **Сложность** | LOW (простой рефакторинг без breaking changes) |
| **Риски** | VERY LOW (изолированные изменения) |
| **Ценность** | ⭐⭐⭐⭐ HIGH (улучшает maintainability, подготовка к Direction 2) |

**Рекомендация:** ✅ **ПЕРВЫЙ ПРИОРИТЕТ** — начать с этого направления.

---

## 📋 Итоговая roadmap

### Phase 1: HoverFormatter (1-2 дня) — HIGHEST PRIORITY

**Цель:** Выделить логику форматирования в отдельный компонент.

**Задачи:**
1. ✅ Создать `shared/src/presentation/formatter.rs`
2. ✅ Определить `TypeFormatter` trait
3. ✅ Реализовать `MarkdownFormatter`
4. ✅ Обновить `TypeSystemService` для использования formatter
5. ✅ Написать unit-тесты для formatter

**Критерии приёмки:**
- Все существующие тесты hover проходят
- `MarkdownFormatter` покрыт unit-тестами
- 0 breaking changes в публичных API

---

### Phase 2: Generic Collections (3-5 дней) — HIGH PRIORITY

**Цель:** Поддержка Массив<T>, Соответствие<K,V>, СписокЗначений<T>.

**Задачи:**
1. ✅ Расширить `RawTypeData` полем `generic_info`
2. ✅ Реализовать `GenericInfo` структуру
3. ✅ Обогатить Platform Types метаданными Generic коллекций
4. ✅ Расширить `FlowSensitiveVisitor` для Generic inference
5. ✅ Обновить `MarkdownFormatter` для Generic коллекций
6. ✅ Написать интеграционные тесты

**Критерии приёмки:**
- Hover на `Массив.Добавить("текст")` показывает `Массив<Строка>`
- Hover на `Соответствие.Вставить("key", 42)` показывает `Соответствие<Строка, Число>`
- Certainty = Known (100%) для выведенных Generic типов

---

### Phase 3: Nested Generics (Future Scope) — LOW PRIORITY

**Цель:** Поддержка вложенных Generic типов (depth > 1).

**Условие:** Реализовать ТОЛЬКО если появится реальная потребность в проекте.

**Задачи:**
1. Реализовать `count_nesting_depth()` в `TypeResolver`
2. Добавить ограничение глубины с честным `Certainty::Inferred(0.3)`
3. Написать тесты для nested Generic

**Критерии приёмки:**
- `Массив<Массив<Строка>>` возвращает `Inferred(30%)` с пояснением

---

## 🎯 Финальные рекомендации

### Value/Effort Matrix

```
     │ HIGH VALUE
     │
  10 │  ┌────────────────┐
     │  │ Direction 2:   │
     │  │ Generic        │
     │  │ Collections    │
   8 │  └────────────────┘
     │
     │  ┌────────────────┐
   6 │  │ Direction 3:   │
     │  │ HoverFormatter │
     │  └────────────────┘
   4 │
     │
     │              ┌────────────────┐
   2 │              │ Direction 1:   │
     │              │ Nested Generics│
     │              └────────────────┘
  LOW│
     └───────────────────────────────── EFFORT
        LOW    2    4    6    8    HIGH
```

### Порядок реализации

1. **Сначала:** Direction 3 (HoverFormatter) — quick win, упрощает Direction 2
2. **Затем:** Direction 2 (Generic Collections) — максимальная ценность для пользователей
3. **Потом:** Direction 1 (Nested Generics) — только если появится реальная потребность

### Ключевые принципы

- ✅ **Gradual Typing:** Честно возвращаем низкий certainty для неподдерживаемых случаев
- ✅ **YAGNI:** Не реализуем Nested Generics без реальной потребности
- ✅ **Separation of Concerns:** HoverFormatter отделён от бизнес-логики
- ✅ **Extensibility:** Trait-based дизайн позволяет добавлять новые форматы

### Риски и митигации

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| Ломание существующих тестов при рефакторинге | MEDIUM | Запускать тесты после каждого коммита |
| Generic inference выводит неправильный тип | MEDIUM | Использовать flow-sensitive анализ + тесты |
| Nested Generics окажутся нужны раньше | LOW | Ограничение глубины + честный certainty |

---

**Итого:** План готов к утверждению. Ожидаю обратную связь от пользователя для начала реализации.
