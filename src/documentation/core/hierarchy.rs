//! Иерархическая модель документации типов

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;

use crate::core::types::{TypeResolution, FacetKind};
use crate::core::types::ConfigurationType as ConfigurationObjectType;
// Типы провайдеров будут определены ниже

/// Полная иерархия типов документации
#[derive(Debug, Clone, Serialize)]
pub struct TypeHierarchy {
    /// Корневые категории
    pub root_categories: Vec<CategoryNode>,
    
    /// Статистика иерархии
    pub statistics: HierarchyStatistics,
    
    /// Быстрые индексы для навигации
    pub navigation_index: NavigationIndex,
    
    /// Метаданные иерархии
    pub metadata: HierarchyMetadata,
}

/// Узел в иерархии документации
#[derive(Debug, Clone, Serialize)]
pub enum DocumentationNode {
    /// Корневая категория (Платформа, Конфигурация, etc.)
    RootCategory(RootCategoryNode),
    
    /// Подкатегория (Универсальные коллекции, Справочники, etc.)
    SubCategory(SubCategoryNode),
    
    /// Платформенный тип (ТаблицаЗначений, Массив, etc.)
    PlatformType(PlatformTypeNode),
    
    /// Конфигурационный тип (Справочники.Контрагенты, etc.)
    ConfigurationType(ConfigurationTypeNode),
    
    /// Метод типа
    Method(MethodNode),
    
    /// Свойство типа
    Property(PropertyNode),
    
    /// Глобальная функция
    GlobalFunction(GlobalFunctionNode),
    
    /// Перечисление
    Enumeration(EnumerationNode),
    
    /// Пользовательский модуль
    UserModule(UserModuleNode),
}

/// Корневая категория
#[derive(Debug, Clone, Serialize)]
pub struct RootCategoryNode {
    /// Уникальный идентификатор
    pub id: String,
    
    /// Название категории
    pub name: String,
    
    /// Описание категории
    pub description: String,
    
    /// Дочерние узлы
    pub children: Vec<DocumentationNode>,
    
    /// UI метаданные
    pub ui_metadata: UiMetadata,
    
    /// Статистика категории
    pub statistics: CategoryStatistics,
}

/// Подкатегория
#[derive(Debug, Clone, Serialize)]
pub struct SubCategoryNode {
    /// Уникальный идентификатор
    pub id: String,
    
    /// Название подкатегории
    pub name: String,
    
    /// Описание
    pub description: String,
    
    /// Путь в иерархии
    pub hierarchy_path: Vec<String>,
    
    /// Дочерние узлы
    pub children: Vec<DocumentationNode>,
    
    /// UI метаданные
    pub ui_metadata: UiMetadata,
    
    /// Статистика подкатегории
    pub statistics: CategoryStatistics,
}

/// Полная документация платформенного типа
#[derive(Debug, Clone, Serialize)]
pub struct PlatformTypeNode {
    /// Базовая информация о типе
    pub base_info: TypeDocumentationFull,
    
    /// Специфичная информация платформенного типа
    pub platform_specific: PlatformTypeSpecific,
}

/// Специфичная информация платформенного типа
#[derive(Debug, Clone, Serialize)]
pub struct PlatformTypeSpecific {
    /// Версия платформы, в которой появился
    pub since_version: String,
    
    /// Доступность (клиент/сервер/мобильное приложение)
    pub availability: Vec<AvailabilityContext>,
    
    /// XDTO информация
    pub xdto_info: Option<XdtoInfo>,
    
    /// Сериализуемость
    pub serializable: bool,
    
    /// Возможность обмена с сервером
    pub exchangeable: bool,
}

/// Конфигурационный тип
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationTypeNode {
    /// Базовая информация о типе
    pub base_info: TypeDocumentationFull,
    
    /// Специфичная информация конфигурационного типа
    pub configuration_specific: ConfigurationTypeSpecific,
}

/// Специфичная информация конфигурационного типа
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationTypeSpecific {
    /// Тип объекта конфигурации
    pub object_type: ConfigurationObjectType,
    
    /// Реквизиты объекта
    pub attributes: Vec<AttributeDocumentation>,
    
    /// Табличные части
    pub tabular_sections: Vec<TabularSectionDocumentation>,
    
    /// Формы объекта
    pub forms: Vec<FormDocumentation>,
    
    /// Права доступа
    pub access_rights: Vec<AccessRight>,
    
    /// Связи с другими объектами
    pub object_relations: Vec<ObjectRelation>,
}

