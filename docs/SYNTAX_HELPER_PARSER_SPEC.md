# Техническое задание на парсер синтакс-помощника 1С v3

## 1. Общие требования

### 1.1 Принципы
- **Discovery-based подход**: парсер не должен иметь предположений о структуре, всё извлекается из файлов
- **Полнота информации**: сохраняем всю документацию для показа в IDE
- **Двуязычность**: поддержка русских и английских названий с возможностью поиска по любому
- **Иерархичность**: правильное отражение структуры категория → тип → члены
- **Интеграция с TypeResolution**: все типы должны быть обёрнуты в TypeResolution для градуальной типизации

### 1.2 Входные данные
- Распакованный архив синтакс-помощника (rebuilt.shcntx_ru/)
- Структура: objects/catalogXXX/... с вложенными каталогами и HTML файлами

### 1.3 Интеграция с архитектурой проекта
Парсер должен интегрироваться с существующей системой типов:
- `TypeResolution` - центральная абстракция для градуальной типизации
- `PlatformType` - представление платформенных типов
- `FacetKind` - система фасетов для разных представлений типов
- `PlatformTypesResolverV2` - основной resolver для платформенных типов

## 2. Структуры данных

```rust
/// Узел в иерархии синтакс-помощника
#[derive(Debug, Clone)]
pub enum SyntaxNode {
    /// Категория типов (например "Таблица значений")
    Category(CategoryInfo),
    
    /// Конкретный тип данных (например "ТаблицаЗначений")
    Type(TypeInfo),
    
    /// Метод типа
    Method(MethodInfo),
    
    /// Свойство типа  
    Property(PropertyInfo),
    
    /// Конструктор типа
    Constructor(ConstructorInfo),
}

/// Информация о категории типов
#[derive(Debug, Clone)]
pub struct CategoryInfo {
    /// Название категории (из <h1>)
    pub name: String,                    // "Таблица значений"
    
    /// Путь в иерархии каталогов
    pub catalog_path: String,            // "catalog234/catalog236"
    
    /// Полное описание категории
    pub description: String,             
    
    /// Ссылки на связанные концепции
    pub related_links: Vec<String>,
    
    /// Список типов в этой категории
    pub types: Vec<String>,              // ["ValueTable", "ValueTableRow", ...]
}

/// Полная информация о типе
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Идентификация типа
    pub identity: TypeIdentity,
    
    /// Документация
    pub documentation: TypeDocumentation,
    
    /// Структура типа
    pub structure: TypeStructure,
    
    /// Метаданные
    pub metadata: TypeMetadata,
}

/// Идентификация типа - как его можно найти
#[derive(Debug, Clone)]
pub struct TypeIdentity {
    /// Русское название из HTML
    pub russian_name: String,            // "ТаблицаЗначений"
    
    /// Английское название из HTML
    pub english_name: String,            // "ValueTable"
    
    /// Путь в каталоге
    pub catalog_path: String,            // "catalog234/catalog236/ValueTable"
    
    /// Альтернативные имена (из текста)
    pub aliases: Vec<String>,
    
    /// Ссылка на категорию
    pub category_path: String,           // "catalog234/catalog236"
}

/// Документация типа
#[derive(Debug, Clone)]
pub struct TypeDocumentation {
    /// Описание из категории (общее)
    pub category_description: Option<String>,
    
    /// Описание типа (специфичное)
    pub type_description: String,
    
    /// Примеры использования
    pub examples: Vec<CodeExample>,
    
    /// Доступность (Сервер, Клиент и т.д.)
    pub availability: Vec<String>,
    
    /// Версия платформы
    pub since_version: String,
}

/// Структура типа
#[derive(Debug, Clone)]
pub struct TypeStructure {
    /// Элемент коллекции (если есть)
    pub collection_element: Option<String>,  // "СтрокаТаблицыЗначений"
    
    /// Методы
    pub methods: Vec<String>,                // Имена методов
    
    /// Свойства
    pub properties: Vec<String>,             // Имена свойств
    
    /// Конструкторы
    pub constructors: Vec<String>,           // Типы конструкторов
    
    /// Поддержка итератора
    pub iterable: bool,
    
    /// Поддержка индексатора
    pub indexable: bool,
}

/// Метаданные типа
#[derive(Debug, Clone)]
pub struct TypeMetadata {
    /// Определённые фасеты для типа
    pub available_facets: Vec<FacetKind>,
    
    /// Активный фасет по умолчанию
    pub default_facet: Option<FacetKind>,
    
    /// Сериализуемость
    pub serializable: bool,
    
    /// Обмен с сервером
    pub exchangeable: bool,
    
    /// XDTO информация
    pub xdto_namespace: Option<String>,
    pub xdto_type: Option<String>,
}

/// Пример кода
#[derive(Debug, Clone)]
pub struct CodeExample {
    /// Описание примера
    pub description: Option<String>,
    
    /// Код примера
    pub code: String,
    
    /// Язык (BSL, Query и т.д.)
    pub language: String,
}
```

