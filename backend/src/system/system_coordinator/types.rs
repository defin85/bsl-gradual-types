//! Типы данных для SystemCoordinator

use std::collections::HashMap;
use std::path::PathBuf;

use crate::data::loaders::config_bsl_modules::ModuleSignatureSnapshot;
use crate::data::loaders::config_metadata_parser::UniversalMetadataObject;

/// Результат загрузки метаданных конфигурации
#[derive(Debug, Clone)]
pub struct LoadMetadataResult {
    /// Количество загруженных базовых конфигураций
    pub base_config_count: usize,
    /// Количество загруженных расширений
    pub extensions_count: usize,
    /// Общее количество типов
    pub total_types: usize,
}

/// Ошибки инициализации системы
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("Failed to load platform types: {0}")]
    PlatformTypesError(#[from] anyhow::Error),
    #[error("Cache initialization failed: {0}")]
    CacheError(String),
}

/// Информация о символе для LSP
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: String,
    pub line: u32,
    pub column: u32,
}

/// Уникальный ключ объекта метаданных (по XML-типу и имени)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectKey {
    pub object_type_raw: String,
    pub name: String,
}

impl ObjectKey {
    pub fn new(object_type_raw: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            object_type_raw: object_type_raw.into(),
            name: name.into(),
        }
    }
}

/// Кэш индекса конфигурации для инкрементального обновления
#[derive(Debug, Clone, Default)]
pub struct ConfigIndexCache {
    pub config_root: PathBuf,
    pub child_objects: HashMap<String, Vec<String>>,
    pub metadata_by_key: HashMap<ObjectKey, UniversalMetadataObject>,
    pub object_xml_map: HashMap<PathBuf, ObjectKey>,
    pub form_xml_map: HashMap<PathBuf, ObjectKey>,
    pub module_signatures: HashMap<PathBuf, ModuleSignatureSnapshot>,
}
