//! Провайдер документации платформенных типов

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO: Update imports after documentation system refactoring
// use super::core::hierarchy::{
//     AvailabilityContext, CodeExample, DocumentationNode, MethodDocumentation,
//     PropertyDocumentation, RootCategoryNode, TypeDocumentationFull, UiMetadata,
// };
// use super::core::providers::{DocumentationProvider, ProviderConfig};
// use super::core::statistics::{InitializationStatus, ProviderStatistics};
use super::search::AdvancedSearchQuery;
use crate::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use crate::domain::types::{FacetKind, Method, Property, TypeResolution};

/// Провайдер документации платформенных типов
///
/// Извлекает информацию из справки синтакс-помощника 1С
/// и предоставляет её в унифицированном формате
pub struct PlatformDocumentationProvider {
    /// Парсер справки синтакс-помощника
    syntax_parser: Arc<RwLock<SyntaxHelperParser>>,

    /// Статус инициализации
    initialization_status: Arc<RwLock<InitializationStatus>>,

    /// Кеш типов
    types_cache: Arc<RwLock<std::collections::HashMap<String, TypeDocumentationFull>>>,

    /// Корневая категория
    root_category_cache: Arc<RwLock<Option<RootCategoryNode>>>,

    /// Конфигурация провайдера
    config: Arc<RwLock<Option<PlatformProviderConfig>>>,
}

/// Конфигурация провайдера платформенных типов
#[derive(Debug, Clone)]
pub struct PlatformProviderConfig {
    /// Путь к справке синтакс-помощника
    pub syntax_helper_path: String,

    /// Версия платформы
    pub platform_version: String,

    /// Фильтры по доступности
    pub availability_filters: Vec<AvailabilityContext>,

    /// Включить экспериментальные типы
    pub include_experimental: bool,

    /// Настройки парсинга
    pub parsing_settings: PlatformParsingSettings,
}

/// Настройки парсинга платформенных типов
#[derive(Debug, Clone)]
pub struct PlatformParsingSettings {
    /// Количество потоков
    pub worker_threads: usize,

    /// Размер батча для обработки
    pub batch_size: usize,

    /// Парсить методы
    pub parse_methods: bool,

    /// Парсить свойства
    pub parse_properties: bool,

    /// Парсить примеры кода
    pub parse_examples: bool,

    /// Показывать прогресс
    pub show_progress: bool,
}

