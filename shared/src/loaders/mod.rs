//! Data loaders and parsers.
//!
//! NOTE: Этот модуль будет постепенно удален в процессе рефакторинга.
//! Infrastructure компоненты переносятся в backend/src/data/

// TEMPORARY: Stub module для config_parser
// TODO Phase 1.2: Удалить после обновления converters.rs в Phase 1.3
pub mod config_parser_guided_discovery {
    use std::path::PathBuf;
    use crate::domain::types::MetadataKind;

    #[derive(Debug, Clone)]
    pub struct DiscoveredMetadata {
        pub name: String,
        pub kind: MetadataKind,
        pub qualified_name: String,
        pub file_path: PathBuf,
        pub reference_source: ReferenceSource,
        pub synonym: Option<String>,
        pub uuid: Option<String>,
        pub attributes: Vec<AttributeInfo>,
        pub tabular_sections: Vec<TabularSectionInfo>,
    }

    #[derive(Debug, Clone)]
    pub enum ReferenceSource {
        ConfigurationChildObjects,
        DirectoryDiscovery,
    }

    #[derive(Debug, Clone)]
    pub struct AttributeInfo {
        pub name: String,
        pub type_definition: String,
        pub synonym: Option<String>,
        pub mandatory: bool,
    }

    #[derive(Debug, Clone)]
    pub struct TabularSectionInfo {
        pub name: String,
        pub synonym: Option<String>,
        pub attributes: Vec<AttributeInfo>,
    }
}

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

// TEMPORARY: Stub functions для converters (Phase 1.3)
// TODO Phase 3: Удалить после переноса логики в SystemCoordinator
use crate::domain::types::{RawTypeData, TypeResolution};

pub fn convert_syntax_helper_to_raw(db: &syntax_helper_parser::SyntaxHelperDatabase) -> Vec<RawTypeData> {
    // STUB: Реальная логика в backend/src/data/adapters/converters.rs
    db.nodes.values().map(|_| RawTypeData::default()).collect()
}

pub fn convert_discovered_metadata_to_raw(metadata: &[config_parser_guided_discovery::DiscoveredMetadata]) -> Vec<RawTypeData> {
    // STUB: Реальная логика в backend/src/data/adapters/converters.rs
    metadata.iter().map(|_| RawTypeData::default()).collect()
}

pub fn convert_resolutions_to_raw(_resolutions: &[TypeResolution]) -> Vec<RawTypeData> {
    // STUB
    vec![]
}

// TEMPORARY: Re-export stub типов для обратной совместимости
// TODO Phase 1.1: Удалить после Phase 3
pub use syntax_helper_parser::SyntaxHelperParser;
pub use syntax_helper_parser::SyntaxHelperDatabase;

// DEPRECATED: SyntaxHelperParser перемещен в backend/src/data/loaders/
// Используйте: use crate::data::loaders::SyntaxHelperParser;