## 3. Система индексов

```rust
/// База данных синтакс-помощника (промежуточное хранилище)
pub struct SyntaxHelperDatabase {
    /// Все узлы по пути
    nodes: HashMap<String, SyntaxNode>,
    
    /// Детали методов
    methods: HashMap<String, MethodInfo>,
    
    /// Детали свойств  
    properties: HashMap<String, PropertyInfo>,
}

/// Индексы для поиска типов
pub struct TypeIndex {
    /// По русскому имени → путь к типу
    by_russian: HashMap<String, String>,
    
    /// По английскому имени → путь к типу
    by_english: HashMap<String, String>,
    
    /// По любому имени → список возможных путей
    by_any_name: HashMap<String, Vec<String>>,
    
    /// По категории → список типов
    by_category: HashMap<String, Vec<String>>,
    
    /// По фасету → список типов
    by_facet: HashMap<Facet, Vec<String>>,
}
```

## 4. Алгоритм парсинга

### 4.1 Фаза 1: Discovery
```rust
fn discover_structure(base_path: &Path) -> Vec<SyntaxNode> {
    let mut nodes = Vec::new();
    
    // Начинаем с objects/
    let objects_path = base_path.join("objects");
    discover_catalog(&objects_path, "", &mut nodes);
    
    nodes
}

fn discover_catalog(path: &Path, parent: &str, nodes: &mut Vec<SyntaxNode>) {
    // 1. Проверяем файл категории (catalogXXX.html в родительской папке)
    let catalog_name = path.file_name();
    let category_file = path.parent().join(format!("{}.html", catalog_name));
    
    if category_file.exists() {
        let category = parse_category_file(&category_file);
        nodes.push(SyntaxNode::Category(category));
    }
    
    // 2. Обходим все файлы в каталоге
    for entry in fs::read_dir(path) {
        let path = entry.path();
        
        if path.is_dir() {
            // Проверяем тип директории
            match path.file_name().to_str() {
                Some("methods") => discover_methods(&path, parent, nodes),
                Some("properties") => discover_properties(&path, parent, nodes),
                Some("ctors") => discover_constructors(&path, parent, nodes),
                Some(name) if name.starts_with("catalog") => {
                    // Вложенная категория
                    discover_catalog(&path, &format!("{}/{}", parent, name), nodes);
                }
                _ => {
                    // Вложенная папка типа
                    discover_catalog(&path, parent, nodes);
                }
            }
        } else if path.extension() == Some("html") {
            // HTML файл - определяем что это
            let node = analyze_html_file(&path, parent);
            nodes.push(node);
        }
    }
}
```

