//! Типы данных для парсинга форм конфигурации 1С

use super::types::ExecutionContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Реквизит формы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormAttribute {
    /// Имя реквизита (например, "Объект", "РеквизитФормы1")
    pub name: String,

    /// ID реквизита из XML (для сопоставления с элементами управления)
    pub id: u32,

    /// Описание типа реквизита
    pub type_description: TypeDescription,

    /// Является ли основным реквизитом формы (MainAttribute)
    pub is_main_attribute: bool,

    /// Сохраняется ли в базу данных (SavedData)
    pub saved_data: bool,
}

/// Описание типа реквизита формы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDescription {
    /// Список типов реквизита
    /// Примеры: ["cfg:DocumentObject.ЗаказНаряды"], ["String"], ["Number", "String"]
    pub types: Vec<String>,
}

/// Событие формы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormEvent {
    /// Имя события (например, "OnCreateAtServer", "OnOpen")
    pub name: String,

    /// Имя обработчика события в модуле формы
    pub handler_name: String,
}

/// Метаданные формы объекта конфигурации
///
/// Содержит информацию о форме:
/// - Реквизиты формы (attributes)
/// - События формы (events)
/// - Путь к модулю формы (module_path)
/// - Контексты выполнения модуля (execution_contexts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormMetadata {
    /// Имя формы (например, "ФормаДокумента", "ФормаЭлемента")
    pub name: String,

    /// Тип владельца формы (например, "Document.ЗаказНаряды", "Catalog.Номенклатура")
    pub owner_type: String,

    /// Реквизиты формы
    pub attributes: Vec<FormAttribute>,

    /// События формы и их обработчики
    pub events: Vec<FormEvent>,

    /// Путь к модулю формы (Module.bsl), если есть
    pub module_path: Option<PathBuf>,

    /// Контексты выполнения кода в модуле формы
    pub execution_contexts: Vec<ExecutionContext>,
}
