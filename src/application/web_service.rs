//! Web Type Service - сервис для веб-интерфейса с богатыми данными

use crate::data::loaders::syntax_helper_parser::{SyntaxHelperDatabase, SyntaxNode};
use crate::domain::types::TypeResolution;
use crate::presentation::SearchFilters;
use anyhow::Result;
use std::collections::HashMap;

/// Результат поиска типов
#[derive(Debug, Clone, Default)]
pub struct TypeSearchResult {
    pub name: String,
    pub description: String,
    pub type_name: String,
    pub category: String,
    pub relevance_score: f64,
    pub url: Option<String>,
}

// Используем единое определение TypeHierarchy из services.rs
use super::services::TypeHierarchy;

/// Результаты расширенного поиска
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub total: usize,
    pub items: Vec<TypeSearchResult>,
}

impl SearchResults {
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl std::ops::Index<usize> for SearchResults {
    type Output = TypeSearchResult;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

/// Метрики производительности
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub average_response_time: f64,
    pub cache_hit_rate: f64,
    pub active_connections: u32,
}

/// Статистика типов
#[derive(Debug, Clone, Default)]
pub struct TypeStatistics {
    pub total_types: usize,
    pub platform_types: usize,
    pub user_types: usize,
}

/// Сервис типов для веб-интерфейса (богатые данные)
pub struct WebTypeService {
    /// Центральный сервис разрешения типов (обязательный!)
    resolution_service: std::sync::Arc<crate::domain::repository::TypeResolutionService>,
}

impl WebTypeService {
    /// Создать новый веб сервис с обязательным resolution_service
    pub fn new(
        resolution_service: std::sync::Arc<crate::domain::repository::TypeResolutionService>,
    ) -> Self {
        Self { resolution_service }
    }

    /// Поиск типов для веб-интерфейса
    pub async fn search_types(&self, query: &str) -> Result<Vec<String>> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let results = self.resolution_service.search_types(query);
        Ok(results)
    }