### 4.2 Фаза 2: Извлечение информации
```rust
fn parse_type_file(path: &Path, category_path: &str) -> TypeInfo {
    let content = fs::read_to_string(path)?;
    let document = Html::parse_document(&content);
    
    // Извлекаем название из <h1 class="V8SH_pagetitle">
    let full_title = extract_element_text(&document, "h1.V8SH_pagetitle");
    let (russian, english) = parse_title(&full_title); // "ТаблицаЗначений (ValueTable)"
    
    TypeInfo {
        identity: TypeIdentity {
            russian_name: russian,
            english_name: english,
            catalog_path: build_path(path),
            category_path: category_path.to_string(),
            aliases: extract_aliases(&document),
        },
        documentation: TypeDocumentation {
            category_description: get_category_description(category_path),
            type_description: extract_description(&document),
            examples: extract_examples(&document),
            availability: extract_availability(&document),
            since_version: extract_version(&document),
        },
        structure: TypeStructure {
            collection_element: extract_collection_element(&document),
            methods: extract_method_links(&document),
            properties: extract_property_links(&document),
            constructors: extract_constructor_links(&document),
            iterable: check_iterable(&document),
            indexable: check_indexable(&document),
        },
        metadata: TypeMetadata {
            facet: detect_facet(&document, &russian),
            serializable: check_serializable(&document),
            exchangeable: check_exchangeable(&document),
            xdto_namespace: extract_xdto_namespace(&document),
            xdto_type: extract_xdto_type(&document),
        }
    }
}

fn parse_title(title: &str) -> (String, String) {
    // Парсим строку вида "ТаблицаЗначений (ValueTable)"
    if let Some(open) = title.find('(') {
        if let Some(close) = title.find(')') {
            let russian = title[..open].trim().to_string();
            let english = title[open+1..close].trim().to_string();
            return (russian, english);
        }
    }
    // Если нет скобок, считаем что это русское название
    (title.trim().to_string(), String::new())
}
```

### 4.3 Фаза 3: Построение индексов
```rust
fn build_indexes(nodes: Vec<SyntaxNode>) -> TypeIndex {
    let mut index = TypeIndex::default();
    
    for node in nodes {
        match node {
            SyntaxNode::Type(type_info) => {
                let path = &type_info.identity.catalog_path;
                
                // Индекс по русскому имени
                index.by_russian.insert(
                    type_info.identity.russian_name.clone(),
                    path.clone()
                );
                
                // Индекс по английскому имени
                index.by_english.insert(
                    type_info.identity.english_name.clone(),
                    path.clone()
                );
                
                // Индекс по всем именам
                for alias in &type_info.identity.aliases {
                    index.by_any_name
                        .entry(alias.clone())
                        .or_default()
                        .push(path.clone());
                }
                
                // Индекс по категории
                index.by_category
                    .entry(type_info.identity.category_path.clone())
                    .or_default()
                    .push(path.clone());
                
                // Индекс по фасету
                if let Some(facet) = type_info.metadata.facet {
                    index.by_facet
                        .entry(facet)
                        .or_default()
                        .push(path.clone());
                }
            }
            _ => {}
        }
    }
    
    index
}
```

## 5. Интеграция с TypeResolution

### 5.1 Создание TypeResolution напрямую из TypeInfo

```rust
impl SyntaxHelperParserV3 {
    /// Создаёт TypeResolution напрямую из TypeInfo (без промежуточной конвертации)
    pub fn to_type_resolution(&self, type_info: &TypeInfo) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known, // Из синтакс-помощника - всегда Known
            result: ResolutionResult::Concrete(
                ConcreteType::Platform(PlatformType {
                    // Используем русское имя как основное
                    name: type_info.identity.russian_name.clone(),
                    // Конвертируем методы и свойства
                    methods: self.extract_methods(type_info),
                    properties: self.extract_properties(type_info),
                })
            ),
            source: ResolutionSource::Static, // Статические данные из документации
            metadata: ResolutionMetadata {
                file: Some(format!("syntax_helper:{}", type_info.identity.catalog_path)),
                line: None,
                column: None,
                notes: vec![
                    // Сохраняем всю важную информацию в notes
                    format!("ru:{} en:{}", 
                        type_info.identity.russian_name, 
                        type_info.identity.english_name),
                    type_info.documentation.type_description.clone(),
                    format!("category:{}", 
                        type_info.documentation.category_description.as_ref()
                            .unwrap_or(&"".to_string())),
                    format!("aliases:{}", type_info.identity.aliases.join(",")),
                ],
            },
            active_facet: type_info.metadata.default_facet,
            available_facets: type_info.metadata.available_facets.clone(),
        }
    }
    
    /// TypeInfo остаётся внутренней структурой парсера
    /// используется только для организации кода при парсинге
}
```

