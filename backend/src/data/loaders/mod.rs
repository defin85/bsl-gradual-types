//! Data Layer: Loaders
//!
//! Infrastructure компоненты для загрузки данных из внешних источников

pub mod config_metadata_parser;
pub mod config_bsl_modules;
pub mod hbk_recovery;
pub mod platform_types;
pub mod progress;
pub mod signature_sources;
pub mod syntax_helper;

// Re-exports для удобства использования
pub use config_metadata_parser::{
    ConfigurationDiscovery, UniversalMetadataObject, UniversalMetadataParser,
};

pub use config_bsl_modules::{
    index_configuration_bsl_modules, index_configuration_bsl_modules_with_progress,
    index_configuration_bsl_modules_with_progress_parallel, IndexedConfigSignatures,
    ModuleIndexProgress, ModuleParseComparison, ModuleParseStats, SinglePassMode,
    compare_module_parsing_from_file, compare_module_parsing_from_file_with_progress,
    compare_module_parsing_from_file_with_progress_mode,
    single_pass_module_stats_from_file_with_progress_mode,
};

// Новые реэкспорты из syntax_helper
pub use syntax_helper::{ParsingStats, SyntaxHelperLoader};

/// Deprecated: используйте SyntaxHelperLoader вместо SyntaxHelperParser
#[deprecated(since = "0.5.0", note = "Use SyntaxHelperLoader instead")]
pub type SyntaxHelperParser = SyntaxHelperLoader;

pub use syntax_helper::{
    CategoryInfo, CodeExample, ConstructorInfo, GlobalFunctionInfo, MethodInfo,
    OptimizationSettings, ParameterInfo, PropertyInfo, SyntaxHelperDatabase, SyntaxNode,
    TypeDocumentation, TypeIdentity, TypeInfo, TypeMetadata, TypeStructure,
};

pub use platform_types::{apply_generic_info_to_repository, get_generic_info_registry};

pub use signature_sources::SyntaxHelperSource;