impl PlatformDocumentationProvider {
    /// Создать новый провайдер
    pub fn new() -> Self {
        Self {
            syntax_parser: Arc::new(RwLock::new(SyntaxHelperParser::new())),
            initialization_status: Arc::new(RwLock::new(InitializationStatus::default())),
            types_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            root_category_cache: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(None)),
        }
    }

    /// Инициализация с конфигурацией платформы
    pub async fn initialize_with_platform_config(
        &self,
        config: PlatformProviderConfig,
    ) -> Result<()> {
        // Сохраняем конфигурацию
        *self.config.write().await = Some(config.clone());

        // Обновляем статус
        {
            let mut status = self.initialization_status.write().await;
            status.is_initializing = true;
            status.current_operation = "Инициализация провайдера платформенных типов".to_string();
            status.progress_percent = 0;
        }

        // Инициализируем парсер
        let parser_config = ProviderConfig {
            data_source: config.syntax_helper_path.clone(),
            ..Default::default()
        };

        self.initialize(&parser_config).await
    }

    /// Получить количество загруженных типов
    pub async fn get_loaded_types_count(&self) -> usize {
        self.types_cache.read().await.len()
    }

    /// Получить типы по категории
    pub async fn get_types_by_category(
        &self,
        category_name: &str,
    ) -> Result<Vec<TypeDocumentationFull>> {
        let cache = self.types_cache.read().await;

        Ok(cache
            .values()
            .filter(|t| t.hierarchy_path.iter().any(|p| p == category_name))
            .cloned()
            .collect())
    }

    /// Конвертировать SyntaxNode в TypeDocumentationFull
    async fn convert_syntax_node_to_documentation(
        &self,
        node: &crate::data::loaders::syntax_helper_parser::SyntaxNode,
    ) -> Result<TypeDocumentationFull> {
        use super::core::hierarchy::DocumentationSourceType;
        use crate::data::loaders::syntax_helper_parser::SyntaxNode;
        use crate::domain::types::{
            Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult,
            ResolutionSource, TypeResolution,
        };

        match node {
            SyntaxNode::Type(type_info) => {
                // Создаем PlatformType для TypeResolution
                let platform_type = PlatformType {
                    name: type_info.identity.russian_name.clone(),
                    methods: self.convert_methods(&type_info.structure.methods).await?,
                    properties: self
                        .convert_properties(&type_info.structure.properties)
                        .await?,
                };

                // Создаем TypeResolution с полной интеграцией
                let type_resolution = TypeResolution {
                    certainty: Certainty::Known, // Из справки 1С - всегда Known
                    result: ResolutionResult::Concrete(ConcreteType::Platform(platform_type)),
                    source: ResolutionSource::Static,
                    metadata: ResolutionMetadata {
                        file: Some(format!("syntax_helper:{}", type_info.identity.catalog_path)),
                        line: None,
                        column: None,
                        notes: vec![
                            format!(
                                "ru:{} en:{}",
                                type_info.identity.russian_name, type_info.identity.english_name
                            ),
                            type_info.documentation.type_description.clone(),
                            format!(
                                "category:{}",
                                type_info
                                    .documentation
                                    .category_description
                                    .as_ref()
                                    .unwrap_or(&"".to_string())
                            ),
                            format!("aliases:{}", type_info.identity.aliases.join(",")),
                        ],
                    },
                    active_facet: type_info.metadata.default_facet,
                    available_facets: type_info.metadata.available_facets.clone(),
                };

                // Конвертируем методы в полную документацию
                let methods = self
                    .convert_methods_full(&type_info.structure.methods)
                    .await?;

                // Конвертируем свойства в полную документацию
                let properties = self
                    .convert_properties_full(&type_info.structure.properties)
                    .await?;

                // Конвертируем примеры
                let examples = type_info
                    .documentation
                    .examples
                    .iter()
                    .map(|ex| CodeExample {
                        title: ex
                            .description
                            .clone()
                            .unwrap_or_else(|| "Пример использования".to_string()),
                        code: ex.code.clone(),
                        language: ex.language.clone(),
                        expected_output: None,
                        executable: false,
                    })
                    .collect();

                // Конвертируем доступность
                let availability = type_info
                    .documentation
                    .availability
                    .iter()
                    .filter_map(|avail| self.parse_availability_context(avail))
                    .collect();

                Ok(TypeDocumentationFull {
                    // === ИДЕНТИФИКАЦИЯ ===
                    id: type_info.identity.catalog_path.clone(),
                    russian_name: type_info.identity.russian_name.clone(),
                    english_name: type_info.identity.english_name.clone(),
                    aliases: type_info.identity.aliases.clone(),

                    // === КЛАССИФИКАЦИЯ ===
                    source_type: DocumentationSourceType::Platform {
                        version: type_info.documentation.since_version.clone(),
                    },
                    hierarchy_path: self.build_hierarchy_path(&type_info.identity.category_path),

                    // === ГРАДУАЛЬНАЯ ТИПИЗАЦИЯ ===
                    type_resolution,
                    available_facets: type_info.metadata.available_facets.clone(),
                    active_facet: type_info.metadata.default_facet,

                    // === СТРУКТУРА ===
                    methods,
                    properties,
                    constructors: Vec::new(), // TODO: конвертировать конструкторы

                    // === ДОКУМЕНТАЦИЯ ===
                    description: type_info.documentation.type_description.clone(),
                    examples,
                    availability,
                    since_version: type_info.documentation.since_version.clone(),
                    notes: Vec::new(),

                    // === СВЯЗИ ===
                    related_types: Vec::new(), // TODO: найти связанные типы
                    parent_type: None,
                    child_types: Vec::new(),

                    // === МЕТАДАННЫ ===
                    source_file: Some(type_info.identity.catalog_path.clone()),
                    ui_metadata: UiMetadata {
                        icon: self.get_type_icon(&type_info.identity.russian_name),
                        color: self.get_type_color(&type_info.metadata.available_facets),
                        tree_path: self.build_hierarchy_path(&type_info.identity.category_path),
                        expanded: false,
                        sort_weight: 0,
                        css_classes: vec![
                            format!("platform-type"),
                            format!(
                                "facet-{:?}",
                                type_info
                                    .metadata
                                    .default_facet
                                    .unwrap_or(FacetKind::Collection)
                            ),
                        ],
                    },
                })
            }

            _ => Err(anyhow::anyhow!(
                "Cannot convert non-Type SyntaxNode to TypeDocumentationFull"
            )),
        }
    }

    /// Построить корневую категорию платформенных типов
    async fn build_platform_root_category(&self) -> Result<RootCategoryNode> {
        use super::core::hierarchy::{CategoryStatistics, SubCategoryNode};
        use crate::data::loaders::syntax_helper_parser::SyntaxNode;

        let parser = self.syntax_parser.read().await;
        let database = parser.export_database();

        // Группируем типы по категориям
        let mut categories_map: std::collections::HashMap<String, Vec<_>> =
            std::collections::HashMap::new();
        let mut total_types = 0;
        let mut total_methods = 0;
        let mut total_properties = 0;

        for (path, node) in &database.nodes {
            match node {
                SyntaxNode::Type(type_info) => {
                    total_types += 1;
                    total_methods += type_info.structure.methods.len();
                    total_properties += type_info.structure.properties.len();

                    let category_name = if type_info.identity.category_path.is_empty() {
                        "Без категории".to_string()
                    } else {
                        // Берем первую часть пути как основную категорию
                        type_info
                            .identity
                            .category_path
                            .split('/')
                            .next()
                            .unwrap_or("Без категории")
                            .to_string()
                    };

                    categories_map
                        .entry(category_name)
                        .or_default()
                        .push((path.clone(), type_info.clone()));
                }
                SyntaxNode::Category(cat_info) => {
                    // Добавляем информацию о категории
                    categories_map.entry(cat_info.name.clone()).or_default();
                }
                _ => {}
            }
        }

        // Создаем подкатегории
        let mut children = Vec::new();

        for (category_name, types) in categories_map {
            if !types.is_empty() {
                let category_node = SubCategoryNode {
                    id: format!("platform_category_{}", category_name.replace(' ', "_")),
                    name: category_name.clone(),
                    description: format!("Платформенные типы категории: {}", category_name),
                    hierarchy_path: vec!["Платформа".to_string(), category_name.clone()],
                    children: Vec::new(), // TODO: добавить типы как дочерние узлы
                    ui_metadata: UiMetadata {
                        icon: "📂".to_string(),
                        color: "#569CD6".to_string(),
                        tree_path: vec!["Платформа".to_string(), category_name.clone()],
                        expanded: false,
                        sort_weight: 0,
                        css_classes: vec!["platform-category".to_string()],
                    },
                    statistics: CategoryStatistics {
                        child_types_count: types.len(),
                        total_methods_count: types
                            .iter()
                            .map(|(_, t)| t.structure.methods.len())
                            .sum(),
                        total_properties_count: types
                            .iter()
                            .map(|(_, t)| t.structure.properties.len())
                            .sum(),
                        most_popular_type: types
                            .first()
                            .map(|(_, t)| t.identity.russian_name.clone()),
                    },
                };

                children.push(DocumentationNode::SubCategory(category_node));
            }
        }

        Ok(RootCategoryNode {
            id: "platform_root".to_string(),
            name: "Платформа 1С:Предприятие".to_string(),
            description: "Встроенные типы и функции платформы 1С:Предприятие".to_string(),
            children,
            ui_metadata: UiMetadata {
                icon: "🏢".to_string(),
                color: "#0078D4".to_string(),
                tree_path: vec!["Платформа".to_string()],
                expanded: true,
                sort_weight: 100,
                css_classes: vec!["root-category".to_string(), "platform-root".to_string()],
            },
            statistics: CategoryStatistics {
                child_types_count: total_types,
                total_methods_count: total_methods,
                total_properties_count: total_properties,
                most_popular_type: Some("ТаблицаЗначений".to_string()),
            },
        })
    }
}