/// Документация реквизита
#[derive(Debug, Clone, Serialize)]
pub struct AttributeDocumentation {
    /// Имя реквизита
    pub name: String,
    
    /// Синоним
    pub synonym: String,
    
    /// Комментарий
    pub comment: Option<String>,
    
    /// Тип данных
    pub data_type: String,
    
    /// Разрешение типа
    pub type_resolution: TypeResolution,
    
    /// Обязательность заполнения
    pub mandatory: bool,
    
    /// Индексируется
    pub indexed: bool,
}

/// Документация табличной части
#[derive(Debug, Clone, Serialize)]
pub struct TabularSectionDocumentation {
    /// Имя табличной части
    pub name: String,
    
    /// Синоним
    pub synonym: String,
    
    /// Комментарий
    pub comment: Option<String>,
    
    /// Реквизиты табличной части
    pub attributes: Vec<AttributeDocumentation>,
    
    /// Разрешение типа (коллекция строк)
    pub type_resolution: TypeResolution,
}

/// Узел метода
#[derive(Debug, Clone, Serialize)]
pub struct MethodNode {
    /// Имя метода
    pub name: String,
    
    /// Русское название
    pub russian_name: String,
    
    /// Английское название
    pub english_name: String,
    
    /// Описание метода
    pub description: String,
    
    /// Параметры
    pub parameters: Vec<ParameterDocumentation>,
    
    /// Возвращаемый тип
    pub return_type: Option<TypeResolution>,
    
    /// Примеры использования
    pub examples: Vec<CodeExample>,
    
    /// Доступность
    pub availability: Vec<AvailabilityContext>,
    
    /// UI метаданные
    pub ui_metadata: UiMetadata,
}

/// Узел свойства
#[derive(Debug, Clone, Serialize)]
pub struct PropertyNode {
    /// Имя свойства
    pub name: String,
    
    /// Русское название
    pub russian_name: String,
    
    /// Английское название
    pub english_name: String,
    
    /// Описание свойства
    pub description: String,
    
    /// Тип свойства
    pub property_type: TypeResolution,
    
    /// Только для чтения
    pub readonly: bool,
    
    /// Примеры использования
    pub examples: Vec<CodeExample>,
    
    /// UI метаданные
    pub ui_metadata: UiMetadata,
}

/// Полная документация о типе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDocumentationFull {
    // === ИДЕНТИФИКАЦИЯ ===
    /// Уникальный идентификатор
    pub id: String,
    
    /// Русское название
    pub russian_name: String,
    
    /// Английское название  
    pub english_name: String,
    
    /// Альтернативные имена
    pub aliases: Vec<String>,
    
    // === КЛАССИФИКАЦИЯ ===
    /// Тип источника
    pub source_type: DocumentationSourceType,
    
    /// Путь в иерархии
    pub hierarchy_path: Vec<String>,
    
    // === ГРАДУАЛЬНАЯ ТИПИЗАЦИЯ ===
    /// Информация о типе из системы
    pub type_resolution: TypeResolution,
    
    /// Доступные фасеты
    pub available_facets: Vec<FacetKind>,
    
    /// Активный фасет
    pub active_facet: Option<FacetKind>,
    
    // === СТРУКТУРА ===
    /// Методы типа
    pub methods: Vec<MethodDocumentation>,
    
    /// Свойства типа
    pub properties: Vec<PropertyDocumentation>,
    
    /// Конструкторы
    pub constructors: Vec<ConstructorDocumentation>,
    
    // === ДОКУМЕНТАЦИЯ ===
    /// Описание
    pub description: String,
    
    /// Примеры использования
    pub examples: Vec<CodeExample>,
    
    /// Доступность (клиент/сервер)
    pub availability: Vec<AvailabilityContext>,
    
    /// Версия появления
    pub since_version: String,
    
    /// Замечания и ограничения
    pub notes: Vec<String>,
    
    // === СВЯЗИ ===
    /// Связанные типы
    pub related_types: Vec<TypeReference>,
    
    /// Родительский тип
    pub parent_type: Option<TypeReference>,
    
    /// Дочерние типы
    pub child_types: Vec<TypeReference>,
    
    // === МЕТАДАННЫЕ ===
    /// Путь к исходному файлу
    pub source_file: Option<String>,
    
    /// UI метаданные
    pub ui_metadata: UiMetadata,
}