    /// Получить детали типа
    pub async fn get_type_details(&self, type_name: &str) -> Result<Option<TypeResolution>> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let platform_globals = self.resolution_service.get_all_platform_globals();
        Ok(platform_globals.get(type_name).cloned())
    }

    /// Получить детальную информацию о типе для веб UI
    pub async fn get_type_completions(
        &self,
        expression: &str,
    ) -> Result<Vec<crate::domain::CompletionItem>> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let completions = self.resolution_service.get_completions(expression);
        Ok(completions)
    }
    /// Построить иерархию типов динамически на основе CategoryInfo
    pub async fn build_type_hierarchy_with_types(
        &self,
        all_types: &HashMap<String, TypeResolution>,
    ) -> Result<TypeHierarchy> {
        // Получаем базу данных синтакс-помощника
        let database = self.resolution_service.get_syntax_helper_database();

        if let Some(db) = database {
            println!(
                "✅ Найдена база данных синтакс-помощника с {} категориями",
                db.categories.len()
            );
            // Строим иерархию на основе CategoryInfo
            self.build_hierarchy_from_category_info(db, all_types).await
        } else {
            println!(
                "⚠️ База данных синтакс-помощника недоступна, используем простую категоризацию"
            );
            // Fallback: если нет базы данных, используем простую категоризацию
            self.build_simple_hierarchy(all_types).await
        }
    }

    /// Построить правильную иерархию с 10 основными категориями платформенных типов
    async fn build_hierarchy_from_category_info(
        &self,
        database: &SyntaxHelperDatabase,
        all_types: &HashMap<String, TypeResolution>,
    ) -> Result<TypeHierarchy> {
        let mut hierarchy = TypeHierarchy::default();

        println!("🏗️ Создаем правильную иерархию с основными категориями");

        // Создаем правильную иерархию с 10 основными категориями платформенных типов
        let mut platform_root = super::services::TypeCategory {
            id: "platform_types".to_string(),
            name: "Платформенные типы".to_string(),
            description: "Типы предоставляемые платформой 1С:Предприятие".to_string(),
            types: Vec::new(),
            subcategories: self.create_main_platform_categories(),
        };

        let mut configuration_root = super::services::TypeCategory {
            id: "configuration_types".to_string(),
            name: "Конфигурационные типы".to_string(),
            description: "Типы объектов метаданных конфигурации".to_string(),
            types: Vec::new(),
            subcategories: Vec::new(),
        };

        // Распределяем типы по основным категориям на основе TypeIdentity
        self.distribute_types_to_main_categories(&mut platform_root, database, all_types);

        // Добавляем нераспределенные типы в "Прочие"
        self.add_uncategorized_types(&mut platform_root, all_types);

        hierarchy.categories = vec![platform_root, configuration_root];

        Ok(hierarchy)
    }

    /// Создает 10 основных категорий платформенных типов
    fn create_main_platform_categories(&self) -> Vec<super::services::TypeCategory> {
        vec![
            super::services::TypeCategory {
                id: "collections".to_string(),
                name: "Универсальные коллекции".to_string(),
                description: "Массивы, списки, таблицы значений, соответствия".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "data_types".to_string(),
                name: "Базовые типы данных".to_string(),
                description: "Строки, числа, даты, булево, UUID".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "file_work".to_string(),
                name: "Работа с файлами и потоками".to_string(),
                description: "Файлы, текстовые документы, двоичные данные".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "web_internet".to_string(),
                name: "Интернет и веб-технологии".to_string(),
                description: "HTTP, FTP, веб-сервисы, почта".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "xml_json".to_string(),
                name: "XML, JSON и данные".to_string(),
                description: "Работа с XML, JSON, XDTO, схемами".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "ui_forms".to_string(),
                name: "Пользовательский интерфейс".to_string(),
                description: "Формы, элементы управления, диаграммы".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "data_analysis".to_string(),
                name: "Система компоновки данных".to_string(),
                description: "Компоновка данных, отчеты, запросы".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "system_admin".to_string(),
                name: "Администрирование".to_string(),
                description: "Пользователи, права, кластер, фоновые задания".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "integration".to_string(),
                name: "Интеграция и обмен".to_string(),
                description: "Планы обмена, внешние источники, COM".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
            super::services::TypeCategory {
                id: "other".to_string(),
                name: "Прочие".to_string(),
                description: "Остальные платформенные типы".to_string(),
                types: Vec::new(),
                subcategories: Vec::new(),
            },
        ]
    }

    /// Распределяет типы по основным категориям на основе их названий и категорий из базы данных
    fn distribute_types_to_main_categories(
        &self,
        platform_root: &mut super::services::TypeCategory,
        database: &SyntaxHelperDatabase,
        all_types: &HashMap<String, TypeResolution>,
    ) {
        for (_path, node) in &database.nodes {
            if let SyntaxNode::Type(type_info) = node {
                let type_name = &type_info.identity.russian_name;
                let category_path = &type_info.identity.category_path;

                if all_types.contains_key(type_name) {
                    let main_category_id = self.map_to_main_category(type_name, category_path);

                    // Находим основную категорию и добавляем тип
                    for main_category in &mut platform_root.subcategories {
                        if main_category.id == main_category_id {
                            if !main_category.types.contains(type_name) {
                                main_category.types.push(type_name.clone());
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Определяет основную категорию для типа на основе его имени и исходной категории
    fn map_to_main_category(&self, type_name: &str, category_path: &str) -> String {
        // Проверяем по имени типа
        if self.is_collection_type(type_name) {
            return "collections".to_string();
        }
        if self.is_data_type(type_name) {
            return "data_types".to_string();
        }
        if self.is_file_type(type_name) {
            return "file_work".to_string();
        }
        if self.is_web_type(type_name) {
            return "web_internet".to_string();
        }
        if self.is_xml_json_type(type_name) {
            return "xml_json".to_string();
        }
        if self.is_ui_type(type_name) {
            return "ui_forms".to_string();
        }
        if self.is_data_analysis_type(type_name) {
            return "data_analysis".to_string();
        }
        if self.is_admin_type(type_name, category_path) {
            return "system_admin".to_string();
        }
        if self.is_integration_type(type_name, category_path) {
            return "integration".to_string();
        }

        // По умолчанию - прочие
        "other".to_string()
    }

    /// Проверяет, является ли тип коллекцией
    fn is_collection_type(&self, type_name: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        name_lower.contains("массив")
            || name_lower.contains("array")
            || name_lower.contains("список")
            || name_lower.contains("list")
            || name_lower.contains("таблицазначений")
            || name_lower.contains("valuetable")
            || name_lower.contains("структура")
            || name_lower.contains("structure")
            || name_lower.contains("соответствие")
            || name_lower.contains("map")
            || name_lower.contains("дерево")
            || name_lower.contains("tree")
            || name_lower.contains("коллекция")
            || name_lower.contains("collection")
            || name_lower.contains("фиксированнаяструктура")
            || name_lower.contains("fixedstructure")
            || name_lower.contains("фиксированныймассив")
            || name_lower.contains("fixedarray")
    }

    /// Проверяет, является ли тип базовым типом данных
    fn is_data_type(&self, type_name: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        name_lower.contains("строка")
            || name_lower.contains("string")
            || name_lower.contains("число")
            || name_lower.contains("number")
            || name_lower.contains("дата")
            || name_lower.contains("date")
            || name_lower.contains("булево")
            || name_lower.contains("boolean")
            || name_lower.contains("уникальныйидентификатор")
            || name_lower.contains("uuid")
            || name_lower.contains("описаниетипов")
            || name_lower.contains("typedescription")
            || (name_lower == "тип" || name_lower == "type") // Только точное совпадение
    }

    /// Проверяет, связан ли тип с файлами
    fn is_file_type(&self, type_name: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        name_lower.contains("файл")
            || name_lower.contains("file")
            || name_lower.contains("текстовыйдокумент")
            || name_lower.contains("textdocument")
            || name_lower.contains("двоичныеданные")
            || name_lower.contains("binarydata")
            || name_lower.contains("поток")
            || name_lower.contains("stream")
            || name_lower.contains("чтение")
            || name_lower.contains("reader")
            || name_lower.contains("запись")
            || name_lower.contains("writer")
    }

    /// Проверяет, связан ли тип с веб-технологиями
    fn is_web_type(&self, type_name: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        name_lower.contains("http")
            || name_lower.contains("ftp")
            || name_lower.contains("веб")
            || name_lower.contains("web")
            || name_lower.contains("почта")
            || name_lower.contains("mail")
            || name_lower.contains("интернет")
            || name_lower.contains("internet")
            || name_lower.contains("ws")
                && (name_lower.contains("прокси") || name_lower.contains("proxy"))
    }

    /// Проверяет, связан ли тип с XML/JSON
    fn is_xml_json_type(&self, type_name: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        name_lower.contains("xml")
            || name_lower.contains("json")
            || name_lower.contains("xdto")
            || name_lower.contains("схема")
            || name_lower.contains("schema")
            || name_lower.contains("dom")
    }

    /// Проверяет, связан ли тип с пользовательским интерфейсом
    fn is_ui_type(&self, type_name: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        name_lower.contains("форма")
            || name_lower.contains("form")
            || name_lower.contains("элемент")
            || name_lower.contains("element")
            || name_lower.contains("кнопка")
            || name_lower.contains("button")
            || name_lower.contains("поле")
            || name_lower.contains("field")
            || name_lower.contains("диаграмма")
            || name_lower.contains("chart")
            || name_lower.contains("табличныйдокумент")
            || name_lower.contains("spreadsheetdocument")
            || name_lower.contains("графическаясхема")
            || name_lower.contains("graphicalschema")
    }

    /// Проверяет, связан ли тип с системой компоновки данных
    fn is_data_analysis_type(&self, type_name: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        name_lower.contains("компоновк")
            || name_lower.contains("composition")
            || name_lower.contains("отчет")
            || name_lower.contains("report")
            || name_lower.contains("запрос")
            || name_lower.contains("query")
            || name_lower.contains("построитель")
            || name_lower.contains("builder")
            || name_lower.contains("настройки") && name_lower.contains("компоновк")
    }

    /// Проверяет, связан ли тип с администрированием
    fn is_admin_type(&self, type_name: &str, category_path: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        let category_lower = category_path.to_lowercase();

        name_lower.contains("пользовател")
            || name_lower.contains("user")
            || name_lower.contains("администр")
            || name_lower.contains("admin")
            || name_lower.contains("кластер")
            || name_lower.contains("cluster")
            || name_lower.contains("фоновые")
            || name_lower.contains("background")
            || name_lower.contains("блокировк")
            || name_lower.contains("lock")
            || name_lower.contains("регламентн")
            || name_lower.contains("scheduled")
            || category_lower.contains("администрирование")
            || category_lower.contains("фоновые задания")
    }

    /// Проверяет, связан ли тип с интеграцией
    fn is_integration_type(&self, type_name: &str, category_path: &str) -> bool {
        let name_lower = type_name.to_lowercase();
        let category_lower = category_path.to_lowercase();

        name_lower.contains("планобмен") || name_lower.contains("exchangeplan") ||
        name_lower.contains("внешни") || name_lower.contains("external") ||
        name_lower.contains("com") && name_lower.len() < 20 || // Избегаем ложных срабатываний
        name_lower.contains("интеграци") || name_lower.contains("integration") ||
        category_lower.contains("планы обмена") ||
        category_lower.contains("внешние источники") ||
        category_lower.contains("интеграци")
    }

    /// Добавляет нераспределенные типы в категорию "Прочие"
    fn add_uncategorized_types(
        &self,
        platform_root: &mut super::services::TypeCategory,
        all_types: &HashMap<String, TypeResolution>,
    ) {
        // Собираем все уже распределенные типы
        let mut distributed_types = std::collections::HashSet::new();
        for category in &platform_root.subcategories {
            for type_name in &category.types {
                distributed_types.insert(type_name.clone());
            }
        }

        // Находим категорию "Прочие" и добавляем нераспределенные типы
        for category in &mut platform_root.subcategories {
            if category.id == "other" {
                for (type_name, resolution) in all_types {
                    // Только платформенные типы
                    let is_platform_type = match &resolution.result {
                        crate::domain::types::ResolutionResult::Concrete(concrete_type) => {
                            matches!(
                                concrete_type,
                                crate::domain::types::ConcreteType::Platform(_)
                                    | crate::domain::types::ConcreteType::Primitive(_)
                                    | crate::domain::types::ConcreteType::Special(_)
                                    | crate::domain::types::ConcreteType::GlobalFunction(_)
                            )
                        }
                        _ => true,
                    };

                    if is_platform_type && !distributed_types.contains(type_name) {
                        category.types.push(type_name.clone());
                    }
                }
                break;
            }
        }
    }

    /// Простая иерархия как fallback
    async fn build_simple_hierarchy(
        &self,
        all_types: &HashMap<String, TypeResolution>,
    ) -> Result<TypeHierarchy> {
        let mut hierarchy = TypeHierarchy::default();

        // Создаем два основных раздела
        let mut platform_root = super::services::TypeCategory {
            id: "platform_types".to_string(),
            name: "Платформенные типы".to_string(),
            description: "Типы предоставляемые платформой 1С:Предприятие".to_string(),
            types: Vec::new(),
            subcategories: Vec::new(),
        };

        let mut configuration_root = super::services::TypeCategory {
            id: "configuration_types".to_string(),
            name: "Конфигурационные типы".to_string(),
            description: "Типы объектов метаданных конфигурации".to_string(),
            types: Vec::new(),
            subcategories: Vec::new(),
        };

        // Временные HashMap для группировки подкатегорий
        let mut platform_subcategories: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut config_subcategories: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for (name, resolution) in all_types {
            // Определяем является ли тип платформенным или конфигурационным
            let is_platform_type = match &resolution.result {
                crate::domain::types::ResolutionResult::Concrete(concrete_type) => {
                    matches!(
                        concrete_type,
                        crate::domain::types::ConcreteType::Platform(_)
                            | crate::domain::types::ConcreteType::Primitive(_)
                            | crate::domain::types::ConcreteType::Special(_)
                            | crate::domain::types::ConcreteType::GlobalFunction(_)
                    )
                }
                _ => true, // По умолчанию считаем платформенным
            };

            if is_platform_type {
                // Категоризация платформенных типов
                let subcategory = if name.contains("Массив")
                    || name.contains("Array")
                    || name.contains("Структура")
                    || name.contains("Structure")
                    || name.contains("ТаблицаЗначений")
                    || name.contains("ValueTable")
                    || name.contains("Список")
                    || name.contains("List")
                    || name.contains("Соответствие")
                    || name.contains("Map")
                    || name.contains("Дерево")
                    || name.contains("Tree")
                {
                    "Универсальные коллекции"
                } else if name.contains("XMLЧтение")
                    || name.contains("XMLЗапись")
                    || name.contains("TextReader")
                    || name.contains("TextWriter")
                    || name.contains("Файл")
                    || name.contains("File")
                    || name.contains("ТекстовыйДокумент")
                    || name.contains("TextDocument")
                {
                    "Файлы и потоки"
                } else if name.contains("HTTPЗапрос")
                    || name.contains("HTTPОтвет")
                    || name.contains("WSПрокси")
                    || name.contains("WSAPI")
                    || name.contains("FTPСоединение")
                    || name.contains("FTPConnection")
                {
                    "Интернет и сеть"
                } else if name.contains("ОбъектXDTO")
                    || name.contains("XDTOObject")
                    || name.contains("JSON")
                    || name.contains("XML")
                {
                    "XML и JSON"
                } else if name.contains("УникальныйИдентификатор")
                    || name.contains("UUID")
                    || name.contains("Дата")
                    || name.contains("Date")
                    || name.contains("Строка")
                    || name.contains("String")
                    || name.contains("Число")
                    || name.contains("Number")
                    || name.contains("Булево")
                    || name.contains("Boolean")
                {
                    "Базовые типы"
                } else {
                    "Системные объекты"
                };

                platform_subcategories
                    .entry(subcategory.to_string())
                    .or_insert_with(Vec::new)
                    .push(name.clone());
            } else {
                // Категоризация конфигурационных типов
                let subcategory = if name.starts_with("Справочники")
                    || name.starts_with("СправочникСсылка")
                    || name.starts_with("СправочникОбъект")
                {
                    "Справочники"
                } else if name.starts_with("Документы")
                    || name.starts_with("ДокументСсылка")
                    || name.starts_with("ДокументОбъект")
                {
                    "Документы"
                } else if name.starts_with("Перечисления") {
                    "Перечисления"
                } else if name.starts_with("Регистры") || name.contains("РегистрСведений")
                {
                    "Регистры"
                } else if name.starts_with("Отчеты") {
                    "Отчеты"
                } else if name.starts_with("Обработки") {
                    "Обработки"
                } else {
                    "Прочие объекты конфигурации"
                };

                config_subcategories
                    .entry(subcategory.to_string())
                    .or_insert_with(Vec::new)
                    .push(name.clone());
            }
        }

        // Добавляем все типы в корневые категории (для общего счетчика)
        platform_root.types = platform_subcategories.values().flatten().cloned().collect();
        configuration_root.types = config_subcategories.values().flatten().cloned().collect();

        // Создаем подкатегории для платформенных типов и добавляем их в корневую категорию
        for (subcategory_name, types) in platform_subcategories {
            let mut subcategory = super::services::TypeCategory {
                id: format!(
                    "platform_{}",
                    subcategory_name
                        .to_lowercase()
                        .replace(" ", "_")
                        .replace("и", "i")
                ),
                name: subcategory_name.clone(),
                description: format!("Платформенные типы категории {}", subcategory_name),
                types: types.clone(),
                subcategories: Vec::new(),
            };

            // Создаем вложенные подкатегории для "Универсальные коллекции"
            if subcategory_name == "Универсальные коллекции" {
                subcategory.subcategories = create_collections_subcategories(&types);
            }

            platform_root.subcategories.push(subcategory);
        }

        // Создаем подкатегории для конфигурационных типов и добавляем их в корневую категорию
        for (subcategory_name, types) in config_subcategories {
            configuration_root
                .subcategories
                .push(super::services::TypeCategory {
                    id: format!(
                        "config_{}",
                        subcategory_name
                            .to_lowercase()
                            .replace(" ", "_")
                            .replace("и", "i")
                    ),
                    name: subcategory_name.clone(),
                    description: format!("Конфигурационные типы категории {}", subcategory_name),
                    types,
                    subcategories: Vec::new(),
                });
        }

        // Добавляем только корневые категории в иерархию
        hierarchy.categories.push(platform_root);
        hierarchy.categories.push(configuration_root);

        hierarchy.total_types = all_types.len();
        Ok(hierarchy)
    }

    pub async fn build_type_hierarchy(&self) -> Result<TypeHierarchy> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let platform_globals = self.resolution_service.get_all_platform_globals();
        self.build_type_hierarchy_with_types(&platform_globals)
            .await
    }

    /// Расширенный поиск типов
    pub async fn advanced_search(
        &self,
        _query: &str,
        _filters: SearchFilters,
    ) -> Result<SearchResults> {
        // TODO: Implement advanced search
        Ok(SearchResults::default())
    }

    /// Получить метрики производительности
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            average_response_time: 0.0,
            cache_hit_rate: 0.0,
            active_connections: 0,
        }
    }

    /// Получить статистику типов для dashboard
    pub async fn get_type_statistics(&self) -> Result<TypeStatistics> {
        // TODO: Implement statistics collection
        Ok(TypeStatistics::default())
    }
}

/// Создает подкатегории для универсальных коллекций
fn create_collections_subcategories(types: &[String]) -> Vec<super::services::TypeCategory> {
    let mut arrays_and_lists = Vec::new();
    let mut tables_and_structures = Vec::new();
    let mut maps_and_associations = Vec::new();
    let mut specialized_collections = Vec::new();

    for type_name in types {
        if type_name.contains("Массив")
            || type_name.contains("Array")
            || type_name.contains("Список")
            || type_name.contains("List")
        {
            arrays_and_lists.push(type_name.clone());
        } else if type_name.contains("Таблица")
            || type_name.contains("Table")
            || type_name.contains("Структура")
            || type_name.contains("Structure")
        {
            tables_and_structures.push(type_name.clone());
        } else if type_name.contains("Соответствие") || type_name.contains("Map") {
            maps_and_associations.push(type_name.clone());
        } else if type_name.contains("Дерево")
            || type_name.contains("Tree")
            || type_name.contains("XDTO")
            || type_name.contains("Коллекция")
        {
            specialized_collections.push(type_name.clone());
        } else {
            // Остальные попадают в общие коллекции
            specialized_collections.push(type_name.clone());
        }
    }

    let mut subcategories = Vec::new();

    if !arrays_and_lists.is_empty() {
        subcategories.push(super::services::TypeCategory {
            id: "collections_arrays".to_string(),
            name: "Массивы и списки".to_string(),
            description: "Линейные коллекции элементов".to_string(),
            types: arrays_and_lists,
            subcategories: Vec::new(),
        });
    }

    if !tables_and_structures.is_empty() {
        subcategories.push(super::services::TypeCategory {
            id: "collections_tables".to_string(),
            name: "Таблицы и структуры".to_string(),
            description: "Структурированные данные с именованными полями".to_string(),
            types: tables_and_structures,
            subcategories: Vec::new(),
        });
    }

    if !maps_and_associations.is_empty() {
        subcategories.push(super::services::TypeCategory {
            id: "collections_maps".to_string(),
            name: "Ассоциативные коллекции".to_string(),
            description: "Коллекции типа ключ-значение".to_string(),
            types: maps_and_associations,
            subcategories: Vec::new(),
        });
    }

    if !specialized_collections.is_empty() {
        subcategories.push(super::services::TypeCategory {
            id: "collections_specialized".to_string(),
            name: "Специализированные коллекции".to_string(),
            description: "Специальные типы коллекций (деревья, XDTO и др.)".to_string(),
            types: specialized_collections,
            subcategories: Vec::new(),
        });
    }

    subcategories
}
