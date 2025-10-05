//! Структуры данных для парсера синтакс-помощника 1С

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use bsl_shared::domain::types::FacetKind;

// ============================================================================
// Структуры данных
// ============================================================================

/// Узел в иерархии синтакс-помощника
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
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
    /// Глобальная функция (например "Мин", "Макс")
    GlobalFunction(GlobalFunctionInfo),
}

/// Информация о категории типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub name: String,
    pub catalog_path: String,
    pub description: String,
    pub related_links: Vec<String>,
    pub types: Vec<String>,
}

/// Полная информация о типе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub identity: TypeIdentity,
    pub documentation: TypeDocumentation,
    pub structure: TypeStructure,
    pub metadata: TypeMetadata,
}

/// Идентификация типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeIdentity {
    pub russian_name: String,
    pub english_name: String,
    pub catalog_path: String,
    pub aliases: Vec<String>,
    pub category_path: String,
}

/// Документация типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDocumentation {
    pub category_description: Option<String>,
    pub type_description: String,
    pub examples: Vec<CodeExample>,
    pub availability: Vec<String>,
    pub since_version: String,
}

/// Двуязычное имя (русское, английское)
pub type BilingualName = (String, String);

/// Структура типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStructure {
    pub collection_element: Option<String>,
    /// Методы с двуязычными именами: (русское, английское)
    pub methods: Vec<BilingualName>,
    /// Свойства с двуязычными именами: (русское, английское)
    pub properties: Vec<BilingualName>,
    pub constructors: Vec<String>,
    pub iterable: bool,
    pub indexable: bool,
    /// Enum values для платформенных перечислений (например "Авто (Auto)", "НеИспользовать (DontUse)")
    pub enum_values: Vec<String>,
}

/// Метаданные типа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMetadata {
    pub available_facets: Vec<FacetKind>,
    pub default_facet: Option<FacetKind>,
    pub serializable: bool,
    pub exchangeable: bool,
    pub xdto_namespace: Option<String>,
    pub xdto_type: Option<String>,
}

/// Пример кода
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub description: Option<String>,
    pub code: String,
    pub language: String,
}

/// Информация о методе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_description: Option<String>,
}

/// Информация о свойстве
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    pub property_type: Option<String>,
    pub is_readonly: bool,
    pub description: Option<String>,
}

/// Информация о конструкторе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub description: Option<String>,
}

/// Информация о параметре
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_name: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Информация о глобальной функции
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalFunctionInfo {
    pub name: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_description: Option<String>,
    pub polymorphic: bool,
    pub pure: bool,
    pub contexts: Vec<String>,    // Where available (Server, Client, etc.)
    pub category: Option<String>, // Category name like "Прочие процедуры и функции"
}

/// База данных синтакс-помощника
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyntaxHelperDatabase {
    pub nodes: HashMap<String, SyntaxNode>,
    pub methods: HashMap<String, MethodInfo>,
    pub properties: HashMap<String, PropertyInfo>,
    pub categories: HashMap<String, CategoryInfo>,
}

/// Индексы для поиска типов
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeIndex {
    pub by_russian: HashMap<String, String>,
    pub by_english: HashMap<String, String>,
    pub by_any_name: HashMap<String, Vec<String>>,
    pub by_category: HashMap<String, Vec<String>>,
    pub by_facet: HashMap<FacetKind, Vec<String>>,
}

/// Настройки оптимизации
#[derive(Debug, Clone)]
pub struct OptimizationSettings {
    /// Максимальное количество потоков
    pub max_threads: Option<usize>,
    /// Размер батча для параллельной обработки
    pub batch_size: usize,
    /// Показывать прогресс-бар
    pub show_progress: bool,
    /// Лимит файлов для обработки (для тестирования)
    pub file_limit: Option<usize>,
    /// Пропускать определённые каталоги
    pub skip_dirs: Vec<String>,
    /// Использовать параллельное построение индексов
    pub parallel_indexing: bool,
}

impl Default for OptimizationSettings {
    fn default() -> Self {
        Self {
            max_threads: None, // Использовать все доступные ядра
            batch_size: 50,    // Оптимальный размер батча
            show_progress: true,
            file_limit: None,
            skip_dirs: vec![
                "tables".to_string(), // Большие таблицы можно пропустить
                "IndexPackLookup".to_string(),
            ],
            parallel_indexing: true,
        }
    }
}