/// Тип источника документации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentationSourceType {
    /// Платформенный тип из справки
    Platform { version: String },
    
    /// Конфигурационный объект
    Configuration { object_type: ConfigurationObjectType },
    
    /// Пользовательский тип
    UserDefined { module_path: String },
    
    /// Глобальная функция
    GlobalFunction,
}

/// UI метаданные для отображения
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiMetadata {
    /// Иконка для отображения
    pub icon: String,
    
    /// Цвет категории  
    pub color: String,
    
    /// Путь в дереве
    pub tree_path: Vec<String>,
    
    /// Развернуто ли в UI
    pub expanded: bool,
    
    /// Сортировочный вес
    pub sort_weight: i32,
    
    /// CSS классы
    pub css_classes: Vec<String>,
}

/// Статистика иерархии
#[derive(Debug, Clone, Serialize)]
pub struct HierarchyStatistics {
    /// Всего узлов в иерархии
    pub total_nodes: usize,
    
    /// Количество по типам узлов
    pub node_counts: HashMap<String, usize>,
    
    /// Глубина иерархии
    pub max_depth: usize,
    
    /// Время построения иерархии (мс)
    pub build_time_ms: u64,
}

/// Статистика категории
#[derive(Debug, Clone, Serialize)]
pub struct CategoryStatistics {
    /// Количество дочерних типов
    pub child_types_count: usize,
    
    /// Количество методов во всех типах
    pub total_methods_count: usize,
    
    /// Количество свойств во всех типах
    pub total_properties_count: usize,
    
    /// Самый популярный тип в категории
    pub most_popular_type: Option<String>,
}

/// Индекс для быстрой навигации
#[derive(Debug, Clone, Serialize)]
pub struct NavigationIndex {
    /// Индекс по ID → путь в иерархии
    pub by_id: HashMap<String, Vec<String>>,
    
    /// Индекс по русскому имени
    pub by_russian_name: HashMap<String, String>,
    
    /// Индекс по английскому имени  
    pub by_english_name: HashMap<String, String>,
    
    /// Индекс по фасетам
    pub by_facet: HashMap<FacetKind, Vec<String>>,
    
    /// Обратный индекс для связей
    pub reverse_relations: HashMap<String, Vec<String>>,
}

/// Метаданные иерархии
#[derive(Debug, Clone, Serialize)]
pub struct HierarchyMetadata {
    /// Версия схемы иерархии
    pub schema_version: String,
    
    /// Время создания
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Источники данных
    pub data_sources: Vec<DataSourceInfo>,
    
    /// Конфигурация построения
    pub build_config: BuildConfig,
}

/// Информация об источнике данных
#[derive(Debug, Clone, Serialize)]
pub struct DataSourceInfo {
    /// Тип источника
    pub source_type: String,
    
    /// Путь к источнику
    pub source_path: String,
    
    /// Время последнего обновления
    pub last_modified: chrono::DateTime<chrono::Utc>,
    
    /// Чек-сумма для отслеживания изменений
    pub checksum: String,
}

/// Конфигурация построения иерархии
#[derive(Debug, Clone, Serialize)]
pub struct BuildConfig {
    /// Включить платформенные типы
    pub include_platform_types: bool,
    
    /// Включить конфигурационные типы
    pub include_configuration_types: bool,
    
    /// Включить пользовательские модули
    pub include_user_modules: bool,
    
    /// Максимальная глубина иерархии
    pub max_hierarchy_depth: usize,
    
    /// Фильтры по доступности
    pub availability_filters: Vec<AvailabilityContext>,
}

// Дополнительные типы, используемые в документации

/// Контекст доступности
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AvailabilityContext {
    Client,
    Server,
    ExternalConnection,
    MobileApp,
    MobileServer,
    WebClient,
}

/// Информация о XDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XdtoInfo {
    /// Пространство имен
    pub namespace: String,
    
    /// Имя типа XDTO
    pub type_name: String,
    
    /// Схема
    pub schema_location: Option<String>,
}

/// Ссылка на тип
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeReference {
    /// ID ссылочного типа
    pub type_id: String,
    
    /// Название для отображения
    pub display_name: String,
    
    /// Тип связи
    pub relation_type: RelationType,
    
    /// Описание связи
    pub relation_description: Option<String>,
}

/// Тип связи между типами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    /// Наследование
    Inheritance,
    
    /// Композиция
    Composition,
    
    /// Агрегация
    Aggregation,
    
    /// Использование
    Usage,
    
    /// Ассоциация
    Association,
}

