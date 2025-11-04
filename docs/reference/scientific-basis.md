# Scientific Basis

Научные основы BSL Gradual Type System проекта.

## 📚 Основополагающая статья

**Balyuk, A. S., & Popova, V. A. (2021).**
*Static type-checking for programs developed on the platform 1C:Enterprise.*
CEUR Workshop Proceedings, Vol-2984.

**Ссылка:** [https://ceur-ws.org/Vol-2984/paper13.pdf](https://ceur-ws.org/Vol-2984/paper13.pdf)

---

## 🎯 Ключевые концепции из статьи

### 1. Фасетная система типов

**Концепция:** Множественное наследование функциональности объектов 1С

Один объект 1С имеет **разные представления (фасеты)** в зависимости от контекста использования:

| Фасет | Английское название | Назначение | Пример |
|-------|---------------------|------------|--------|
| **Manager** | ManagerRef | Создание, поиск элементов | `Справочники.Контрагенты.НайтиПоНаименованию("Рога и копыта")` |
| **Object** | ObjectRef | Изменяемый объект | `Объект = Справочники.Контрагенты.СоздатьЭлемент()` |
| **Reference** | Reference | Ссылка на элемент | `СсылкаНаКонтрагента = Справочники.Контрагенты.НайтиПоКоду("000001")` |
| **Selection** | Selection | Обход элементов справочника | `Выборка = Справочники.Контрагенты.Выбрать()` |
| **List** | List | Управление списком в форме | `Список = Справочники.Контрагенты.СоздатьСписокЗначений()` |

**Реализация в проекте:**

[shared/src/domain/types.rs](../../shared/src/domain/types.rs):

```rust
pub enum FacetKind {
    Manager,      // Справочники.Контрагенты
    Object,       // СправочникОбъект.Контрагенты
    Reference,    // СправочникСсылка.Контрагенты
    Selection,    // СправочникВыборка.Контрагенты (добавлено из статьи)
    List,         // СправочникСписок.Контрагенты (добавлено из статьи)
    Metadata,     // Метаданные.Справочники.Контрагенты
}
```

**Автоматическое переключение контекста:**

TypeResolver автоматически определяет активный фасет по контексту использования:

```bsl
// Manager фасет
Элемент = Справочники.Контрагенты.НайтиПоКоду("000001");
// TypeResolver: активный фасет = Manager
// Доступные методы: НайтиПоКоду, НайтиПоНаименованию, СоздатьЭлемент

// Reference фасет
Наименование = Элемент.Наименование;
// TypeResolver: активный фасет = Reference
// Доступные свойства: Наименование, Код, ИНН, Адрес

// Object фасет
Элемент.Наименование = "Новое наименование";
Элемент.Записать();
// TypeResolver: активный фасет = Object
// Доступные методы: Записать, Удалить, УстановитьСсылкуНового
```

---

### 2. Configuration Types Tree (CTT)

**Концепция:** Упрощённый формат для описания типов конфигурации

Вместо полного XML формата метаданных 1С, используется **упрощённое дерево**:

```
Configuration:
  Справочники:
    - Контрагенты:
        Attributes:
          - Наименование: Строка(150)
          - ИНН: Строка(12)
          - Адрес: Строка(500)
        TabularSections:
          - КонтактнаяИнформация:
              Columns:
                - Тип: Строка(50)
                - Значение: Строка(200)
```

**Реализация в проекте:**

[backend/src/data/loaders/config_metadata_parser/](../../backend/src/data/loaders/config_metadata_parser/):

```rust
pub struct ConfigurationType {
    pub name: String,
    pub category: ConfigurationCategory,  // Справочник | Документ | Регистр
    pub attributes: Vec<Attribute>,
    pub tabular_sections: Vec<TabularSection>,
}

pub enum ConfigurationCategory {
    Catalog,           // Справочник
    Document,          // Документ
    InformationRegister,  // Регистр сведений
    AccumulationRegister, // Регистр накопления
    // ... другие категории
}
```

**Парсинг метаданных:**

```rust
let discovery = ConfigurationDiscovery::new(config_path);
let configurations = discovery.discover_all_configurations()?;
let metadata = discovery.discover_metadata_in_configuration(&configurations[0], None)?;
```

---

### 3. Три категории типовых ошибок

**Из статьи Balyuk & Popova (2021), глава 4 "Type Errors":**

#### Категория 1: Некорректная передача параметров методам

**Пример:**
```bsl
// ❌ Ошибка: Массив.Добавить() ожидает 1 параметр
МассивДанных.Добавить();

// ✅ Правильно
МассивДанных.Добавить(42);
```

**Валидация в проекте:**

[shared/src/domain/validators.rs](../../shared/src/domain/validators.rs):

```rust
pub fn validate_method_call(
    method: &Method,
    args: &[Argument]
) -> Result<(), ValidationError> {
    // Проверка количества параметров
    if args.len() < method.required_params {
        return Err(ValidationError::MissingRequiredParameter {
            method: method.name.clone(),
            expected: method.required_params,
            got: args.len(),
        });
    }

    // Проверка типов параметров
    for (arg, param) in args.iter().zip(method.params.iter()) {
        validate_argument_type(arg, param)?;
    }

    Ok(())
}
```

#### Категория 2: Обращение к несуществующим свойствам объектов

**Пример:**
```bsl
// ❌ Ошибка: Массив не имеет свойства "Размер"
Размер = МассивДанных.Размер;

// ✅ Правильно
Размер = МассивДанных.Количество();
```

**Валидация в проекте:**

```rust
pub fn validate_property_access(
    type_metadata: &TypeMetadata,
    property_name: &str
) -> Result<(), ValidationError> {
    if !type_metadata.has_property(property_name) {
        return Err(ValidationError::PropertyNotFound {
            type_name: type_metadata.name.clone(),
            property: property_name.to_string(),
            suggestion: find_closest_property(type_metadata, property_name),
        });
    }

    Ok(())
}
```

#### Категория 3: Обработка простых типов как коллекций

**Пример:**
```bsl
// ❌ Ошибка: Строка не является коллекцией
Строка = "Привет, мир!";
Для Каждого Символ Из Строка Цикл
    Сообщить(Символ);
КонецЦикла;

// ✅ Правильно: Массив является коллекцией
Массив = Новый Массив();
Массив.Добавить(1);
Массив.Добавить(2);
Для Каждого Элемент Из Массив Цикл
    Сообщить(Элемент);
КонецЦикла;
```

**Валидация в проекте:**

```rust
pub fn validate_iterable(
    type_hint: &TypeHint
) -> Result<(), ValidationError> {
    let type_metadata = repository.find_type(&type_hint.name)?;

    if !type_metadata.implements_trait(Trait::Iterable) {
        return Err(ValidationError::NotIterable {
            type_name: type_metadata.name.clone(),
        });
    }

    Ok(())
}
```

---

## 🔍 Применение в проекте

### Фасетная система → TypeResolver

**Статья:** "Faceted type system for 1C:Enterprise objects"

**Реализация:**

[shared/src/domain/resolver.rs](../../shared/src/domain/resolver.rs):

```rust
pub struct TypeResolver {
    repository: Arc<TypeRepository>,
    facet_rules: FacetRules,  // Правила определения активного фасета
}

impl TypeResolver {
    pub fn resolve_member_access(&self, prefix: &str, member: &str)
        -> TypeResolution
    {
        // Определяем активный фасет по контексту
        let facet = self.facet_rules.infer_facet(prefix, member);

        // Резолвим тип с учётом фасета
        let type_name = format!("{}.{}", prefix, member);
        let type_metadata = self.repository.find_type_with_facet(&type_name, facet)?;

        TypeResolution {
            result: ResolutionResult::Concrete(type_metadata),
            certainty: Certainty::Known,
            active_facet: facet,
        }
    }
}
```

### Configuration Types Tree → ConfigurationDiscovery

**Статья:** "Configuration Type Tree (CTT) format"

**Реализация:**

[backend/src/data/loaders/config_metadata_parser/discovery.rs](../../backend/src/data/loaders/config_metadata_parser/discovery.rs):

```rust
pub struct ConfigurationDiscovery {
    config_path: PathBuf,
}

impl ConfigurationDiscovery {
    pub fn discover_all_configurations(&self) -> Result<Vec<Configuration>> {
        // Сканирование папок конфигураций
        // Парсинг метаданных в CTT формат
        // Построение дерева типов
    }

    pub fn discover_metadata_in_configuration(
        &self,
        config: &Configuration,
        filter: Option<ConfigurationCategory>
    ) -> Result<Vec<ConfigurationType>> {
        // Извлечение типов (Справочники, Документы, Регистры)
        // Парсинг атрибутов и табличных частей
        // Конвертация в TypeMetadata
    }
}
```

### Три категории ошибок → Validators

**Статья:** "Chapter 4: Type Errors"

**Реализация:**

[shared/src/domain/validators.rs](../../shared/src/domain/validators.rs):

```rust
pub mod validators {
    // Категория 1: Некорректная передача параметров
    pub fn validate_method_call(...) -> Result<(), ValidationError>;

    // Категория 2: Обращение к несуществующим свойствам
    pub fn validate_property_access(...) -> Result<(), ValidationError>;

    // Категория 3: Обработка простых типов как коллекций
    pub fn validate_iterable(...) -> Result<(), ValidationError>;
}
```

---

## 📊 Дополнения к статье

Проект **расширяет** концепции из статьи:

### 1. Градуальная типизация (Gradual Typing)

**Не в статье**, но критично для MVP:

```rust
pub enum Certainty {
    Known,              // 100% уверенности (типы платформы)
    Inferred(f32),      // 0.0-1.0 уверенности (выведенные типы)
    Unknown,            // 0% уверенности (невозможно определить)
}
```

**Честная оценка certainty** (исправление 2025-01-18):

```rust
// Если метаданные найдены → Known (100%)
// Если только синтаксис → Inferred (50%)
// Если тип неизвестен → Unknown (0%)
let (certainty, source) = if has_metadata {
    (Certainty::Known, ResolutionSource::Static)
} else {
    (Certainty::Inferred(0.5), ResolutionSource::Inferred)
};
```

### 2. Semantic IR Layer (Milestone 2.8)

**Не в статье**, но упрощает реализацию:

- SemanticProgram — промежуточное представление
- Parser trait — dependency inversion для парсеров
- AstToIrConverter — мост между синтаксисом и семантикой

### 3. Inline Scope Analysis (Milestone 2.9)

**Не в статье**, но практично для LSP:

- Анализ типов "на лету" при hover
- Без управления жизненным циклом runtime типов
- Работает в пределах одной процедуры/функции

---

## 🔗 Связанные документы

- **[Type System Architecture](../architecture/type_system_architecture.md)** — реализация концепций
- **[Components Detailed](../architecture/components-detailed.md)** — детали компонентов
- **[Milestones History](../architecture/milestones-history.md)** — история развития

---

## 📖 Цитирование

Если используешь концепции из этого проекта, пожалуйста, цитируй:

```bibtex
@inproceedings{balyuk2021static,
  title={Static type-checking for programs developed on the platform 1C:Enterprise},
  author={Balyuk, Andrey S and Popova, Valentina A},
  booktitle={CEUR Workshop Proceedings},
  volume={2984},
  year={2021},
  url={https://ceur-ws.org/Vol-2984/paper13.pdf}
}
```

---

## 🎯 Дальнейшие исследования

Перспективные направления:

1. **Flow-sensitive анализ** (Milestone 2.19)
   - Control Flow Graph (CFG)
   - Type narrowing после проверок
   - Nullability analysis

2. **Inter-procedural анализ**
   - Анализ вызовов между модулями
   - Call graph построение
   - Whole-program analysis

3. **Runtime контракты**
   - Design by Contract для 1С
   - Preconditions и postconditions
   - Invariants для объектов

4. **Формальная верификация**
   - Доказательство корректности типизации
   - Soundness и completeness
   - Theorem proving для критичных модулей