#[async_trait]
impl DocumentationProvider for PlatformDocumentationProvider {
    fn provider_id(&self) -> &str {
        "platform_types"
    }

    fn display_name(&self) -> &str {
        "Платформенные типы 1С"
    }

    async fn initialize(&self, config: &ProviderConfig) -> Result<()> {
        {
            let mut status = self.initialization_status.write().await;
            status.is_initializing = true;
            status.current_operation = "Загрузка синтакс-помощника".to_string();
            status.progress_percent = 10;
        }

        // Инициализируем парсер
        {
            let mut parser = self.syntax_parser.write().await;
            if std::path::Path::new(&config.data_source).exists() {
                parser.parse_directory(&config.data_source)?;
            }
        }

        {
            let mut status = self.initialization_status.write().await;
            status.current_operation = "Построение документации типов".to_string();
            status.progress_percent = 50;
        }

        // Строим кеш типов
        self.build_types_cache().await?;

        {
            let mut status = self.initialization_status.write().await;
            status.current_operation = "Создание корневой категории".to_string();
            status.progress_percent = 80;
        }

        // Строим корневую категорию
        let root_category = self.build_platform_root_category().await?;
        *self.root_category_cache.write().await = Some(root_category);

        {
            let mut status = self.initialization_status.write().await;
            status.is_initializing = false;
            status.progress_percent = 100;
            status.current_operation = "Провайдер платформенных типов готов".to_string();
        }

        Ok(())
    }