### 5.2 Хранение двуязычной информации

```rust
// PlatformType остаётся неизменным для совместимости:
// pub struct PlatformType {
//     pub name: String,
//     pub methods: Vec<Method>,
//     pub properties: Vec<Property>,
// }

// Вся дополнительная информация хранится в ResolutionMetadata.notes:
// - "ru:ТаблицаЗначений en:ValueTable" - имена на двух языках
// - "category:Таблица значений" - категория из catalog236.html
// - "aliases:ТЗ,ValueTable,ТаблицаЗначений" - все варианты имён

// Парсер только собирает данные, индексы строит PlatformTypesResolverV2:
pub struct SyntaxHelperParserV3 {
    // База данных с узлами
    database: SyntaxHelperDatabase,
    
    // TypeInfo используется только внутри парсера
    // и не экспортируется наружу
    // Индексы НЕ хранятся в парсере!
}
```

### 5.3 Определение фасетов

```rust
impl SyntaxHelperParserV3 {
    /// Определяет доступные фасеты для типа на основе HTML содержимого
    fn detect_facets(&self, document: &Html, type_info: &TypeInfo) -> Vec<FacetKind> {
        let mut facets = vec![];
        let description = type_info.documentation.type_description.as_str();
        let type_name = &type_info.identity.russian_name;
        
        // Коллекции
        if description.contains("коллекция") || 
           description.contains("Для объекта доступен обход") ||
           description.contains("посредством оператора Для каждого") ||
           type_name.contains("Таблица") || 
           type_name.contains("Список") ||
           type_name.contains("Массив") ||
           type_name.contains("Соответствие") {
            facets.push(FacetKind::Collection);
        }
        
        // Конструкторы
        if type_info.structure.constructors.len() > 0 ||
           description.contains("Новый") {
            facets.push(FacetKind::Constructor);
        }
        
        // Singleton для глобальных объектов
        if self.is_global_object(type_name) {
            facets.push(FacetKind::Singleton);
        }
        
        // Менеджеры (для будущих конфигурационных типов)
        if type_name.ends_with("Manager") || 
           type_name.contains("Менеджер") {
            facets.push(FacetKind::Manager);
        }
        
        facets
    }
    
    /// Определяет фасет по умолчанию
    fn detect_default_facet(&self, facets: &[FacetKind]) -> Option<FacetKind> {
        // Приоритеты фасетов
        if facets.contains(&FacetKind::Collection) {
            return Some(FacetKind::Collection);
        }
        if facets.contains(&FacetKind::Singleton) {
            return Some(FacetKind::Singleton);
        }
        if facets.contains(&FacetKind::Manager) {
            return Some(FacetKind::Manager);
        }
        facets.first().copied()
    }
}
```

## 6. API для использования

```rust
impl SyntaxHelperDatabase {
    /// Получить узел по пути (внутренний метод)
    pub fn get_node(&self, path: &str) -> Option<&SyntaxNode> {
        self.nodes.get(path)
    }
    
    /// Получить все типы (для построения индексов)
    pub fn get_all_types(&self) -> Vec<(&String, &SyntaxNode)> {
        self.nodes.iter()
            .filter(|(_, node)| matches!(node, SyntaxNode::Type(_)))
            .collect()
    }
}
```

