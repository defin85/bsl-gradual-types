//! Metadata types for 1C configuration objects
//!
//! This module contains types for working with 1C configuration metadata:
//! - `MetadataKind`: Catalog, Document, Register, etc.
//! - `ConfigurationType`: Configuration object type with facet
//! - `TabularRowType`: Row type for tabular sections
//! - `Attribute`, `TabularSection`: Object attributes and tabular sections

use serde::{Deserialize, Serialize};

use super::facets::FacetKind;
use super::raw_data::RawAttributeData;

/// Kind of 1C metadata object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MetadataKind {
    #[default]
    Unknown,
    Catalog,
    Document,
    Register,
    Report,
    DataProcessor,
    Enum,
    ChartOfAccounts,
    ChartOfCharacteristicTypes,
    ChartOfCalculationTypes,
    // Registers (added for config_parser.rs)
    InformationRegister,
    AccumulationRegister,
    AccountingRegister,
    CalculationRegister,
    // Business processes and tasks
    BusinessProcess,
    Task,
    // Other configuration types
    ExchangePlan,
    Constant,
    CommonModule,
    Role,
    Subsystem,
    Language,
}

impl MetadataKind {
    /// Converts XML tag of metadata object to MetadataKind
    pub fn from_xml_tag(tag: &str) -> Option<Self> {
        match tag {
            "Catalog" => Some(MetadataKind::Catalog),
            "Document" => Some(MetadataKind::Document),
            "Enum" => Some(MetadataKind::Enum),
            "Report" => Some(MetadataKind::Report),
            "DataProcessor" => Some(MetadataKind::DataProcessor),
            "ChartOfAccounts" => Some(MetadataKind::ChartOfAccounts),
            "ChartOfCharacteristicTypes" => Some(MetadataKind::ChartOfCharacteristicTypes),
            "ChartOfCalculationTypes" => Some(MetadataKind::ChartOfCalculationTypes),
            "InformationRegister" => Some(MetadataKind::InformationRegister),
            "AccumulationRegister" => Some(MetadataKind::AccumulationRegister),
            "AccountingRegister" => Some(MetadataKind::AccountingRegister),
            "CalculationRegister" => Some(MetadataKind::CalculationRegister),
            "BusinessProcess" => Some(MetadataKind::BusinessProcess),
            "Task" => Some(MetadataKind::Task),
            "ExchangePlan" => Some(MetadataKind::ExchangePlan),
            "Constant" => Some(MetadataKind::Constant),
            "CommonModule" => Some(MetadataKind::CommonModule),
            "Role" => Some(MetadataKind::Role),
            "Subsystem" => Some(MetadataKind::Subsystem),
            "Language" => Some(MetadataKind::Language),
            _ => None,
        }
    }

