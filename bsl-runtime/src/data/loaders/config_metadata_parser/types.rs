//! Типы данных для универсального парсера метаданных

use super::form_types::FormMetadata;
use bsl_shared::domain::types::{FacetKind, MetadataKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Информация об атрибуте объекта метаданных
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub name: String,
    pub type_name: String,
    pub synonym: Option<String>,
}

/// Информация о табличной части объекта метаданных
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// StandardAttributes applied-object (например, Ref/DeletionMark/Date/Number/Posted)
    pub standard_attributes: Vec<String>,

    /// Табличные части объекта (для документов, справочников)
    pub tabular_sections: Vec<TabularSectionInfo>,

    /// Значения перечисления (для Enum)
    pub enum_values: Vec<String>,

    /// Имена предопределённых элементов (из Ext/Predefined.xml)
    pub predefined_items: Vec<String>,

    /// Дополнительные свойства объекта, извлечённые из XML
    pub properties: HashMap<String, String>,

    /// Контексты выполнения (для общих модулей)
    pub execution_contexts: Vec<ExecutionContext>,

    /// Свойства общего модуля (если объект - CommonModule)
    pub common_module_properties: Option<CommonModuleProperties>,

    /// Формы объекта метаданных (для справочников, документов и т.д.)
    pub forms: Vec<FormMetadata>,

    /// Канонический путь к XML-файлу объекта метаданных
    pub metadata_xml_path: Option<PathBuf>,

    /// Пути к модулям объекта (ObjectModule, ManagerModule, RecordSetModule)
    pub object_module_path: Option<PathBuf>,
    pub manager_module_path: Option<PathBuf>,
    pub record_set_module_path: Option<PathBuf>,

    /// Путь к модулю общего модуля (CommonModule)
    pub common_module_path: Option<PathBuf>,
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
            standard_attributes: Vec::new(),
            tabular_sections: Vec::new(),
            enum_values: Vec::new(),
            predefined_items: Vec::new(),
            properties: HashMap::new(),
            execution_contexts: Vec::new(),
            common_module_properties: None,
            forms: Vec::new(),
            metadata_xml_path: None,
            object_module_path: None,
            manager_module_path: None,
            record_set_module_path: None,
            common_module_path: None,
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
            Some(MetadataKind::Unknown) | None => vec![], // Неизвестный тип - пустой список фасетов
        }
    }
}

/// Контекст выполнения кода в 1С
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ExecutionContext {
    /// Серверный контекст (выполняется на сервере)
    Server,
    /// Контекст управляемого клиентского приложения
    ClientManaged,
    /// Контекст обычного клиентского приложения
    ClientOrdinary,
    /// Контекст внешнего соединения
    ExternalConnection,
}

/// Режим повторного использования возвращаемых значений
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReturnValuesReuse {
    /// Не использовать повторно
    DontUse,
    /// Повторно использовать в течение запроса
    DuringRequest,
    /// Повторно использовать в течение сеанса
    DuringSession,
}

/// Свойства общего модуля (CommonModule)
///
/// Определяет контекстные свойства общего модуля:
/// - В каких контекстах доступен модуль (Server, Client, External Connection)
/// - Является ли модуль глобальным (процедуры/функции доступны без префикса)
/// - Является ли модуль привилегированным
/// - Режим повторного использования возвращаемых значений
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonModuleProperties {
    /// Выполняется на сервере
    pub server: bool,
    /// Доступен в управляемом клиентском приложении
    pub client_managed_application: bool,
    /// Доступен в обычном клиентском приложении
    pub client_ordinary_application: bool,
    /// Доступен во внешнем соединении
    pub external_connection: bool,
    /// Серверный вызов с клиента (ServerCall)
    pub server_call: bool,
    /// Глобальный модуль (процедуры/функции доступны без префикса)
    pub global: bool,
    /// Привилегированный модуль
    pub privileged: bool,
    /// Режим компиляции (Compile)
    pub compile: bool,
    /// Режим повторного использования возвращаемых значений
    pub return_values_reuse: ReturnValuesReuse,
}

impl Default for CommonModuleProperties {
    fn default() -> Self {
        Self {
            server: false,
            client_managed_application: false,
            client_ordinary_application: false,
            external_connection: false,
            server_call: false,
            global: false,
            privileged: false,
            compile: true,
            return_values_reuse: ReturnValuesReuse::DontUse,
        }
    }
}

impl CommonModuleProperties {
    /// Получить список контекстов выполнения на основе свойств модуля
    ///
    /// Возвращает вектор контекстов, в которых доступен данный модуль
    pub fn get_execution_contexts(&self) -> Vec<ExecutionContext> {
        let mut contexts = Vec::new();

        if self.server {
            contexts.push(ExecutionContext::Server);
        }
        if self.client_managed_application {
            contexts.push(ExecutionContext::ClientManaged);
        }
        if self.client_ordinary_application {
            contexts.push(ExecutionContext::ClientOrdinary);
        }
        if self.external_connection {
            contexts.push(ExecutionContext::ExternalConnection);
        }

        contexts
    }
}

/// Тип конфигурации 1С
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigurationType {
    /// Основная конфигурация (базовая)
    Base,
    /// Расширение конфигурации
    Extension,
}

/// Информация о найденной конфигурации
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Режим совместимости платформы (CompatibilityMode)
    pub compatibility_mode: Option<String>,
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
            compatibility_mode: None,
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
            compatibility_mode: None,
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
