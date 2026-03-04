//! Местоположение определения типа для Go To Definition
//!
//! НЕ путать с CodeLocation (контекст выполнения кода)!
//! TypeDefinitionLocation указывает где ОПРЕДЕЛЁН тип, а не где используется.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Местоположение определения типа
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeDefinitionLocation {
    /// Платформенный тип — определён в платформе 1С
    /// Нет исходного файла, только ссылка на документацию
    Platform {
        type_name: String,
        /// URI для Syntax Helper документации (опционально)
        docs_uri: Option<String>,
    },

    /// Конфигурационный тип — определён в метаданных конфигурации
    Configuration {
        /// Путь к файлу метаданных (.xml)
        metadata_path: PathBuf,
        /// Пути к модулям (если есть)
        module_paths: ModulePaths,
    },

    /// Пользовательский тип — определён в BSL коде
    UserDefined {
        file_path: PathBuf,
        /// Позиция определения (byte offsets)
        start: u32,
        end: u32,
    },

    /// Примитивный тип — встроен в язык, нет определения
    Primitive,
}

/// Пути к модулям конфигурационного типа
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModulePaths {
    /// Модуль объекта (Ext/ObjectModule.bsl)
    pub object_module: Option<PathBuf>,
    /// Модуль менеджера (Ext/ManagerModule.bsl)
    pub manager_module: Option<PathBuf>,
    /// Модуль набора записей (Ext/RecordSetModule.bsl)
    pub recordset_module: Option<PathBuf>,
}

impl TypeDefinitionLocation {
    /// Создать location для платформенного типа
    pub fn platform(type_name: &str) -> Self {
        Self::Platform {
            type_name: type_name.to_string(),
            docs_uri: Some(format!("bsl://docs/{}", type_name)),
        }
    }

    /// Создать location для платформенного типа с custom docs URI
    pub fn platform_with_docs(type_name: &str, docs_uri: &str) -> Self {
        Self::Platform {
            type_name: type_name.to_string(),
            docs_uri: Some(docs_uri.to_string()),
        }
    }

    /// Создать location для конфигурационного типа
    pub fn configuration(metadata_path: PathBuf) -> Self {
        Self::Configuration {
            metadata_path,
            module_paths: ModulePaths::default(),
        }
    }

    /// Создать location для конфигурационного типа с путями к модулям
    pub fn configuration_with_modules(metadata_path: PathBuf, module_paths: ModulePaths) -> Self {
        Self::Configuration {
            metadata_path,
            module_paths,
        }
    }

    /// Создать location для пользовательского типа
    pub fn user_defined(file_path: PathBuf, start: u32, end: u32) -> Self {
        Self::UserDefined {
            file_path,
            start,
            end: end.max(start),
        }
    }

    /// Создать location для примитива
    pub fn primitive() -> Self {
        Self::Primitive
    }

    /// Получить основной путь для навигации (если есть)
    pub fn primary_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Configuration {
                module_paths,
                metadata_path,
                ..
            } => {
                // Приоритет: object_module > manager_module > metadata_path
                module_paths
                    .object_module
                    .as_ref()
                    .or(module_paths.manager_module.as_ref())
                    .or(Some(metadata_path))
            }
            Self::UserDefined { file_path, .. } => Some(file_path),
            _ => None,
        }
    }

    /// Проверить, есть ли навигируемый путь
    pub fn is_navigable(&self) -> bool {
        match self {
            Self::Configuration { .. } | Self::UserDefined { .. } => true,
            Self::Platform { docs_uri, .. } => docs_uri.is_some(),
            Self::Primitive => false,
        }
    }

    /// Получить docs URI для платформенного типа
    pub fn docs_uri(&self) -> Option<&str> {
        match self {
            Self::Platform { docs_uri, .. } => docs_uri.as_deref(),
            _ => None,
        }
    }
}

impl ModulePaths {
    /// Создать пустые пути
    pub fn new() -> Self {
        Self::default()
    }

    /// Установить путь к модулю объекта
    pub fn with_object_module(mut self, path: PathBuf) -> Self {
        self.object_module = Some(path);
        self
    }

    /// Установить путь к модулю менеджера
    pub fn with_manager_module(mut self, path: PathBuf) -> Self {
        self.manager_module = Some(path);
        self
    }

    /// Установить путь к модулю набора записей
    pub fn with_recordset_module(mut self, path: PathBuf) -> Self {
        self.recordset_module = Some(path);
        self
    }

    /// Проверить наличие хотя бы одного модуля
    pub fn has_any_module(&self) -> bool {
        self.object_module.is_some()
            || self.manager_module.is_some()
            || self.recordset_module.is_some()
    }
}

#[cfg(test)]
#[path = "type_definition_location/tests.rs"]
mod tests;
