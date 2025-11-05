# Шаг 2: Расширение SignatureIndex для конструкторов

**Статус**: ✅ Завершён
**Milestone**: 2.22 Constructor Support
**Дата**: 2025-11-05

## Обзор

Расширение `SignatureIndex` для хранения и управления сигнатурами конструкторов платформенных типов.

## Реализованные компоненты

### 1. Структура ConstructorSignature

```rust
pub struct ConstructorSignature {
    /// Имя типа ("Массив", "ТаблицаЗначений")
    pub type_name: String,

    /// Параметры конструктора
    pub params: Vec<ParameterInfo>,

    /// Результирующий facet (Object, Reference, Manager)
    /// None означает что конструктор возвращает сам тип
    pub facet: Option<String>,

    /// Источник сигнатуры
    pub source: SignatureSource,

    /// Является ли тип коллекцией (для generic inference)
    pub is_collection: bool,

    /// Количество generic параметров для коллекций
    pub generic_params_count: usize,
}
```

### 2. Расширение SignatureIndex

Добавлено новое поле:
```rust
pub struct SignatureIndex {
    // ... существующие поля ...
    constructors: HashMap<String, ConstructorSignature>,
}
```

### 3. Публичные методы

#### add_constructor
```rust
pub fn add_constructor(&mut self, type_name: String, constructor: ConstructorSignature)
```
Добавить конструктор в индекс.

#### find_constructor
```rust
pub fn find_constructor(&self, type_name: &str) -> Option<&ConstructorSignature>
```
Найти конструктор по имени типа (регистронезависимо).

#### get_all_constructors
```rust
pub fn get_all_constructors(&self) -> &HashMap<String, ConstructorSignature>
```
Получить все зарегистрированные конструкторы.

#### is_collection_type
```rust
pub fn is_collection_type(&self, type_name: &str) -> bool
```
Проверить является ли тип коллекцией.

#### get_generic_params_count
```rust
pub fn get_generic_params_count(&self, type_name: &str) -> Option<usize>
```
Получить количество generic параметров для типа.

#### initialize_builtin_constructors
```rust
pub fn initialize_builtin_constructors(&mut self)
```
Инициализировать встроенные конструкторы коллекций платформы.

## Встроенные конструкторы

### Коллекции с generic параметрами

| Тип | Generic параметры | Параметры конструктора |
|-----|-------------------|------------------------|
| Массив | 1 (элемент) | Размер? (Число) |
| Соответствие | 2 (ключ, значение) | - |
| СписокЗначений | 1 (значение) | - |
| ФиксированныйМассив | 1 (элемент) | Массив (обязательный) |

### Специальные типы

| Тип | Generic параметры | Описание |
|-----|-------------------|----------|
| ТаблицаЗначений | 0 | Не является generic коллекцией |

## Примеры использования

### Добавление конструктора

```rust
let mut index = SignatureIndex::new();

let constructor = ConstructorSignature {
    type_name: "Массив".to_string(),
    params: vec![
        ParameterInfo {
            name: "Размер".to_string(),
            type_name: Some("Число".to_string()),
            is_optional: true,
            default_value: None,
            description: Some("Начальный размер массива".to_string()),
        }
    ],
    facet: None,
    source: SignatureSource::Platform,
    is_collection: true,
    generic_params_count: 1,
};

index.add_constructor("Массив".to_string(), constructor);
```

### Поиск конструктора

```rust
// Регистронезависимый поиск
let array_ctor = index.find_constructor("Массив");
let array_ctor2 = index.find_constructor("массив");
let array_ctor3 = index.find_constructor("МАССИВ");
// Все три вызова вернут один и тот же конструктор
```

### Проверка типа коллекции

```rust
index.initialize_builtin_constructors();

assert!(index.is_collection_type("Массив")); // true
assert!(index.is_collection_type("Соответствие")); // true
assert!(!index.is_collection_type("ТаблицаЗначений")); // false
```

### Получение количества generic параметров

```rust
assert_eq!(index.get_generic_params_count("Массив"), Some(1));
assert_eq!(index.get_generic_params_count("Соответствие"), Some(2));
assert_eq!(index.get_generic_params_count("ТаблицаЗначений"), Some(0));
```

## Тестирование

### Unit тесты

Добавлено 5 новых тестов:

1. **test_add_and_find_constructor** - базовое добавление и поиск
2. **test_find_constructor_case_insensitive** - регистронезависимый поиск
3. **test_is_collection_type** - проверка типа коллекции
4. **test_get_generic_params_count** - получение количества generic параметров
5. **test_builtin_constructors** - проверка всех встроенных конструкторов

### Результаты тестирования

```bash
cargo test --package bsl-shared signature_index

running 8 tests
test domain::signature_index::tests::test_add_and_find_constructor ... ok
test domain::signature_index::tests::test_builtin_constructors ... ok
test domain::signature_index::tests::test_find_constructor_case_insensitive ... ok
test domain::signature_index::tests::test_get_generic_params_count ... ok
test domain::signature_index::tests::test_is_collection_type ... ok
test domain::signature_index::tests::test_signature_index_basic ... ok
test domain::signature_index::tests::test_signature_index_case_insensitive ... ok
test domain::signature_index::tests::test_signature_index_not_found ... ok

test result: ok. 8 passed; 0 failed
```

## Файлы

- **Изменённые файлы**:
  - `shared/src/domain/signature_index.rs` (+178 строк)

## Интеграция

### Используется в

- `TypeChecker` - для проверки типов конструкторов
- `GenericInferenceEngine` - для вывода generic типов коллекций
- `AnalysisEngine` - для валидации вызовов конструкторов

### Связанные компоненты

- `NewExpression` (IR node) - представление конструктора в IR
- `ParameterInfo` - общая структура для параметров
- `SignatureSource` - источник сигнатуры (Platform/Configuration/UserCode)

## Следующие шаги

**Шаг 3: Semantic Pass для NewExpression**
- Конвертация AST NewExpression → IR NewExpression
- Валидация существования конструктора
- Проверка параметров конструктора

## Дизайн решения

### Почему ConstructorSignature отделена от MethodSignature?

1. **Разная семантика**: конструктор создаёт объект, метод вызывается на объекте
2. **Разные атрибуты**: конструктор имеет `is_collection` и `generic_params_count`
3. **Разные правила валидации**: конструктор может иметь facet
4. **Разные источники**: конструкторы только Platform/Configuration, методы могут быть UserCode

### Почему HashMap вместо Vec?

- Быстрый поиск по имени типа O(1)
- Уникальность конструктора по типу (один конструктор на тип)
- Не требуется порядок конструкторов

### Почему регистронезависимый поиск?

BSL язык регистронезависимый:
```bsl
Массив = Новый Массив;
массив = Новый массив;  // То же самое
МАССИВ = Новый МАССИВ;  // То же самое
```

## Ограничения

1. **Один конструктор на тип** - HashMap<String, ConstructorSignature>
   - 1С не поддерживает перегрузку конструкторов
   - Все параметры опциональные или обязательные, но нет перегрузки

2. **Только платформенные и конфигурационные конструкторы**
   - UserCode не может определять конструкторы
   - Конструкторы только для встроенных типов

3. **Generic inference отложен**
   - Сейчас только хранение `generic_params_count`
   - Реальный inference в Milestone 2.23

## Ссылки

- [Шаг 1: IR узел NewExpression](constructor-support-step1.md)
- [Вариант 3: Constructor Support Design](constructor-support-design-variant3.md)
- [ROADMAP_2025.md](../../ROADMAP_2025.md) - Milestone 2.22
