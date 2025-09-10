//! API types for BSL Type System

use serde::{Deserialize, Serialize};

/// Уровень уверенности в типе
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Certainty {
    Known,
    Inferred(f32), // 0.0-1.0
    Unknown,
}

impl Certainty {
    pub fn as_percentage(&self) -> String {
        match self {
            Certainty::Known => "100%".to_string(),
            Certainty::Inferred(value) => format!("{:.0}%", value * 100.0),
            Certainty::Unknown => "0%".to_string(),
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Certainty::Known => "#28a745",
            Certainty::Inferred(_) => "#ffc107",
            Certainty::Unknown => "#dc3545",
        }
    }
}

/// Фасеты 1C объектов
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FacetKind {
    Manager,
    Object,
    Reference,
    Collection,
    Metadata,
}

impl FacetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FacetKind::Manager => "Manager",
            FacetKind::Object => "Object",
            FacetKind::Reference => "Reference",
            FacetKind::Collection => "Collection",
            FacetKind::Metadata => "Metadata",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            FacetKind::Manager => "#007bff",
            FacetKind::Object => "#28a745",
            FacetKind::Reference => "#ffc107",
            FacetKind::Collection => "#17a2b8",
            FacetKind::Metadata => "#6f42c1",
        }
    }
}

/// Категория типа
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TypeCategory {
    Platform,
    Configuration,
    Union,
    Dynamic,
}

impl TypeCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TypeCategory::Platform => "Platform",
            TypeCategory::Configuration => "Configuration",
            TypeCategory::Union => "Union",
            TypeCategory::Dynamic => "Dynamic",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            TypeCategory::Platform => "#007bff",
            TypeCategory::Configuration => "#28a745",
            TypeCategory::Union => "#ffc107",
            TypeCategory::Dynamic => "#dc3545",
        }
    }
}

/// Взвешенный тип для Union Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedType {
    pub type_name: String,
    pub weight: f32, // 0.0-1.0
}

/// Информация о типе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub display_name: String,
    pub category: TypeCategory,
    pub certainty: Certainty,
    pub facets: Vec<FacetKind>,
    pub active_facet: Option<FacetKind>,
    pub union_types: Option<Vec<WeightedType>>,
    pub is_flow_sensitive: bool,
    pub source: String,
    pub methods_count: Option<u32>,
    pub properties_count: Option<u32>,
    pub description: Option<String>,
}

/// Метрики системы типизации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMetrics {
    pub total_types: u32,
    pub known_types: u32,
    pub inferred_types: u32,
    pub unknown_types: u32,
    pub flow_sensitive_types: u32,
    pub cache_hit_rate: f32,
    pub analysis_speed_ms: f32,
}

/// Фильтры для поиска типов
#[derive(Debug, Clone, Default)]
pub struct TypeFilters {
    pub search_query: Option<String>,
    pub category: Option<TypeCategory>,
    pub certainty_level: Option<String>,
    pub facet: Option<FacetKind>,
    pub flow_sensitive_only: bool,
}

/// Результат поиска типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSearchResult {
    pub types: Vec<TypeInfo>,
    pub total_count: u32,
    pub filtered_count: u32,
}

/// Узел графа типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeGraphNode {
    pub id: String,
    pub type_info: TypeInfo,
    pub x: f32,
    pub y: f32,
    pub connections: Vec<String>, // IDs связанных узлов
}

/// Связь между типами в графе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeConnection {
    pub from: String,
    pub to: String,
    pub connection_type: ConnectionType,
    pub label: Option<String>,
}

/// Тип связи между типами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Dependency,
    Inheritance,
    Composition,
    FlowTransition,
    Reference,
}

impl ConnectionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionType::Dependency => "dependency",
            ConnectionType::Inheritance => "inheritance",
            ConnectionType::Composition => "composition",
            ConnectionType::FlowTransition => "flow",
            ConnectionType::Reference => "reference",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            ConnectionType::Dependency => "rgba(255,255,255,0.3)",
            ConnectionType::Inheritance => "#007bff",
            ConnectionType::Composition => "#28a745",
            ConnectionType::FlowTransition => "#f59e0b",
            ConnectionType::Reference => "#6f42c1",
        }
    }
}

/// Граф типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeGraph {
    pub nodes: Vec<TypeGraphNode>,
    pub connections: Vec<TypeConnection>,
}
