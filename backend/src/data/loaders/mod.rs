//! Data Layer: Loaders
//!
//! Infrastructure компоненты для загрузки данных из внешних источников

pub mod config_metadata_parser;
pub mod platform_types;
pub mod progress;
pub mod syntax_helper;
pub mod syntax_helper_parser;

// Re-exports для удобства использования
pub use config_metadata_parser::{
    ConfigurationDiscovery, UniversalMetadataObject, UniversalMetadataParser,
};
pub use syntax_helper_parser::SyntaxHelperParser;

pub use syntax_helper::{
    CategoryInfo, CodeExample, ConstructorInfo, GlobalFunctionInfo, MethodInfo,
    OptimizationSettings, ParameterInfo, PropertyInfo, SyntaxHelperDatabase, SyntaxNode,
    TypeDocumentation, TypeIdentity, TypeInfo, TypeMetadata, TypeStructure,
};

pub use platform_types::{
    create_tabular_section_type, load_all_platform_types,
    populate_signature_index_from_platform_types,
};
