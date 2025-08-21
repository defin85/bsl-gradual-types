# Идеальная архитектура системы типов BSL

> **Подход**: Проектирование с чистого листа, абстрагируясь от текущего кода
> **Дата**: 2025-08-20

## 🎯 Цели идеальной системы типов

### **Что должна делать система:**
1. **Разрешение типов** - определить тип выражения `ТаблицаЗначений.Добавить()`
2. **Автодополнение** - предложить методы при вводе `Массив.`
3. **Документация** - показать полную справку по типу `Структура`
4. **Поиск** - найти все типы содержащие "Табли"
5. **Иерархия** - показать дерево всех типов BSL
6. **Валидация** - проверить корректность присваивания типов
7. **Конфигурационные типы** - поддержка пользовательских типов
8. **Производительность** - быстрая работа на больших проектах

## 📊 Источники данных о типах

### **1. Платформенные типы (HTML справка 1С):**
```
Источник: examples/syntax_helper/rebuilt.shcntx_ru/objects/
Содержит: ~13,593 типа
Примеры: Массив, ТаблицаЗначений, Структура, Справочники, etc.
Данные: Названия, методы, свойства, описания, иерархия
```

### **2. Конфигурационные типы (XML конфигурации):**
```
Источник: Configuration.xml + Catalogs/*.xml + Documents/*.xml
Содержит: Пользовательские типы
Примеры: Справочники.Контрагенты, Документы.ЗаказНаряд
Данные: Атрибуты, табличные части, связи, фасеты
```

### **3. Пользовательские типы (BSL код):**  
```
Источник: *.bsl файлы проекта
Содержит: Переменные, функции, процедуры
Примеры: локальные переменные, пользовательские функции
Данные: Типы переменных, сигнатуры функций
```

## 🏗️ Идеальные абстракции

### **Центральная абстракция: TypeResolution**
```rust
// НЕ тип, а "разрешение типа" с уровнем уверенности
pub struct TypeResolution {
    certainty: Certainty,      // Known | Inferred(0.8) | Unknown
    result: ResolutionResult,  // Concrete | Union | Conditional | Dynamic
    context: TypeContext,      // Где и как используется
    facets: Vec<Facet>,        // Manager | Object | Reference
    metadata: ResolutionMetadata, // Отладочная информация
}
```

### **Вспомогательные абстракции:**
```rust
// Сырые данные о типе (из парсеров)
pub struct TypeInfo {
    name: String,
    english_name: String, 
    methods: Vec<MethodInfo>,
    properties: Vec<PropertyInfo>,
    documentation: String,
    source: TypeSource, // Platform | Configuration | UserDefined
}

// Полная документация типа (для UI)
pub struct TypeDocumentation {
    basic_info: TypeInfo,
    resolution: TypeResolution,
    usage_examples: Vec<CodeExample>,
    related_types: Vec<String>,
    ui_metadata: UiMetadata,
}

// Результат поиска
pub struct TypeSearchResult {
    type_info: TypeInfo,
    relevance_score: f32,
    match_highlights: Vec<TextSpan>,
}
```

## 👥 Потребители системы и их потребности

### **1. LSP Сервер:**
```rust
// Нужны: БЫСТРЫЕ операции
trait LspTypeProvider {
    fn resolve_expression(expr: &str) -> TypeResolution;     // <10ms
    fn get_completions(prefix: &str) -> Vec<CompletionItem>; // <50ms  
    fn check_assignment(from: &Type, to: &Type) -> bool;     // <1ms
    fn get_hover_info(expr: &str) -> Option<HoverInfo>;      // <20ms
}
```

### **2. Веб-интерфейс:**
```rust  
// Нужны: БОГАТЫЕ данные
trait WebTypeProvider {
    fn get_all_types() -> Vec<TypeDocumentation>;           // Все типы с документацией
    fn get_type_hierarchy() -> TypeHierarchy;               // Полная иерархия
    fn search_types(query: &str) -> Vec<TypeSearchResult>;  // Поиск с подсветкой
    fn get_type_details(name: &str) -> TypeDocumentation;   // Детальная страница
}
```

### **3. CLI Инструменты:**
```rust
// Нужны: АНАЛИТИЧЕСКИЕ операции  
trait AnalysisTypeProvider {
    fn analyze_project(path: &Path) -> ProjectTypeAnalysis; // Анализ проекта
    fn get_type_coverage() -> TypeCoverageReport;           // Покрытие типизации
    fn validate_types(files: &[Path]) -> Vec<Diagnostic>;   // Валидация
}
```

## 🏗️ Идеальная слоистая архитектура

