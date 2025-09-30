//! Data loaders and parsers.
//!
//! NOTE: Этот модуль будет постепенно удален в процессе рефакторинга.
//! Infrastructure компоненты переносятся в backend/src/data/

pub mod config_parser_guided_discovery;
pub mod converters;

// TEMPORARY: Stub module для обратной совместимости
// TODO Phase 1.1: Удалить после обновления всех импортов
pub mod syntax_helper_parser {
    // Re-export основных типов, которые используются в Domain Layer
    // Эти типы будут переопределены локально в Domain или удалены

    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct ParameterInfo {
        pub name: String,
        pub type_name: Option<String>,
        pub is_optional: bool,
        pub default_value: Option<String>,
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct GlobalFunctionInfo {
        pub name: String,
        pub english_name: Option<String>,
        pub description: Option<String>,
        pub parameters: Vec<ParameterInfo>,
        pub return_type: Option<String>,
        pub return_description: Option<String>,
        pub polymorphic: bool,
        pub pure: bool,
        pub contexts: Vec<String>,
        pub category: Option<String>,
    }

    // Stub типы для временной совместимости
    use crate::domain::types::FacetKind;

    #[derive(Debug, Clone)]
    pub struct TypeInfo {
        pub identity: TypeIdentity,
        pub documentation: TypeDocumentation,
        pub structure: TypeStructure,
        pub metadata: TypeMetadata,
    }

    #[derive(Debug, Clone)]
    pub struct TypeIdentity {
        pub russian_name: String,
        pub english_name: String,
        pub catalog_path: String,
        pub category_path: String,
        pub aliases: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub struct TypeDocumentation {
        pub category_description: Option<String>,
        pub type_description: String,
        pub examples: Vec<CodeExample>,
        pub availability: Vec<String>,
        pub since_version: String,
    }

    #[derive(Debug, Clone)]
    pub struct TypeStructure {
        pub collection_element: Option<String>,
        pub methods: Vec<String>,
        pub properties: Vec<String>,
        pub constructors: Vec<String>,
        pub iterable: bool,
        pub indexable: bool,
    }

    #[derive(Debug, Clone)]
    pub struct TypeMetadata {
        pub available_facets: Vec<FacetKind>,
        pub default_facet: Option<FacetKind>,
        pub serializable: bool,
        pub exchangeable: bool,
        pub xdto_namespace: Option<String>,
        pub xdto_type: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub struct CodeExample {
        pub description: Option<String>,
        pub code: String,
        pub language: String,
    }

    #[derive(Debug, Clone)]
    pub enum SyntaxNode {
        Type(TypeInfo),
    }

    #[derive(Debug, Clone, Default)]
    pub struct SyntaxHelperDatabase {
        pub nodes: HashMap<String, SyntaxNode>,
    }

    pub struct SyntaxHelperParser;

    impl SyntaxHelperParser {
        pub fn new() -> Self {
            SyntaxHelperParser
        }

        pub fn parse_syntax_helper<P: AsRef<std::path::Path>>(&mut self, _path: P) -> anyhow::Result<()> {
            // STUB: Реальная логика в backend/src/data/loaders/syntax_helper_parser.rs
            Ok(())
        }

        pub fn export_database(&self) -> SyntaxHelperDatabase {
            SyntaxHelperDatabase::default()
        }
    }
}

// Re-export key components (для обратной совместимости)
pub use config_parser_guided_discovery::ConfigurationGuidedParser;
pub use converters::*;

// TEMPORARY: Re-export stub типов для обратной совместимости
// TODO Phase 1.1: Удалить после Phase 3
pub use syntax_helper_parser::SyntaxHelperParser;
pub use syntax_helper_parser::SyntaxHelperDatabase;

// DEPRECATED: SyntaxHelperParser перемещен в backend/src/data/loaders/
// Используйте: use crate::data::loaders::SyntaxHelperParser;