    async fn get_root_category(&self) -> Result<RootCategoryNode> {
        let cache = self.root_category_cache.read().await;
        match cache.as_ref() {
            Some(category) => Ok(category.clone()),
            None => {
                drop(cache);
                let category = self.build_platform_root_category().await?;
                *self.root_category_cache.write().await = Some(category.clone());
                Ok(category)
            }
        }
    }

    async fn get_type_details(&self, type_id: &str) -> Result<Option<TypeDocumentationFull>> {
        let cache = self.types_cache.read().await;

        println!("🔍 Поиск типа по ID: '{}'", type_id);
        println!("📊 Доступно типов в кеше: {}", cache.len());

        // Показываем первые несколько ключей для отладки
        if cache.len() > 0 {
            println!("🔑 Примеры ключей в кеше:");
            for (key, _) in cache.iter().take(5) {
                println!("   - {}", key);
            }
        }

        // Попробуем найти по частичному совпадению
        if let Some(found_type) = cache.get(type_id) {
            println!("✅ Точное совпадение найдено");
            return Ok(Some(found_type.clone()));
        }

        // Поиск по русскому названию
        for (_, type_doc) in cache.iter() {
            if type_doc.russian_name.contains(type_id)
                || type_doc.english_name.contains(type_id)
                || type_id.contains(&type_doc.russian_name)
            {
                println!(
                    "✅ Найдено по названию: {} -> {}",
                    type_id, type_doc.russian_name
                );
                return Ok(Some(type_doc.clone()));
            }
        }

        println!("❌ Тип '{}' не найден", type_id);
        Ok(None)
    }

    async fn search_types(&self, _query: &AdvancedSearchQuery) -> Result<Vec<DocumentationNode>> {
        // TODO: Реализовать поиск в платформенных типах
        Ok(Vec::new())
    }

    async fn get_all_types(&self) -> Result<Vec<TypeDocumentationFull>> {
        let cache = self.types_cache.read().await;
        Ok(cache.values().cloned().collect())
    }

