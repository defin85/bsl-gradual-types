# Архитектура единой системы типов BSL v2.0

## Содержание

1. [Обзор](#обзор)
2. [Ключевые концепции](#ключевые-концепции)
3. [Архитектура системы](#архитектура-системы)
4. [Структуры данных](#структуры-данных)
5. [Специальные возможности](#специальные-возможности)
6. [Контекстное разрешение типов](#контекстное-разрешение-типов)
7. [Автодополнение](#автодополнение)
8. [Примеры использования](#примеры-использования)

## Обзор

Система типов 1С:Предприятие имеет уникальную структуру, которая не укладывается в классические парадигмы ООП. Данный документ описывает архитектуру "Unified Type System with Facets" - подход к представлению типов 1С в анализаторе BSL.

### Ключевые особенности системы типов 1С

1. **Множественные представления** - один тип может иметь разные формы (менеджер, объект, ссылка, метаданные)
2. **Контекстная доступность** - типы доступны по-разному в зависимости от контекста
3. **Двойной источник типов**:
   - **Платформа (синтаксис-помощник)** - ~4000+ базовых типов, методы, интерфейсы
   - **Конфигурация (XML)** - конкретные справочники, документы, их реквизиты
4. **Двойственность runtime/metadata** - параллельные системы для данных и структуры

## Ключевые концепции

### Фасеты (Facets)

Вместо создания отдельных типов для каждого представления, используем единый тип с различными "фасетами" - гранями, которые активируются в зависимости от контекста использования.

```rust
// Один тип "Контрагенты" с разными фасетами
UnifiedBslType {
    core_name: "Контрагенты",
    facets: {
        manager,    // СправочникМенеджер.Контрагенты
        object,     // СправочникОбъект.Контрагенты
        reference,  // СправочникСсылка.Контрагенты
        metadata,   // ОбъектМетаданныхСправочник
    }
}
```

### Примеры множественных представлений

```bsl
// Runtime представления - работа с данными
Справочники.Контрагенты                    // manager facet
Справочники.Контрагенты.СоздатьЭлемент()   // object facet
Справочники.Контрагенты.НайтиПоКоду("123") // reference facet

// Metadata представления - работа со структурой
Метаданные.Справочники.Контрагенты         // metadata facet
```

## Архитектура системы

### Источники данных о типах

1. **Синтаксис-помощник** (rebuilt.shcntx_ru.zip):
   - ~4000+ платформенных типов
   - Все методы и их сигнатуры
   - Базовые свойства (Код, Наименование для всех справочников)
   - Интерфейсы (СправочникОбъект, СправочникСсылка)

2. **Configuration.xml**:
   - Конкретные справочники, документы, регистры
   - Специфичные реквизиты (ИНН, КПП для Контрагентов)
   - Табличные части
   - Структура метаданных

### Трёхуровневая структура

```rust
pub struct UnifiedBslIndex {
    // Уровень 1: Шаблоны фасетов (из синтаксис-помощника)
    facet_templates: FacetTemplateRegistry,
    
    // Уровень 2: Конфигурационные типы (композиция платформы + XML)
    configuration_types: HashMap<String, UnifiedBslType>,
    
    // Уровень 3: Глобальный контекст
    global_context: GlobalContext,
}
```

#### Уровень 1: Шаблоны фасетов

Хранятся ОДИН раз для всех типов одной категории. Содержат методы и свойства из платформы (синтаксис-помощника).

```rust
pub struct FacetTemplateRegistry {
    catalog_facets: CatalogFacets,
    document_facets: DocumentFacets,
    register_facets: RegisterFacets,
    collection_facets: CollectionFacets,
}
```

#### Уровень 2: Конфигурационные типы

Каждый тип из конфигурации хранит ссылки на шаблоны и свои расширения.

```rust
pub struct UnifiedBslType {
    core_name: String,
    metadata_kind: MetadataKind,
    facets: TypeFacets,
    access_rules: AccessRules,
}
```

#### Уровень 3: Глобальный контекст

Глобальные функции, свойства и константы - это НЕ типы, а элементы глобального контекста.

```rust
pub struct GlobalContext {
    properties: HashMap<String, GlobalProperty>,  // Справочники, Метаданные
    functions: HashMap<String, GlobalFunction>,   // СтрНайти, Сообщить
    constants: HashMap<String, GlobalConstant>,   // Неопределено, Истина
}
```

## Структуры данных

### Основная структура типа

```rust
pub struct UnifiedBslType {
    // Идентификация
    core_name: String,              // "Контрагенты"
    metadata_kind: MetadataKind,    // Catalog, Document, Register...
    
    // Фасеты - различные представления типа
    facets: TypeFacets {
        manager: Option<ManagerFacet>,
        object: Option<ObjectFacet>,
        reference: Option<ReferenceFacet>,
        metadata: Option<MetadataFacet>,
        constructor: Option<ConstructorFacet>,
        collection: Option<CollectionFacet>,
        readonly_element: Option<ReadOnlyElementFacet>,
        singleton: Option<SingletonFacet>,
    },
    
    // Правила доступа
    access_rules: AccessRules,
}
```

### Фасеты

#### ManagerFacet - менеджер объектов конфигурации

```rust
pub struct ManagerFacet {
    template_ref: FacetRef,         // Ссылка на шаблон
    type_name: String,               // "СправочникМенеджер.Контрагенты"
    access_path: AccessPath,         // Справочники.Контрагенты
    extensions: FacetExtensions,     // Дополнительные методы из модуля
}
```

#### ObjectFacet - изменяемый объект

```rust
pub struct ObjectFacet {
    template_ref: FacetRef,
    type_name: String,               // "СправочникОбъект.Контрагенты"
    access_path: AccessPath,         // Через СоздатьЭлемент()
    extensions: FacetExtensions {
        properties: Vec<String>,     // ["ИНН", "КПП"] - из конфигурации
        custom_methods: Vec<String>, // Из модуля объекта
    },
}
```

#### ConstructorFacet - конструируемые типы

```rust
pub struct ConstructorFacet {
    type_name: String,
    constructors: Vec<ConstructorSignature>, // Множественные конструкторы
    methods: Vec<Method>,
    properties: Vec<Property>,
}

pub struct ConstructorSignature {
    description: Option<String>,
    parameters: Vec<ConstructorParameter>,
    examples: Vec<String>,
}
```

#### CollectionFacet - коллекции с типизированными элементами

```rust
pub struct CollectionFacet {
    type_name: String,
    element_access: ElementAccessRules,
    constructors: Vec<ConstructorSignature>,
    methods: Vec<Method>,
}

pub struct ElementAccessRules {
    iteration_type: Option<String>,      // Тип при Для Каждого
    index_access_type: Option<String>,   // Тип при индексном доступе
    method_return_types: HashMap<String, String>,
    element_mutability: ElementMutability,
}
```

### Правила доступа

```rust
pub struct AccessRules {
    can_construct: bool,             // Можно создать через Новый
    is_global_property: bool,        // Доступен как глобальное свойство
    is_singleton: bool,              // Единственный экземпляр
    is_iteration_element: bool,      // Получается при итерации
    available_contexts: Vec<BslContext>,
}
```

### Глобальный контекст

```rust
pub struct GlobalProperty {
    name: String,                    // "Справочники"
    returns_type: String,            // "СправочникиМенеджер"
    provides_access_to: Vec<String>, // ["СправочникМенеджер.*"]
    indexing: Option<IndexingSupport>, // Поддержка Справочники[имя]
}

pub struct GlobalFunction {
    name: String,                    // "СтрНайти"
    english_name: Option<String>,    // "StrFind"
    parameters: Vec<Parameter>,
    return_type: Option<String>,
    // НЕ имеет фасетов, вызывается напрямую
}
```

## Специальные возможности

### Множественные конструкторы

Многие типы в 1С имеют несколько вариантов конструкторов:

```bsl
// Разные способы создания
Массив1 = Новый Массив;
Массив2 = Новый Массив(10);
Массив3 = Новый Массив(10, 5);
```

Поддерживается через `constructors: Vec<ConstructorSignature>` в ConstructorFacet.

### Коллекции с типизированными элементами

```bsl
// Соответствие → КлючИЗначение
Для Каждого Элемент Из Соответствие Цикл
    Ключ = Элемент.Ключ;      // readonly
    Значение = Элемент.Значение; // readonly
КонецЦикла;

// НО: индексный доступ возвращает другой тип!
Значение = Соответствие["ключ"]; // Произвольный, НЕ КлючИЗначение
```

### Переходы между фасетами

```bsl
// Получение метаданных из runtime объекта
Контрагент = Справочники.Контрагенты.НайтиПоКоду("123");
МетаданныеКонтрагента = Контрагент.Метаданные(); // Переход к metadata facet

// Динамический доступ по имени
ИмяСправочника = "Контрагенты";
Менеджер = Справочники[ИмяСправочника]; // Динамическое разрешение
```

#### Cross-facet методы

```rust
pub struct CrossFacetMethod {
    method_name: String,             // "Метаданные"
    returns: FacetTransition {
        target_facet: FacetKind,     // Metadata
        of_same_type: bool,          // true - тот же core_type
    },
}
```

#### Динамический доступ

```rust
pub struct IndexingSupport {
    index_type: IndexType,           // ByName, ByIndex
    resolution: IndexResolution::DynamicLookup {
        returns_facet: FacetKind,     // Manager для Справочники[имя]
    },
}
```

## Контекстное разрешение типов

### Определение активного фасета

```rust
impl ContextResolver {
    pub fn resolve_facet(&self, type_name: &str, context: &Context) -> Option<ActiveFacet> {
        let unified_type = self.index.find_type(type_name)?;
        
        match context {
            Context::AfterDot("Справочники") => unified_type.facets.manager,
            Context::AfterDot("Метаданные.Справочники") => unified_type.facets.metadata,
            Context::MethodResult("СоздатьЭлемент") => unified_type.facets.object,
            Context::MethodResult("НайтиПоКоду") => unified_type.facets.reference,
            _ => unified_type.facets.reference // по умолчанию
        }
    }
}
```

### Определение типа в итерации

```rust
impl TypeResolver {
    pub fn resolve_iteration_variable(
        &self,
        collection_type: &str,
        context: &IterationContext
    ) -> Option<TypeInfo> {
        let unified_type = self.index.find_type(collection_type)?;
        
        if let Some(collection_facet) = self.get_collection_facet(unified_type) {
            if let Some(iteration_type) = &collection_facet.element_access.iteration_type {
                return Some(TypeInfo {
                    type_name: iteration_type.clone(),
                    is_readonly: /* определяется по element_mutability */,
                    context: TypeContext::IterationElement,
                });
            }
        }
        
        Some(TypeInfo::arbitrary())
    }
}
```

## Автодополнение

### Правила показа элементов

```rust
pub fn get_completions(context: &Context) -> Vec<CompletionItem> {
    match context {
        // Пустой контекст - глобальные элементы
        Context::EmptyLine => {
            vec![]
                .extend(global_context.functions)  // СтрНайти, Сообщить
                .extend(global_context.properties) // Справочники, Метаданные
                .extend(global_context.constants)  // Неопределено, Истина
        },
        
        // После "Новый" - только конструируемые типы
        Context::AfterNew => {
            configuration_types
                .filter(|t| t.access_rules.can_construct)
                .filter(|t| !t.is_manager())  // НЕ показываем менеджеры!
        },
        
        // После точки - члены активного фасета
        Context::AfterDot(parent) => {
            let facet = resolve_facet_for_context(parent_type, context)?;
            get_members_for_facet(facet)
        },
        
        // В цикле - тип элемента коллекции
        Context::ForEachLoop(collection) => {
            resolve_iteration_variable(collection)
        }
    }
}
```

### Проверка типов для конструкторов

```rust
impl TypeChecker {
    pub fn check_constructor_call(
        &self,
        type_name: &str,
        arguments: &[Expression]
    ) -> Result<TypeInfo> {
        let unified_type = self.index.find_type(type_name)?;
        
        if let Some(constructor_facet) = &unified_type.facets.constructor {
            // Ищем подходящую сигнатуру среди множественных конструкторов
            for signature in &constructor_facet.constructors {
                if self.arguments_match_signature(arguments, signature) {
                    return Ok(TypeInfo { /* ... */ });
                }
            }
        }
        
        Err(/* нет подходящего конструктора */)
    }
}
```

## Примеры использования

### Пример 1: Создание объекта справочника

```bsl
НовыйКонтрагент = Справочники.Контрагенты.СоздатьЭлемент();
НовыйКонтрагент.ИНН = "1234567890";
НовыйКонтрагент.Записать();
```

**Разрешение типов:**
1. `Справочники` - GlobalProperty → provides access to managers
2. `Контрагенты` - активируется manager facet
3. `СоздатьЭлемент()` - возвращает object facet
4. `ИНН` - свойство из extensions (конфигурация)
5. `Записать()` - метод из шаблона ObjectFacet

### Пример 2: Итерация по коллекции

```bsl
Соотв = Новый Соответствие;
Соотв.Вставить("Имя", "Иван");

Для Каждого Элемент Из Соотв Цикл
    Сообщить(Элемент.Ключ);  // КлючИЗначение - readonly
КонецЦикла;

Значение = Соотв["Имя"];  // Строка - НЕ КлючИЗначение!
```

**Разрешение типов:**
1. `Новый Соответствие` - constructor facet
2. В цикле `Элемент` - тип из `iteration_type` = КлючИЗначение
3. Индексный доступ - тип из `index_access_type` = Произвольный

### Пример 3: Переход между runtime и metadata

```bsl
// Runtime → Metadata
Контрагент = Справочники.Контрагенты.НайтиПоКоду("123");
Мета = Контрагент.Метаданные();

// Metadata → Runtime (динамически)
ИмяСправочника = Мета.Имя;
Менеджер = Справочники[ИмяСправочника];
```

**Разрешение типов:**
1. `Контрагент` - reference facet
2. `Метаданные()` - cross-facet метод → metadata facet
3. `Мета.Имя` - свойство metadata facet
4. `Справочники[...]` - динамический доступ → manager facet

## Категории типов в системе

### 1. Конфигурационные типы (с фасетами)
- Справочники, Документы, Регистры
- Имеют manager, object, reference, metadata фасеты
- **Структура** из Configuration.xml (реквизиты, табличные части)
- **Методы** из платформы (синтаксис-помощник)

### 2. Платформенные коллекции
- Массив, Структура, Соответствие, ТаблицаЗначений
- Полностью из синтаксис-помощника (методы и свойства)
- Имеют constructor facet
- Могут иметь типизированные элементы

### 3. Элементы коллекций
- КлючИЗначение, СтрокаТаблицыЗначений
- НЕ конструируемые (нет constructor facet)
- Часто readonly

### 4. Singleton типы
- Метаданные, XMLСхема
- Единственный экземпляр
- Доступны как глобальные свойства

### 5. Глобальные элементы (НЕ типы)
- Функции: СтрНайти, Сообщить
- Свойства: Справочники, Документы
- Константы: Неопределено, Истина

## Преимущества архитектуры

1. **Единая точка правды** - один тип вместо множества отдельных
2. **Эффективность хранения** - нет дублирования методов
3. **Контекстная корректность** - правильное автодополнение
4. **Поддержка всех особенностей 1С**:
   - Множественные представления
   - Переходы между runtime/metadata
   - Динамический доступ
   - Множественные конструкторы
   - Типизированные элементы коллекций
5. **Расширяемость** - легко добавить новые фасеты или типы

## Заключение

Архитектура "Unified Type System with Facets v2.0" обеспечивает:

- Корректное представление уникальной системы типов 1С
- Эффективное хранение без дублирования
- Правильное контекстное автодополнение
- Поддержку всех особенностей платформы
- Простоту расширения и поддержки

Данный подход решает проблему множественных представлений одной сущности и полностью поддерживает семантику платформы 1С:Предприятие.