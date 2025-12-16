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
//! - `form_types.rs` - Структуры данных для форм (FormMetadata, FormAttribute, FormEvent)
//! - `form_parser.rs` - Парсер форм (FormParser)

pub mod converter;
pub mod discovery;
pub mod form_parser;
pub mod form_types;
pub mod parser;
pub mod types;

// Re-export публичного API
pub use discovery::ConfigurationDiscovery;
pub use parser::UniversalMetadataParser;
pub use types::{
    AttributeInfo, ConfigurationInfo, ConfigurationType, ExecutionContext, TabularSectionInfo,
    UniversalMetadataObject,
};
pub use form_parser::FormParser;
pub use form_types::{FormAttribute, FormElementBinding, FormEvent, FormMetadata, TypeDescription};