    async fn get_statistics(&self) -> Result<ProviderStatistics> {
        let cache = self.types_cache.read().await;

        let types_count = cache.len();
        let total_methods: usize = cache.values().map(|t| t.methods.len()).sum();
        let total_properties: usize = cache.values().map(|t| t.properties.len()).sum();

        // Примерная оценка использования памяти
        let memory_mb = (types_count * 1024 + total_methods * 256 + total_properties * 128) as f64
            / (1024.0 * 1024.0);

        Ok(ProviderStatistics {
            total_types: types_count,
            total_methods,
            total_properties,
            last_load_time_ms: 0, // TODO: засекать время загрузки
            memory_usage_mb: memory_mb,
        })
    }

    async fn get_initialization_status(&self) -> Result<InitializationStatus> {
        Ok(self.initialization_status.read().await.clone())
    }

    async fn check_availability(&self) -> Result<bool> {
        let config = self.config.read().await;
        match config.as_ref() {
            Some(cfg) => Ok(std::path::Path::new(&cfg.syntax_helper_path).exists()),
            None => Ok(false),
        }
    }

    async fn refresh(&self) -> Result<()> {
        // Очищаем кеши
        self.types_cache.write().await.clear();
        *self.root_category_cache.write().await = None;

        // Переинициализируем
        if let Some(config) = self.config.read().await.as_ref() {
            let provider_config = ProviderConfig {
                data_source: config.syntax_helper_path.clone(),
                ..Default::default()
            };
            self.initialize(&provider_config).await?;
        }

        Ok(())
    }
}

impl PlatformDocumentationProvider {
    /// Построить кеш типов из парсера
    async fn build_types_cache(&self) -> Result<()> {
        use crate::data::loaders::syntax_helper_parser::SyntaxNode;

        let parser = self.syntax_parser.read().await;
        let database = parser.export_database();

        let mut cache = self.types_cache.write().await;

        for (path, node) in &database.nodes {
            if let SyntaxNode::Type(_) = node {
                if let Ok(type_doc) = self.convert_syntax_node_to_documentation(node).await {
                    cache.insert(path.clone(), type_doc);
                }
            }
        }

        println!("📊 Построен кеш платформенных типов: {} типов", cache.len());
        Ok(())
    }

    /// Конвертировать методы для TypeResolution
    async fn convert_methods(&self, method_names: &[String]) -> Result<Vec<Method>> {
        // TODO: Получить полную информацию о методах из парсера
        Ok(method_names
            .iter()
            .map(|name| Method {
                name: name.clone(),
                parameters: Vec::new(), // TODO: загрузить параметры
                return_type: None,
                is_function: false,
            })
            .collect())
    }

    /// Конвертировать свойства для TypeResolution
    async fn convert_properties(&self, property_names: &[String]) -> Result<Vec<Property>> {
        // TODO: Получить полную информацию о свойствах из парсера
        Ok(property_names
            .iter()
            .map(|name| Property {
                name: name.clone(),
                type_: "Dynamic".to_string(), // TODO: определить тип свойства
                readonly: false,
            })
            .collect())
    }

    /// Конвертировать методы в полную документацию
    async fn convert_methods_full(
        &self,
        method_names: &[String],
    ) -> Result<Vec<MethodDocumentation>> {
        // TODO: Загрузить полную информацию о методах включая параметры и примеры
        Ok(method_names
            .iter()
            .map(|name| {
                // Разбираем русское и английское название
                let (russian_name, english_name) = self.parse_method_name(name);

                MethodDocumentation {
                    name: name.clone(),
                    russian_name,
                    english_name,
                    description: format!("Метод {}", name), // TODO: загрузить реальное описание
                    parameters: Vec::new(),                 // TODO: загрузить параметры
                    return_type: None,                      // TODO: определить возвращаемый тип
                    examples: Vec::new(),                   // TODO: загрузить примеры
                    availability: Vec::new(),               // TODO: загрузить доступность
                    exceptions: Vec::new(),
                }
            })
            .collect())
    }

