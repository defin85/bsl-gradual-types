//! Data Layer: Loaders
//!
//! Infrastructure компоненты для загрузки данных из внешних источников

pub mod config_metadata_parser;
pub mod hbk_recovery;
pub mod platform_types;
pub mod progress;
pub mod signature_sources;
pub mod syntax_helper;
pub mod syntax_helper_parser;

// Re-exports для удобства использования
pub use config_metadata_parser::{
    ConfigurationDiscovery, UniversalMetadataObject, UniversalMetadataParser,
};
pub use syntax_helper_parser::{ParsingStats, SyntaxHelperParser};

pub use syntax_helper::{
    CategoryInfo, CodeExample, ConstructorInfo, GlobalFunctionInfo, MethodInfo,
    OptimizationSettings, ParameterInfo, PropertyInfo, SyntaxHelperDatabase, SyntaxNode,
    TypeDocumentation, TypeIdentity, TypeInfo, TypeMetadata, TypeStructure,
};

pub use platform_types::{apply_generic_info_to_repository, get_generic_info_registry};

pub use signature_sources::SyntaxHelperSource;
