//! Universal Configuration Metadata Parser
//!
//! Универсальный парсер метаданных конфигурации 1С.
//! Обрабатывает ЛЮБОЙ тип объекта метаданных из Configuration.xml и ChildObjects.
//!
//! ## Архитектура
//!
//! - `types.rs` - UniversalMetadataObject структура
//! - `discovery.rs` - ConfigurationDiscovery для обнаружения объектов
//! - `parser.rs` - UniversalMetadataParser для парсинга XML
//! - `converter.rs` - Конвертация в RawTypeData для TypeRepository

pub mod converter;
pub mod discovery;
pub mod parser;
pub mod types;

// Re-export публичного API
pub use discovery::ConfigurationDiscovery;
pub use parser::UniversalMetadataParser;
pub use types::{
    AttributeInfo, ConfigurationInfo, ConfigurationType, TabularSectionInfo,
    UniversalMetadataObject,
};
