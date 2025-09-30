//! Data layer (flat structure)
//!
//! Data access and repository implementations

use serde::{Deserialize, Serialize};

// Подключаем загрузчики данных
pub mod loaders;
pub mod adapters;


/// Source of type information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
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
    #[default]
    Builtin,
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

// Re-export essential types for data layer access (simplified)
pub use bsl_shared::domain::types::RawTypeData;

// TODO: После упрощения архитектуры добавить сюда реальные репозитории