### **Data Layer (Данные):**
```rust
// Парсеры сырых данных
trait TypeDataParser {
    fn parse(&mut self) -> Result<Vec<RawTypeData>>;
}

struct HtmlSyntaxParser;    // Парсит HTML справки
struct XmlConfigParser;     // Парсит XML конфигурации  
struct BslCodeParser;       // Парсит BSL код

// Репозиторий данных
trait TypeRepository {
    fn save_types(&self, types: Vec<RawTypeData>) -> Result<()>;
    fn load_types(&self, filter: TypeFilter) -> Result<Vec<RawTypeData>>;
    fn get_type_by_name(&self, name: &str) -> Option<RawTypeData>;
}
```

### **Domain Layer (Бизнес-логика):**
```rust
// Основной сервис типов
struct TypeResolutionService {
    repository: Arc<dyn TypeRepository>,
    cache: Arc<TypeCache>,
    resolvers: Vec<Box<dyn TypeResolver>>,
}

impl TypeResolutionService {
    // Основной API для разрешения типов
    fn resolve_type(&self, expression: &str, context: &TypeContext) -> TypeResolution;
    fn get_type_info(&self, name: &str) -> Option<TypeInfo>;
    fn search_types(&self, query: &SearchQuery) -> Vec<TypeSearchResult>;
}

// Резолверы для разных типов
trait TypeResolver {
    fn can_resolve(&self, expression: &str) -> bool;
    fn resolve(&self, expression: &str, context: &TypeContext) -> TypeResolution;
}

struct PlatformTypeResolver;    // Справочники.Контрагенты
struct ConfigTypeResolver;      // Пользовательские типы  
struct BuiltinTypeResolver;     // Массив, Структура
struct ExpressionResolver;      // х.Получить().Длина()
```

### **Application Layer (Сервисы приложений):**
```rust
// Специализированные сервисы для разных потребителей
struct LspTypeService {
    core_service: Arc<TypeResolutionService>,
    lsp_cache: Arc<LspCache>,
}

struct WebTypeService {
    core_service: Arc<TypeResolutionService>,
    documentation_builder: DocumentationBuilder,
    search_engine: SearchEngine,
}

struct AnalysisTypeService {
    core_service: Arc<TypeResolutionService>,
    project_analyzer: ProjectAnalyzer,
    coverage_calculator: CoverageCalculator,
}
```

### **Presentation Layer (Интерфейсы):**
```rust
// Разные интерфейсы для разных потребителей
trait LspInterface {
    async fn resolve_expression(&self, expr: &str) -> TypeResolution;
    async fn get_completions(&self, prefix: &str) -> Vec<CompletionItem>;
}

trait WebInterface {  
    async fn get_type_hierarchy(&self) -> TypeHierarchy;
    async fn search_types(&self, query: &str) -> Vec<TypeSearchResult>;
    async fn get_type_documentation(&self, name: &str) -> TypeDocumentation;
}

trait CliInterface {
    async fn analyze_project(&self, path: &Path) -> ProjectAnalysis;
    async fn check_types(&self, files: &[Path]) -> Vec<Diagnostic>;
}
```

## 🎯 Ключевые принципы дизайна

### **1. Single Responsibility Principle:**
- **TypeRepository** - только хранение данных
- **TypeResolver** - только разрешение типов  
- **LspTypeService** - только LSP нужды
- **WebTypeService** - только веб нужды

### **2. Open/Closed Principle:**
- Легко добавлять новые **TypeResolver** (для новых источников)
- Легко добавлять новые **TypeDataParser** (для новых форматов)
- Легко добавлять новые **Interface** (для новых потребителей)

### **3. Dependency Inversion:**
- Сервисы зависят от **абстракций** (traits), не от конкретных классов
- Core не знает о Presentation layer
- Domain layer не знает о Data layer деталях

### **4. Single Source of Truth:**
- **Один TypeRepository** для всех данных о типах
- **Один TypeResolutionService** для всей бизнес-логики
- **Разные интерфейсы** для разных потребителей

## 💡 Идеальный data flow

### **Инициализация:**
```
1. HtmlSyntaxParser → parse HTML → RawTypeData[]
2. XmlConfigParser → parse XML → RawTypeData[]  
3. TypeRepository ← save all RawTypeData
4. TypeResolutionService ← load from repository
5. LspTypeService, WebTypeService ← inject TypeResolutionService
6. Interfaces ← inject specialized services
```

### **Использование:**
```
LSP запрос → LspInterface → LspTypeService → TypeResolutionService → TypeRepository
Web запрос → WebInterface → WebTypeService → TypeResolutionService → TypeRepository  
CLI запрос → CliInterface → AnalysisTypeService → TypeResolutionService → TypeRepository
```

## 🔧 Идеальная архитектура в коде

