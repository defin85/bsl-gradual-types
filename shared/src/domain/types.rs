//! Core type definitions for the gradual type system

use serde::{Deserialize, Serialize};
use crate::loaders::syntax_helper_parser::GlobalFunctionInfo;

// --- RawTypeData and its components ---
// This structure is designed to hold all information from all parsers.

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawTypeData {
    pub name: String,
    pub english_name: String,
    pub description: String,
    pub category: String,
    pub source: RawDataSource,
    pub methods: Vec<RawMethodData>,
    pub properties: Vec<RawPropertyData>,
    pub facets: Vec<FacetKind>,
    pub kind: Option<MetadataKind>,
    pub attributes: Vec<RawAttributeData>,
    pub tabular_sections: Vec<RawTabularSectionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RawDataSource {
    Platform,
    Configuration,
    UserDefined,
}

impl Default for RawDataSource {
    fn default() -> Self { RawDataSource::Platform }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawMethodData {
    pub name: String,
    pub english_name: String,
    pub return_type: String,
    pub params: Vec<RawParamData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawPropertyData {
    pub name: String,
    pub prop_type: String,
    pub is_readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawParamData {
    pub name: String,
    pub param_type: String,
    pub is_optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawAttributeData {
    pub name: String,
    pub attr_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTabularSectionData {
    pub name: String,
    pub attributes: Vec<RawAttributeData>,
}

// --- Core Abstractions (Restored from previous version) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacetKind {
    Manager, Object, Reference, Metadata, Constructor, Collection, Singleton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataKind {
    Catalog, Document, Register, Report, DataProcessor, Enum,
    ChartOfAccounts, ChartOfCharacteristicTypes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeResolution {
    pub certainty: Certainty,
    pub result: ResolutionResult,
    pub source: ResolutionSource,
    pub metadata: ResolutionMetadata,
    pub active_facet: Option<FacetKind>,
    pub available_facets: Vec<FacetKind>,
}

impl TypeResolution {
    pub fn unknown() -> Self {
        Self {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    pub fn known(concrete: ConcreteType) -> Self {
        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(concrete),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Certainty {
    Known,
    Inferred(f32),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolutionResult {
    Concrete(ConcreteType),
    Union(Vec<WeightedType>),
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedType {
    pub type_: ConcreteType,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConcreteType {
    Platform(PlatformType),
    Configuration(ConfigurationType),
    Primitive(PrimitiveType),
    Special(SpecialType),
    GlobalFunction(GlobalFunctionInfo), // FIXED
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationType {
    pub kind: MetadataKind,
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub tabular_sections: Vec<TabularSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub type_: String,
    pub is_composite: bool,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularSection {
    pub name: String,
    pub synonym: Option<String>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    String, Number, Boolean, Date,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialType {
    Undefined, Null, Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionSource {
    Static, Inferred, Annotated, Runtime, Predicted,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResolutionMetadata {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub notes: Vec<String>,
}

// --- Analysis-related structures ---

#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    pub symbol_table: std::collections::HashMap<String, TypeResolution>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature{}

// === DISPLAY IMPLEMENTATIONS ===

use std::fmt;

impl fmt::Display for ConcreteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConcreteType::Platform(platform) => write!(f, "{}", platform.name),
            ConcreteType::Configuration(config) => {
                write!(f, "{}.{}", config.kind.display_name(), config.name)
            }
            ConcreteType::Primitive(primitive) => write!(f, "{}", primitive.display_name()),
            ConcreteType::Special(special) => write!(f, "{}", special.display_name()),
            ConcreteType::GlobalFunction(func) => write!(f, "{}()", func.name),
        }
    }
}

impl PrimitiveType {
    pub fn display_name(&self) -> &'static str {
        match self {
            PrimitiveType::String => "Строка",
            PrimitiveType::Number => "Число",
            PrimitiveType::Boolean => "Булево",
            PrimitiveType::Date => "Дата",
        }
    }
}

impl SpecialType {
    pub fn display_name(&self) -> &'static str {
        match self {
            SpecialType::Undefined => "Неопределено",
            SpecialType::Null => "Null",
            SpecialType::Type => "Тип",
        }
    }
}

impl MetadataKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::Enum => "Перечисления",
            MetadataKind::Report => "Отчеты",
            MetadataKind::DataProcessor => "Обработки",
            MetadataKind::Register => "Регистры",
            MetadataKind::ChartOfAccounts => "ПланыСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
        }
    }
}