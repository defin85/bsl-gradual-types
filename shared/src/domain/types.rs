//! Core type definitions for the gradual type system

use serde::{Deserialize, Serialize};

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
    /// Enum values for platform enumeration types (e.g., "Авто (Auto)", "НеИспользовать (DontUse)")
    pub enum_values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum RawDataSource {
    #[default]
    Platform,
    Configuration,
    UserDefined,
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
    Manager,     // Создание, поиск (СправочникМенеджер)
    Object,      // Изменяемый объект (СправочникОбъект)
    Reference,   // Ссылка на элемент (СправочникСсылка)
    Metadata,    // Метаданные
    Constructor, // Конструктор
    Collection,  // Коллекция
    Singleton,   // Одиночный объект
    Selection,   // Обход элементов (СправочникВыборка) - из статьи Balyuk & Popova
    List,        // Управление списком в форме (СправочникСписок) - из статьи Balyuk & Popova
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

    /// Создать TypeResolution из RawTypeData с сохранением всех метаданных (в т.ч. фасетов)
    pub fn from_raw_type(raw_type: &RawTypeData) -> Self {
        let mut resolution = Self::known(
            ConcreteType::Platform(PlatformType {
                name: raw_type.name.clone(),
            })
        );
        // Копируем фасеты из RawTypeData
        resolution.available_facets = raw_type.facets.clone();
        resolution
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
    Intersection(Vec<ConcreteType>),
    Generic(GenericType),
    Nullable(Box<ConcreteType>),
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedType {
    pub type_: ConcreteType,
    pub weight: f32,
}

/// Generic type with type parameters
/// Examples: Массив<Строка>, Соответствие<Строка, Число>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericType {
    pub base_type: String,
    pub type_params: Vec<ConcreteType>,
}

impl GenericType {
    /// Создать типизированный массив: Массив<T>
    pub fn array(element_type: ConcreteType) -> Self {
        Self {
            base_type: "Массив".to_string(),
            type_params: vec![element_type],
        }
    }

    /// Создать типизированное соответствие: Соответствие<K, V>
    pub fn map(key_type: ConcreteType, value_type: ConcreteType) -> Self {
        Self {
            base_type: "Соответствие".to_string(),
            type_params: vec![key_type, value_type],
        }
    }

    /// Создать типизированный список: Список<T>
    pub fn list(element_type: ConcreteType) -> Self {
        Self {
            base_type: "Список".to_string(),
            type_params: vec![element_type],
        }
    }

    /// Создать типизированную структуру: Структура<...>
    pub fn structure(field_types: Vec<ConcreteType>) -> Self {
        Self {
            base_type: "Структура".to_string(),
            type_params: field_types,
        }
    }

    /// Получить тип элемента для коллекций (первый параметр)
    pub fn element_type(&self) -> Option<&ConcreteType> {
        self.type_params.first()
    }
}

/// Информация о глобальной функции (определена в Domain Layer)
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

/// Информация о параметре функции
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_name: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConcreteType {
    Platform(PlatformType),
    Configuration(ConfigurationType),
    Primitive(PrimitiveType),
    Special(SpecialType),
    GlobalFunction(GlobalFunctionInfo),
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

impl FacetKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            FacetKind::Manager => "Менеджер",
            FacetKind::Object => "Объект",
            FacetKind::Reference => "Ссылка",
            FacetKind::Metadata => "Метаданные",
            FacetKind::Constructor => "Конструктор",
            FacetKind::Collection => "Коллекция",
            FacetKind::Singleton => "Одиночный",
            FacetKind::Selection => "Выборка",
            FacetKind::List => "Список",
        }
    }

    pub fn platform_suffix(&self) -> &'static str {
        match self {
            FacetKind::Manager => "Менеджер",
            FacetKind::Object => "Объект",
            FacetKind::Reference => "Ссылка",
            FacetKind::Selection => "Выборка",
            FacetKind::List => "Список",
            _ => "",
        }
    }
}

// ============================================================================
// Advanced Type System Utilities (Milestone 2.3)
// ============================================================================