    /// Конвертировать свойства в полную документацию
    async fn convert_properties_full(
        &self,
        property_names: &[String],
    ) -> Result<Vec<PropertyDocumentation>> {
        Ok(property_names
            .iter()
            .map(|name| {
                let (russian_name, english_name) = self.parse_property_name(name);

                PropertyDocumentation {
                    name: name.clone(),
                    russian_name,
                    english_name,
                    property_type: TypeResolution::unknown(), // TODO: определить тип свойства
                    description: format!("Свойство {}", name),
                    readonly: false, // TODO: определить из справки
                    examples: Vec::new(),
                }
            })
            .collect())
    }

    /// Парсинг названия метода (извлечение русского/английского)
    fn parse_method_name(&self, name: &str) -> (String, String) {
        // Название может быть в формате "Добавить (Add)" или просто "Добавить"
        if let Some(open) = name.find('(') {
            if let Some(close) = name.find(')') {
                let russian = name[..open].trim().to_string();
                let english = name[open + 1..close].trim().to_string();
                return (russian, english);
            }
        }
        // Если нет скобок, считаем что это русское название
        (name.trim().to_string(), String::new())
    }

    /// Парсинг названия свойства
    fn parse_property_name(&self, name: &str) -> (String, String) {
        self.parse_method_name(name) // Используем ту же логику
    }

    /// Парсинг контекста доступности
    fn parse_availability_context(&self, availability: &str) -> Option<AvailabilityContext> {
        match availability.to_lowercase().as_str() {
            "клиент" | "client" => Some(AvailabilityContext::Client),
            "сервер" | "server" => Some(AvailabilityContext::Server),
            "внешнее соединение" | "external connection" => {
                Some(AvailabilityContext::ExternalConnection)
            }
            "мобильное приложение" | "mobile app" => {
                Some(AvailabilityContext::MobileApp)
            }
            "мобильный сервер" | "mobile server" => {
                Some(AvailabilityContext::MobileServer)
            }
            "веб-клиент" | "web client" => Some(AvailabilityContext::WebClient),
            _ => None,
        }
    }

    /// Построить путь в иерархии
    fn build_hierarchy_path(&self, category_path: &str) -> Vec<String> {
        if category_path.is_empty() {
            vec!["Платформа".to_string(), "Без категории".to_string()]
        } else {
            let mut path = vec!["Платформа".to_string()];
            path.extend(category_path.split('/').map(|s| s.to_string()));
            path
        }
    }

    /// Получить иконку для типа
    fn get_type_icon(&self, type_name: &str) -> String {
        match type_name {
            name if name.contains("Таблица") => "📊".to_string(),
            name if name.contains("Массив") => "📋".to_string(),
            name if name.contains("Структура") => "🏗️".to_string(),
            name if name.contains("Соответствие") => "🗺️".to_string(),
            name if name.contains("Список") => "📝".to_string(),
            _ => "📄".to_string(),
        }
    }

    /// Получить цвет для типа по фасетам
    fn get_type_color(&self, facets: &[FacetKind]) -> String {
        if facets.contains(&FacetKind::Collection) {
            "#4CAF50".to_string() // Зеленый для коллекций
        } else if facets.contains(&FacetKind::Manager) {
            "#FF9800".to_string() // Оранжевый для менеджеров
        } else if facets.contains(&FacetKind::Constructor) {
            "#2196F3".to_string() // Синий для конструкторов
        } else {
            "#9E9E9E".to_string() // Серый по умолчанию
        }
    }
}

impl Default for PlatformProviderConfig {
    fn default() -> Self {
        Self {
            syntax_helper_path: "examples/syntax_helper/rebuilt.shcntx_ru".to_string(),
            platform_version: "8.3.23".to_string(),
            availability_filters: Vec::new(),
            include_experimental: false,
            parsing_settings: PlatformParsingSettings::default(),
        }
    }
}

impl Default for PlatformParsingSettings {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            batch_size: 100,
            parse_methods: true,
            parse_properties: true,
            parse_examples: true,
            show_progress: true,
        }
    }
}