### **Центральный компонент:**
```rust
pub struct IdealTypeSystem {
    // === DATA LAYER ===
    repository: Arc<dyn TypeRepository>,
    
    // === DOMAIN LAYER ===  
    resolution_service: Arc<TypeResolutionService>,
    
    // === APPLICATION LAYER ===
    lsp_service: Arc<LspTypeService>,
    web_service: Arc<WebTypeService>, 
    analysis_service: Arc<AnalysisTypeService>,
    
    // === INFRASTRUCTURE ===
    cache: Arc<TypeCache>,
    metrics: Arc<TypeMetrics>,
}

impl IdealTypeSystem {
    /// Единственный метод инициализации
    pub async fn initialize(config: TypeSystemConfig) -> Result<Self> {
        // 1. Инициализируем репозиторий
        let repository = Arc::new(InMemoryTypeRepository::new());
        
        // 2. Парсим все источники данных
        let html_parser = HtmlSyntaxParser::new();
        let xml_parser = XmlConfigParser::new();
        
        let html_data = html_parser.parse(&config.html_path)?;
        let xml_data = xml_parser.parse(&config.config_path)?;
        
        // 3. Сохраняем в репозиторий
        repository.save_types(html_data).await?;
        repository.save_types(xml_data).await?;
        
        // 4. Создаём сервисы
        let resolution_service = Arc::new(TypeResolutionService::new(repository.clone()));
        let lsp_service = Arc::new(LspTypeService::new(resolution_service.clone()));
        let web_service = Arc::new(WebTypeService::new(resolution_service.clone()));
        let analysis_service = Arc::new(AnalysisTypeService::new(resolution_service.clone()));
        
        Ok(Self {
            repository,
            resolution_service, 
            lsp_service,
            web_service,
            analysis_service,
            cache: Arc::new(TypeCache::new()),
            metrics: Arc::new(TypeMetrics::new()),
        })
    }
    
    /// Получить интерфейс для LSP
    pub fn lsp_interface(&self) -> LspInterface {
        LspInterface::new(self.lsp_service.clone())
    }
    
    /// Получить интерфейс для веб
    pub fn web_interface(&self) -> WebInterface {
        WebInterface::new(self.web_service.clone())  
    }
    
    /// Получить интерфейс для CLI
    pub fn cli_interface(&self) -> CliInterface {
        CliInterface::new(self.analysis_service.clone())
    }
}
```

### **Использование в приложениях:**
```rust
// LSP сервер
async fn main() -> Result<()> {
    let type_system = IdealTypeSystem::initialize(config).await?;
    let lsp_interface = type_system.lsp_interface();
    
    // Все LSP операции через интерфейс
    let completions = lsp_interface.get_completions("Массив.").await;
    let hover = lsp_interface.get_hover_info("ТаблицаЗначений").await;
}

// Веб-сервер
async fn main() -> Result<()> {
    let type_system = IdealTypeSystem::initialize(config).await?;
    let web_interface = type_system.web_interface();
    
    // Все веб-операции через интерфейс
    async fn handle_hierarchy() -> Result<impl warp::Reply> {
        let hierarchy = web_interface.get_type_hierarchy().await;
        render_hierarchy(hierarchy)
    }
    
    async fn handle_search(query: String) -> Result<impl warp::Reply> {
        let results = web_interface.search_types(&query).await;
        render_search_results(results)
    }
}

// CLI инструменты
async fn main() -> Result<()> {
    let type_system = IdealTypeSystem::initialize(config).await?;
    let cli_interface = type_system.cli_interface();
    
    // Анализ проекта
    let analysis = cli_interface.analyze_project(&project_path).await;
    println!("Найдено {} ошибок типизации", analysis.errors.len());
}
```

## 📈 Преимущества идеальной архитектуры

### **1. Ясность ответственности:**
- **Repository** - только хранение
- **Service** - только бизнес-логика  
- **Interface** - только адаптация к потребителям

### **2. Масштабируемость:**
- Легко добавить новый источник типов (новый Parser)
- Легко добавить новый потребитель (новый Interface)
- Легко добавить новую функциональность (новый Service)

### **3. Тестируемость:**
- Каждый слой тестируется независимо
- Mock'и для каждого интерфейса
- Изолированные unit-тесты

### **4. Производительность:**
- Единый кеш на уровне Repository
- Специализированные кеши на уровне Services
- Lazy loading данных

### **5. Поддерживаемость:**
- Понятная структура кода
- Четкие границы между компонентами
- Легкая отладка проблем

## 🤔 Сравнение с текущим кодом

### **Текущий код:**
```
PlatformTypeResolver + PlatformDocumentationProvider + DocumentationSearchEngine
= 3 компонента с дублированием данных и логики
```

### **Идеальная архитектура:**
```
IdealTypeSystem {
    Repository + ResolutionService + 3 специализированных сервиса + 3 интерфейса
}
= Четкая слоистая архитектура без дублирования
```

## 🎯 Вопросы для обсуждения

1. **Стоит ли реализовывать идеальную архитектуру с нуля?**
2. **Или лучше эволюционно дорабатывать текущий код?** 
3. **Какие компромиссы готовы принять для скорости разработки?**
4. **Насколько важна perfect архитектура vs working solution?**

---

*Создано: 2025-08-20*  
*Статус: Концептуальный дизайн идеальной архитектуры*  
*Для обсуждения: Революция vs эволюция*