impl ResolutionResult {
    /// Normalize Union types: deduplicate, simplify, and sort
    ///
    /// Examples:
    /// - `String | String` → `String`
    /// - `Number | String | Number` → `Number | String`
    /// - `String | Dynamic` → `Dynamic`
    pub fn normalize_union(types: Vec<WeightedType>) -> Self {
        if types.is_empty() {
            return ResolutionResult::Dynamic;
        }

        // 1. Check for Dynamic - if present, return Dynamic
        if types.iter().any(|wt| matches!(wt.type_, ConcreteType::Special(SpecialType::Undefined))) {
            return ResolutionResult::Dynamic;
        }

        // 2. Deduplicate and merge weights
        let mut type_map: std::collections::HashMap<String, (ConcreteType, f32)> = std::collections::HashMap::new();

        for weighted in types {
            let key = format!("{:?}", weighted.type_); // Simple key based on Debug representation
            type_map.entry(key)
                .and_modify(|(_, w)| *w += weighted.weight)
                .or_insert((weighted.type_, weighted.weight));
        }

        // 3. Convert back to Vec and sort by weight (descending)
        let mut normalized: Vec<WeightedType> = type_map
            .into_values()
            .map(|(t, w)| WeightedType { type_: t, weight: w })
            .collect();

        normalized.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

        // 4. If only one type remains, return Concrete
        if normalized.len() == 1 {
            return ResolutionResult::Concrete(normalized.into_iter().next().unwrap().type_);
        }

        ResolutionResult::Union(normalized)
    }

    /// Create an intersection type with validation
    pub fn intersection(types: Vec<ConcreteType>) -> Self {
        if types.is_empty() {
            return ResolutionResult::Dynamic;
        }

        if types.len() == 1 {
            return ResolutionResult::Concrete(types.into_iter().next().unwrap());
        }

        // Deduplicate
        let mut unique_types = Vec::new();
        for t in types {
            if !unique_types.contains(&t) {
                unique_types.push(t);
            }
        }

        if unique_types.len() == 1 {
            return ResolutionResult::Concrete(unique_types.into_iter().next().unwrap());
        }

        ResolutionResult::Intersection(unique_types)
    }

    /// Create a nullable type (T | Null)
    pub fn nullable(base_type: ConcreteType) -> Self {
        ResolutionResult::Nullable(Box::new(base_type))
    }

    /// Check if this result is nullable
    pub fn is_nullable(&self) -> bool {
        match self {
            ResolutionResult::Nullable(_) => true,
            ResolutionResult::Union(types) => {
                types.iter().any(|wt| matches!(wt.type_, ConcreteType::Special(SpecialType::Null)))
            },
            _ => false,
        }
    }

    /// Extract the non-null type from nullable
    pub fn unwrap_nullable(&self) -> Option<&ConcreteType> {
        match self {
            ResolutionResult::Nullable(t) => Some(t),
            _ => None,
        }
    }
}

impl WeightedType {
    /// Create a weighted type with default weight (1.0)
    pub fn new(type_: ConcreteType) -> Self {
        Self { type_, weight: 1.0 }
    }

    /// Create a weighted type with custom weight
    pub fn with_weight(type_: ConcreteType, weight: f32) -> Self {
        Self { type_, weight }
    }
}

impl ConcreteType {
    /// Check if this type is compatible with another for intersection
    pub fn is_intersection_compatible(&self, other: &Self) -> bool {
        // Primitive types cannot be intersected
        if matches!(self, ConcreteType::Primitive(_)) && matches!(other, ConcreteType::Primitive(_)) {
            return false;
        }

        // Special types (Null, Undefined) cannot be intersected with primitives
        if matches!(self, ConcreteType::Special(_)) || matches!(other, ConcreteType::Special(_)) {
            return false;
        }

        // Platform types can be intersected if they share common facets
        true
    }

    /// Create a primitive string type
    pub fn string() -> Self {
        ConcreteType::Primitive(PrimitiveType::String)
    }

    /// Create a primitive number type
    pub fn number() -> Self {
        ConcreteType::Primitive(PrimitiveType::Number)
    }

    /// Create a primitive boolean type
    pub fn boolean() -> Self {
        ConcreteType::Primitive(PrimitiveType::Boolean)
    }

    /// Create a null type
    pub fn null() -> Self {
        ConcreteType::Special(SpecialType::Null)
    }

    /// Create an undefined type
    pub fn undefined() -> Self {
        ConcreteType::Special(SpecialType::Undefined)
    }
}