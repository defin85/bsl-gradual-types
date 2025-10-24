//! Типы данных для универсального парсера метаданных

use bsl_shared::domain::types::{FacetKind, MetadataKind};
use std::collections::HashMap;
use std::path::PathBuf;

/// Информация об атрибуте объекта метаданных
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    pub name: String,
    pub type_name: String,
    pub synonym: Option<String>,
}

/// Информация о табличной части объекта метаданных
#[derive(Debug, Clone)]
pub struct TabularSectionInfo {
    pub name: String,
    pub synonym: Option<String>,
    pub attributes: Vec<AttributeInfo>,
}

/// Универсальный объект метаданных конфигурации 1С
///
/// Содержит всю информацию о любом типе объекта метаданных:
/// - Справочники, Документы, Регистры
/// - Константы, Перечисления
/// - Планы счетов, видов характеристик, расчета
/// - Общие модули, Роли, Подсистемы и т.д.
#[derive(Debug, Clone)]
pub struct UniversalMetadataObject {
    /// Сырое имя типа из XML (например, "Catalog", "InformationRegister")
    pub object_type_raw: String,

    /// Распознанный тип объекта (Some) или None для неизвестных типов
    pub object_type: Option<MetadataKind>,

    /// UUID объекта метаданных
    pub uuid: String,

    /// Имя объекта (например, "Контрагенты")
    pub name: String,

    /// Синоним объекта (русскоязычное представление)
    pub synonym: Option<String>,

    /// Фасеты, доступные для данного типа объекта
    /// (Manager, Object, Reference, Selection, List для справочников)
    pub facets: Vec<FacetKind>,

    /// Атрибуты объекта (для справочников, документов и т.д.)
    pub attributes: Vec<AttributeInfo>,

    /// Табличные части объекта (для документов, справочников)
    pub tabular_sections: Vec<TabularSectionInfo>,

    /// Дополнительные свойства объекта, извлечённые из XML
    pub properties: HashMap<String, String>,
}

impl UniversalMetadataObject {
    /// Создать новый универсальный объект метаданных
    pub fn new(object_type_raw: String, name: String, uuid: String) -> Self {
        let object_type = MetadataKind::from_xml_tag(&object_type_raw);
        let facets = Self::determine_facets(object_type);

        Self {
            object_type_raw,
            object_type,
            uuid,
            name,
            synonym: None,
            facets,
            attributes: Vec::new(),
            tabular_sections: Vec::new(),
            properties: HashMap::new(),
        }
    }

    /// Определить фасеты на основе типа объекта метаданных
    fn determine_facets(kind: Option<MetadataKind>) -> Vec<FacetKind> {
        match kind {
            Some(MetadataKind::Catalog) => vec![
                FacetKind::Manager,
                FacetKind::Object,
                FacetKind::Reference,
                FacetKind::Selection,
                FacetKind::List,
            ],
            Some(MetadataKind::Document) => vec![
                FacetKind::Manager,
                FacetKind::Object,
                FacetKind::Reference,
                FacetKind::Selection,
                FacetKind::List,
            ],
            Some(MetadataKind::Enum) => vec![FacetKind::Manager, FacetKind::Reference],
            Some(MetadataKind::InformationRegister)
            | Some(MetadataKind::AccumulationRegister)
            | Some(MetadataKind::AccountingRegister)
            | Some(MetadataKind::CalculationRegister) => {
                vec![FacetKind::Manager, FacetKind::Object, FacetKind::Selection]
            }
            Some(MetadataKind::ChartOfAccounts)
            | Some(MetadataKind::ChartOfCharacteristicTypes)
            | Some(MetadataKind::ChartOfCalculationTypes) => vec![
                FacetKind::Manager,
                FacetKind::Object,
                FacetKind::Reference,
                FacetKind::Selection,
            ],
            Some(MetadataKind::Report) | Some(MetadataKind::DataProcessor) => {
                vec![FacetKind::Manager, FacetKind::Object]
            }
            Some(MetadataKind::BusinessProcess) | Some(MetadataKind::Task) => {
                vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference]
            }
            Some(MetadataKind::Constant) => vec![FacetKind::Manager],
            Some(MetadataKind::CommonModule) => vec![FacetKind::Singleton],
            Some(MetadataKind::Role)
            | Some(MetadataKind::Subsystem)
            | Some(MetadataKind::Language)
            | Some(MetadataKind::ExchangePlan) => vec![FacetKind::Metadata],
            Some(MetadataKind::Register) => vec![FacetKind::Manager, FacetKind::Object],
            None => vec![], // Неизвестный тип - пустой список фасетов
        }
    }
}

/// Тип конфигурации 1С
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationType {
    /// Основная конфигурация (базовая)
    Base,
    /// Расширение конфигурации
    Extension,
}

/// Информация о найденной конфигурации
#[derive(Debug, Clone)]
pub struct ConfigurationInfo {
    /// Путь к папке конфигурации (содержит Configuration.xml)
    pub path: PathBuf,

    /// Тип конфигурации (Base или Extension)
    pub config_type: ConfigurationType,

    /// Имя конфигурации (из <Name> тега)
    pub name: String,

    /// Префикс расширения (если есть, например "Тест_")
    /// Для основной конфигурации всегда None
    pub prefix: Option<String>,

    /// UUID конфигурации (для диагностики)
    pub uuid: Option<String>,
}

impl ConfigurationInfo {
    /// Создать информацию об основной конфигурации
    pub fn base(path: PathBuf, name: String) -> Self {
        Self {
            path,
            config_type: ConfigurationType::Base,
            name,
            prefix: None,
            uuid: None,
        }
    }

    /// Создать информацию о расширении
    pub fn extension(path: PathBuf, name: String, prefix: Option<String>) -> Self {
        Self {
            path,
            config_type: ConfigurationType::Extension,
            name,
            prefix,
            uuid: None,
        }
    }

    /// Является ли конфигурация расширением
    pub fn is_extension(&self) -> bool {
        matches!(self.config_type, ConfigurationType::Extension)
    }

    /// Является ли конфигурация базовой
    pub fn is_base(&self) -> bool {
        matches!(self.config_type, ConfigurationType::Base)
    }
}