/// Документация метода
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDocumentation {
    /// Имя метода
    pub name: String,
    
    /// Русское название
    pub russian_name: String,
    
    /// Английское название
    pub english_name: String,
    
    /// Описание
    pub description: String,
    
    /// Параметры
    pub parameters: Vec<ParameterDocumentation>,
    
    /// Возвращаемый тип
    pub return_type: Option<TypeResolution>,
    
    /// Примеры использования
    pub examples: Vec<CodeExample>,
    
    /// Доступность
    pub availability: Vec<AvailabilityContext>,
    
    /// Возможные исключения
    pub exceptions: Vec<ExceptionDocumentation>,
}

/// Документация параметра
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDocumentation {
    /// Имя параметра
    pub name: String,
    
    /// Тип параметра
    pub parameter_type: TypeResolution,
    
    /// Описание
    pub description: String,
    
    /// Обязательный параметр
    pub required: bool,
    
    /// Значение по умолчанию
    pub default_value: Option<String>,
}

/// Документация свойства
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDocumentation {
    /// Имя свойства
    pub name: String,
    
    /// Русское название
    pub russian_name: String,
    
    /// Английское название
    pub english_name: String,
    
    /// Тип свойства
    pub property_type: TypeResolution,
    
    /// Описание
    pub description: String,
    
    /// Только для чтения
    pub readonly: bool,
    
    /// Примеры использования
    pub examples: Vec<CodeExample>,
}

/// Документация конструктора
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorDocumentation {
    /// Имя конструктора
    pub name: String,
    
    /// Описание
    pub description: String,
    
    /// Параметры конструктора
    pub parameters: Vec<ParameterDocumentation>,
    
    /// Примеры использования
    pub examples: Vec<CodeExample>,
    
    /// Доступность
    pub availability: Vec<AvailabilityContext>,
}

/// Пример кода
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    /// Описание примера
    pub title: String,
    
    /// Код примера
    pub code: String,
    
    /// Язык (BSL, Query, etc.)
    pub language: String,
    
    /// Ожидаемый результат
    pub expected_output: Option<String>,
    
    /// Исполняемый (можно запустить в браузере)
    pub executable: bool,
}

/// Документация исключения
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionDocumentation {
    /// Тип исключения
    pub exception_type: String,
    
    /// Описание
    pub description: String,
    
    /// Условия возникновения
    pub conditions: Vec<String>,
}

/// Глобальная функция
#[derive(Debug, Clone, Serialize)]
pub struct GlobalFunctionNode {
    /// Базовая документация метода
    pub method_info: MethodDocumentation,
    
    /// Категория глобальной функции
    pub category: GlobalFunctionCategory,
    
    /// Синонимы функции
    pub synonyms: Vec<String>,
}

/// Категория глобальной функции
#[derive(Debug, Clone, Serialize)]
pub enum GlobalFunctionCategory {
    /// Работа со строками
    StringFunctions,
    
    /// Работа с числами
    NumberFunctions,
    
    /// Работа с датами
    DateFunctions,
    
    /// Работа с типами
    TypeFunctions,
    
    /// Системные функции
    SystemFunctions,
    
    /// Пользовательские функции
    UserFunctions,
}

/// Узел перечисления
#[derive(Debug, Clone, Serialize)]
pub struct EnumerationNode {
    /// Базовая информация
    pub base_info: TypeDocumentationFull,
    
    /// Значения перечисления
    pub values: Vec<EnumerationValue>,
}

/// Значение перечисления
#[derive(Debug, Clone, Serialize)]
pub struct EnumerationValue {
    /// Имя значения
    pub name: String,
    
    /// Русское название
    pub russian_name: String,
    
    /// Английское название
    pub english_name: String,
    
    /// Описание
    pub description: String,
    
    /// Числовое значение (если есть)
    pub numeric_value: Option<i64>,
}

/// Пользовательский модуль
#[derive(Debug, Clone, Serialize)]
pub struct UserModuleNode {
    /// Путь к модулю
    pub module_path: String,
    
    /// Имя модуля
    pub module_name: String,
    
    /// Тип модуля
    pub module_type: UserModuleType,
    
    /// Экспортируемые функции/процедуры
    pub exported_functions: Vec<MethodDocumentation>,
    
    /// Экспортируемые переменные
    pub exported_variables: Vec<VariableDocumentation>,
    
    /// Зависимости модуля
    pub dependencies: Vec<String>,
}