## 6. Интеграция с PlatformTypesResolverV2

```rust
/// Расширение PlatformTypesResolverV2 для работы с парсером v3
impl PlatformTypesResolverV2 {
    /// Загружает данные из парсера v3 и строит индексы
    pub fn load_from_parser_v3(&mut self, parser: SyntaxHelperParserV3) {
        // 1. Сохраняем базу данных
        self.syntax_database = Some(parser.database);
        
        // 2. Строим индексы из данных парсера
        self.type_index = self.build_indexes_from_database();
    }
    
    /// Строит индексы для быстрого поиска
    fn build_indexes_from_database(&self) -> TypeIndex {
        let mut index = TypeIndex::default();
        
        if let Some(ref db) = self.syntax_database {
            for (path, node) in &db.nodes {
                if let SyntaxNode::Type(type_info) = node {
                    // Парсим имена из notes в ResolutionMetadata
                    let resolution = self.node_to_resolution(node);
                    let (ru_name, en_name) = self.extract_names_from_notes(&resolution.metadata.notes);
                    
                    // Строим индексы
                    index.by_russian.insert(ru_name, path.clone());
                    index.by_english.insert(en_name, path.clone());
                    // ... и т.д.
                }
            }
        }
        
        index
    }
    
    /// Поиск типа с использованием индексов
    pub fn resolve_type(&self, name: &str) -> Option<TypeResolution> {
        // 1. Проверяем индекс русских имён
        if let Some(path) = self.type_index.by_russian.get(name) {
            return self.get_type_resolution_by_path(path);
        }
        
        // 2. Проверяем индекс английских имён
        if let Some(path) = self.type_index.by_english.get(name) {
            return self.get_type_resolution_by_path(path);
        }
        
        // 3. Проверяем альтернативные имена
        if let Some(paths) = self.type_index.by_any_name.get(name) {
            if let Some(path) = paths.first() {
                return self.get_type_resolution_by_path(path);
            }
        }
        
        None
    }
    
    /// Получение документации для hover
    pub fn get_hover_info(&self, type_name: &str) -> Option<HoverInfo> {
        let type_resolution = self.resolve_type(type_name)?;
        
        // Извлекаем информацию из metadata
        let notes = &type_resolution.metadata.notes;
        let category = notes.iter()
            .find(|n| n.starts_with("Категория:"))
            .map(|n| n.replace("Категория: ", ""));
        
        Some(HoverInfo {
            type_name: type_name.to_string(),
            description: notes.first().cloned(),
            category,
            facets: type_resolution.available_facets.clone(),
            certainty: type_resolution.certainty,
        })
    }
    
    // Добавляем поля для индексов
    type_index: TypeIndex,
}
```

## 7. Интеграция с LSP

```rust
impl LanguageServer for BslLanguageServer {
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let type_name = extract_type_at_position(&params);
        
        let documentation = self.syntax_helper_db
            .get_hover_documentation(&type_name)?;
        
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: documentation,
            }),
            range: None,
        }))
    }
    
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let context = extract_context(&params);
        
        // Если это обращение к типу
        if let Some(type_name) = context.type_name {
            let type_info = self.syntax_helper_db.find_type(&type_name)?;
            
            let items = type_info.structure.methods.iter()
                .map(|method_name| CompletionItem {
                    label: method_name.clone(),
                    kind: Some(CompletionItemKind::METHOD),
                    documentation: self.get_method_doc(&type_name, method_name),
                    ..Default::default()
                })
                .collect();
            
            return Ok(Some(CompletionResponse::Array(items)));
        }
        
        Ok(None)
    }
}
```

## 7. Обработка особых случаев

