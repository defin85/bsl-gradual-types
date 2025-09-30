//! Data Layer: Loaders
//!
//! Infrastructure компоненты для загрузки данных из внешних источников

pub mod syntax_helper_parser;

// Re-exports для удобства использования
pub use syntax_helper_parser::{
    SyntaxHelperParser,
    SyntaxHelperDatabase,
    SyntaxNode,
    TypeInfo,
    CategoryInfo,
    MethodInfo,
    PropertyInfo,
    ConstructorInfo,
    GlobalFunctionInfo,
    ParameterInfo,
    TypeIdentity,
    TypeDocumentation,
    TypeStructure,
    TypeMetadata,
    CodeExample,
    OptimizationSettings,
    ParsingStats,
};