/// Тип пользовательского модуля
#[derive(Debug, Clone, Serialize)]
pub enum UserModuleType {
    CommonModule,
    ObjectModule,
    FormModule,
    CommandModule,
}

/// Документация переменной
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDocumentation {
    /// Имя переменной
    pub name: String,
    
    /// Тип переменной
    pub variable_type: TypeResolution,
    
    /// Описание
    pub description: String,
    
    /// Экспортируемая
    pub exported: bool,
}

/// Документация формы
#[derive(Debug, Clone, Serialize)]
pub struct FormDocumentation {
    /// Имя формы
    pub name: String,
    
    /// Назначение формы
    pub purpose: FormPurpose,
    
    /// Элементы формы
    pub form_elements: Vec<FormElementDocumentation>,
    
    /// Команды формы
    pub commands: Vec<CommandDocumentation>,
}

/// Назначение формы
#[derive(Debug, Clone, Serialize)]
pub enum FormPurpose {
    ObjectForm,
    ListForm,
    ChoiceForm,
    SelectionForm,
    CustomForm,
}

/// Документация элемента формы
#[derive(Debug, Clone, Serialize)]
pub struct FormElementDocumentation {
    /// Имя элемента
    pub name: String,
    
    /// Тип элемента
    pub element_type: String,
    
    /// Связанные данные
    pub data_path: Option<String>,
    
    /// Описание
    pub description: String,
}

/// Документация команды
#[derive(Debug, Clone, Serialize)]
pub struct CommandDocumentation {
    /// Имя команды
    pub name: String,
    
    /// Описание команды
    pub description: String,
    
    /// Параметры команды
    pub parameters: Vec<ParameterDocumentation>,
}

/// Права доступа
#[derive(Debug, Clone, Serialize)]
pub struct AccessRight {
    /// Название права
    pub name: String,
    
    /// Описание права
    pub description: String,
    
    /// Применимо к операциям
    pub operations: Vec<String>,
}

/// Связь между объектами
#[derive(Debug, Clone, Serialize)]
pub struct ObjectRelation {
    /// Тип связи
    pub relation_type: RelationType,
    
    /// Связанный объект
    pub related_object: String,
    
    /// Описание связи
    pub description: String,
    
    /// Поле связи
    pub relation_field: Option<String>,
}

impl TypeHierarchy {
    /// Построить иерархию из провайдеров
    pub async fn build(
        _platform_provider: &crate::documentation::PlatformDocumentationProvider,
        _configuration_provider: &crate::documentation::ConfigurationDocumentationProvider,
    ) -> Result<Self> {
        let start_time = std::time::Instant::now();
        
        let mut root_categories = Vec::new();
        
        // TODO: Добавляем платформенные типы
        // if let Ok(platform_category) = platform_provider.get_root_category().await {
        //     root_categories.push(platform_category);
        // }
        
        // TODO: Добавляем конфигурационные типы
        // if let Ok(config_category) = configuration_provider.get_root_category().await {
        //     root_categories.push(config_category);
        // }
        
        // Строим индексы
        let navigation_index = Self::build_navigation_index(&root_categories);
        
        // Собираем статистику
        let statistics = Self::calculate_statistics(&root_categories);
        
        Ok(Self {
            root_categories,
            statistics,
            navigation_index,
            metadata: HierarchyMetadata {
                schema_version: "1.0.0".to_string(),
                created_at: chrono::Utc::now(),
                data_sources: Vec::new(), // TODO: заполнить
                build_config: BuildConfig::default(),
            },
        })
    }
    
    /// Построить навигационный индекс
    fn build_navigation_index(categories: &[CategoryNode]) -> NavigationIndex {
        // TODO: реализовать построение индексов
        NavigationIndex {
            by_id: HashMap::new(),
            by_russian_name: HashMap::new(),
            by_english_name: HashMap::new(),
            by_facet: HashMap::new(),
            reverse_relations: HashMap::new(),
        }
    }
    
    /// Подсчитать статистику иерархии
    fn calculate_statistics(categories: &[CategoryNode]) -> HierarchyStatistics {
        // TODO: реализовать подсчет статистики
        HierarchyStatistics {
            total_nodes: 0,
            node_counts: HashMap::new(),
            max_depth: 0,
            build_time_ms: 0,
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            include_platform_types: true,
            include_configuration_types: true,
            include_user_modules: true,
            max_hierarchy_depth: 10,
            availability_filters: Vec::new(),
        }
    }
}

// Type aliases для совместимости
pub type CategoryNode = RootCategoryNode;