### 7.1 Определение фасетов
```rust
fn detect_facet(document: &Html, type_name: &str) -> Option<Facet> {
    // Анализируем описание типа
    let description = extract_description(document);
    
    // Ключевые слова для определения фасета
    if description.contains("менеджер") || type_name.ends_with("Manager") {
        return Some(Facet::Manager);
    }
    
    if description.contains("ссылка") || type_name.ends_with("Ref") {
        return Some(Facet::Reference);
    }
    
    if description.contains("объект") && description.contains("изменен") {
        return Some(Facet::Object);
    }
    
    // Анализируем методы
    let methods = extract_method_links(document);
    if methods.iter().any(|m| m.contains("НайтиПоКоду") || m.contains("CreateItem")) {
        return Some(Facet::Manager);
    }
    
    if methods.iter().any(|m| m.contains("Записать") || m.contains("Write")) {
        return Some(Facet::Object);
    }
    
    None
}
```

### 7.2 Парсинг примеров кода
```rust
fn extract_examples(document: &Html) -> Vec<CodeExample> {
    let mut examples = Vec::new();
    
    // Ищем секцию "Пример:"
    let table_selector = Selector::parse("table").unwrap();
    for table in document.select(&table_selector) {
        if let Some(code_element) = table.select(&Selector::parse("font").unwrap()).next() {
            let code = extract_code_from_html(code_element);
            examples.push(CodeExample {
                description: None,
                code,
                language: "bsl".to_string(),
            });
        }
    }
    
    examples
}
```

## 8. Требования к производительности

- Парсинг полного архива синтакс-помощника: < 10 секунд
- Поиск типа по имени: < 1 мс
- Получение документации для hover: < 10 мс
- Построение списка автодополнения: < 50 мс

## 9. Тестирование

### 9.1 Unit тесты
- Парсинг отдельных HTML файлов
- Извлечение русских/английских названий
- Построение индексов
- Поиск по разным типам имён

### 9.2 Интеграционные тесты
- Парсинг catalog236 (ТаблицаЗначений)
- Проверка связей между типами
- Полный цикл от парсинга до LSP hover

### 9.3 Snapshot тесты
- Сохранение эталонных результатов парсинга
- Проверка регрессий при изменениях

## 10. Документация

Необходимо создать:
1. README с примерами использования
2. Документацию API для разработчиков
3. Примеры интеграции с LSP
4. Описание формата данных синтакс-помощника

## 11. Ключевые изменения в v3

### 11.1 Интеграция с архитектурой проекта
- Все типы обёрнуты в `TypeResolution` для градуальной типизации
- Поддержка уровней уверенности (Certainty)
- Интеграция с `ResolutionSource` для отслеживания источника информации
- **TypeInfo остаётся внутренней структурой парсера** (не экспортируется)
- **Прямое создание TypeResolution без промежуточных конвертаций**
- **PlatformType остаётся простым** (name, methods, properties)
- **Вся метаинформация в ResolutionMetadata.notes**

### 11.2 Фасетная система
- Определение фасетов из содержимого HTML
- `FacetKind::Collection` для коллекций (ТаблицаЗначений, Массив, Список)
- `FacetKind::Constructor` для типов с конструкторами
- `FacetKind::Singleton` для глобальных объектов

### 11.3 Двуязычность и индексы
- Отдельные индексы для русских и английских имён
- Извлечение имён из HTML, а не из путей файлов
- Поддержка поиска по любому варианту имени

### 11.4 Категории и полная документация
- Парсинг файлов категорий (catalog236.html)
- Сохранение полной документации для hover в IDE
- Связь типов с их категориями

### 11.5 Discovery-based подход
- Никаких предположений о структуре
- Рекурсивный обход всех каталогов
- Определение типа узла по содержимому HTML

### 11.6 Разделение ответственности
- **SyntaxHelperParserV3** - только парсинг и сбор данных
- **SyntaxHelperDatabase** - промежуточное хранилище без индексов
- **PlatformTypesResolverV2** - построение индексов и поиск
- **TypeInfo** - внутренняя структура парсера (не экспортируется)
- **TypeResolution** - финальное представление для системы типов