    /// Returns the prefix for configuration type (plural form)
    ///
    /// # Examples
    /// ```
    /// use bsl_shared::domain::types::MetadataKind;
    ///
    /// assert_eq!(MetadataKind::Catalog.to_prefix(), "Справочники");
    /// assert_eq!(MetadataKind::Document.to_prefix(), "Документы");
    /// ```
    pub fn to_prefix(&self) -> &'static str {
        match self {
            MetadataKind::Unknown => "Неизвестный",
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::Register => "Регистры",
            MetadataKind::Report => "Отчеты",
            MetadataKind::DataProcessor => "Обработки",
            MetadataKind::Enum => "Перечисления",
            MetadataKind::ChartOfAccounts => "ПланыСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
            MetadataKind::ChartOfCalculationTypes => "ПланыВидовРасчета",
            MetadataKind::InformationRegister => "РегистрыСведений",
            MetadataKind::AccumulationRegister => "РегистрыНакопления",
            MetadataKind::AccountingRegister => "РегистрыБухгалтерии",
            MetadataKind::CalculationRegister => "РегистрыРасчета",
            MetadataKind::BusinessProcess => "БизнесПроцессы",
            MetadataKind::Task => "Задачи",
            MetadataKind::ExchangePlan => "ПланыОбмена",
            MetadataKind::Constant => "Константы",
            MetadataKind::CommonModule => "ОбщиеМодули",
            MetadataKind::Role => "Роли",
            MetadataKind::Subsystem => "Подсистемы",
            MetadataKind::Language => "Языки",
        }
    }

    /// Returns the Russian name of metadata kind (singular form)
    ///
    /// Used for error messages and hover.
    /// Unlike `to_prefix()` which returns plural form for collections,
    /// this method returns singular form for specific objects.
    ///
    /// # Examples
    /// ```
    /// use bsl_shared::domain::types::MetadataKind;
    ///
    /// assert_eq!(MetadataKind::Catalog.to_russian_name(), "Справочник");
    /// assert_eq!(MetadataKind::Document.to_russian_name(), "Документ");
    /// assert_eq!(MetadataKind::InformationRegister.to_russian_name(), "Регистр сведений");
    /// ```
    pub fn to_russian_name(&self) -> &'static str {
        match self {
            MetadataKind::Unknown => "Объект метаданных",
            MetadataKind::Catalog => "Справочник",
            MetadataKind::Document => "Документ",
            MetadataKind::Enum => "Перечисление",
            MetadataKind::Report => "Отчет",
            MetadataKind::DataProcessor => "Обработка",
            MetadataKind::InformationRegister => "Регистр сведений",
            MetadataKind::AccumulationRegister => "Регистр накопления",
            MetadataKind::AccountingRegister => "Регистр бухгалтерии",
            MetadataKind::CalculationRegister => "Регистр расчета",
            MetadataKind::ChartOfAccounts => "План счетов",
            MetadataKind::ChartOfCharacteristicTypes => "План видов характеристик",
            MetadataKind::ChartOfCalculationTypes => "План видов расчета",
            MetadataKind::BusinessProcess => "Бизнес-процесс",
            MetadataKind::Task => "Задача",
            MetadataKind::ExchangePlan => "План обмена",
            MetadataKind::Constant => "Константа",
            MetadataKind::CommonModule => "Общий модуль",
            MetadataKind::Role => "Роль",
            MetadataKind::Subsystem => "Подсистема",
            MetadataKind::Language => "Язык",
            MetadataKind::Register => "Регистр",
        }
    }

    /// Returns the display name (same as to_prefix for compatibility)
    pub fn display_name(&self) -> &'static str {
        self.to_prefix()
    }

    /// Returns type prefix with facet consideration
    ///
    /// # Examples
    /// - Catalog + Manager -> "СправочникМенеджер"
    /// - Catalog + Object -> "СправочникОбъект"
    /// - Document + Reference -> "ДокументСсылка"
    pub fn faceted_type_prefix(&self, facet: &FacetKind) -> String {
        let base = match self {
            MetadataKind::Catalog => "Справочник",
            MetadataKind::Document => "Документ",
            MetadataKind::Enum => "Перечисление",
            MetadataKind::Report => "Отчет",
            MetadataKind::DataProcessor => "Обработка",
            MetadataKind::ChartOfAccounts => "ПланСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланВидовХарактеристик",
            MetadataKind::ChartOfCalculationTypes => "ПланВидовРасчета",
            MetadataKind::InformationRegister => "РегистрСведений",
            MetadataKind::AccumulationRegister => "РегистрНакопления",
            MetadataKind::AccountingRegister => "РегистрБухгалтерии",
            MetadataKind::CalculationRegister => "РегистрРасчета",
            MetadataKind::BusinessProcess => "БизнесПроцесс",
            MetadataKind::Task => "Задача",
            MetadataKind::ExchangePlan => "ПланОбмена",
            MetadataKind::Constant => "Константа",
            // For types without facets return display_name
            _ => return self.display_name().to_string(),
        };

        let suffix = facet.platform_suffix();
        format!("{}{}", base, suffix)
    }
}

/// Configuration type with metadata kind, name, and facet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationType {
    /// Kind of metadata (Catalog, Document, etc.)
    pub kind: MetadataKind,
    /// Name of the configuration object
    pub name: String,
    /// Active facet (Manager, Object, Reference, Selection, List)
    /// Determines which representation of the type is active
    pub facet: Option<FacetKind>,
    /// Object attributes
    pub attributes: Vec<Attribute>,
    /// Tabular sections
    pub tabular_sections: Vec<TabularSection>,
}

/// Attribute of a configuration object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name
    pub name: String,
    /// Attribute type
    pub type_: String,
    /// Whether attribute has composite type
    pub is_composite: bool,
    /// List of possible types for composite attribute
    pub types: Vec<String>,
}

/// Tabular section of a configuration object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularSection {
    /// Tabular section name
    pub name: String,
    /// Synonym (display name)
    pub synonym: Option<String>,
    /// Attributes of the tabular section
    pub attributes: Vec<Attribute>,
}

/// Type for tabular section row
///
/// # Examples
/// - `СтрокаРаботы` for `Документы.ЗаказНаряды.Работы`
/// - `СтрокаСторон` for `Документы.ДоговорКонтрагента.Стороны`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularRowType {
    /// Full name of parent type (e.g., "Документы.ЗаказНаряды")
    pub parent_type: String,
    /// Tabular section name (e.g., "Работы")
    pub tabular_section_name: String,
    /// Attributes of the tabular section row
    pub attributes: Vec<RawAttributeData>,
}

impl TabularRowType {
    /// Creates a new tabular row type
    pub fn new(
        parent_type: String,
        section_name: String,
        attributes: Vec<RawAttributeData>,
    ) -> Self {
        Self {
            parent_type,
            tabular_section_name: section_name,
            attributes,
        }
    }

    /// Returns the full type name (e.g., "СтрокаРаботы")
    pub fn get_full_name(&self) -> String {
        format!("Строка{}", self.tabular_section_name)
    }

    /// Returns attribute name by index
    pub fn get_attribute_name(&self, index: usize) -> Option<&str> {
        self.attributes.get(index).map(|attr| attr.name.as_str())
    }

    /// Returns attribute type by name
    pub fn get_attribute_type(&self, name: &str) -> Option<&String> {
        self.attributes
            .iter()
            .find(|attr| attr.name == name)
            .map(|attr| &attr.attr_type)
    }
}
