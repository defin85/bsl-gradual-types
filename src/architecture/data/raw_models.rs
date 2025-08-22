use crate::core::types::Facet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TypeSource {
    Platform { version: String },
    Configuration { config_version: String },
    UserDefined { file_path: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawPropertyData {
    pub name: String,
    pub type_name: String,
    pub is_readonly: bool,
    pub description: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawParameterData {
    pub name: String,
    pub type_name: String,
    pub description: String,
    pub is_optional: bool,
    pub is_by_value: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawMethodData {
    pub name: String,
    pub documentation: String,
    pub parameters: Vec<RawParameterData>,
    pub return_type: Option<String>,
    pub return_type_name: Option<String>,
    pub params: Vec<RawParameterData>,
    pub is_function: bool,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawTypeData {
    pub id: String,
    pub russian_name: String,
    pub english_name: String,
    pub source: TypeSource,
    pub category_path: Vec<String>,
    pub methods: Vec<RawMethodData>,
    pub properties: Vec<RawPropertyData>,
    pub documentation: String,
    pub examples: Vec<String>,
    pub available_facets: Vec<Facet>,
    pub parse_metadata: ParseMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParseMetadata {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
}
