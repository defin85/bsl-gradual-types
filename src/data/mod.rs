//! Data layer (flat structure)
//!
//! Data access and repository implementations

use serde::{Deserialize, Serialize};

// Подключаем загрузчики данных
pub mod loaders;

/// Source of type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeSource {
    /// Type from 1C platform
    Platform {
        platform_version: String,
    },
    /// Type from configuration metadata
    Configuration {
        config_version: String,
    },
    /// User-defined type in BSL code  
    UserDefined {
        file_path: String,
        line: usize,
    },
    /// Built-in type (primitives, etc.)
    Builtin,
}

impl Default for TypeSource {
    fn default() -> Self {
        TypeSource::Builtin
    }
}

impl TypeSource {
    /// Create platform type source
    pub fn platform(version: String) -> Self {
        TypeSource::Platform {
            platform_version: version,
        }
    }
    
    /// Create configuration type source
    pub fn configuration(version: String) -> Self {
        TypeSource::Configuration {
            config_version: version,
        }
    }
    
    /// Create user-defined type source
    pub fn user_defined(file_path: String, line: usize) -> Self {
        TypeSource::UserDefined { file_path, line }
    }
}

// Re-export search types for data layer access
pub use crate::domain::search::{RawTypeData, RawMethodData, RawPropertyData, RawParameterData, ParseMetadata};

// TODO: После удаления architecture/ добавить сюда реальные репозитории