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

/// Информация о типе (совместимо с backend API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub id: String,
    pub name: String,
    pub category: String, // "Platform", "Configuration", etc.
    pub certainty: u32, // 0-100
    #[serde(rename = "certaintyText")]
    pub certainty_text: String, // "Known 100%", "Inferred 85%", etc.
    pub facets: Vec<String>,
    pub source: String,
    #[serde(rename = "flowSensitive")]
    pub flow_sensitive: bool,
    pub description: String,
}

/// Вспомогательная структура для обратной совместимости
impl TypeInfo {
    pub fn display_name(&self) -> &str {
        &self.name
    }

    pub fn get_category(&self) -> TypeCategory {
        match self.category.as_str() {
            "Platform" => TypeCategory::Platform,
            "Configuration" => TypeCategory::Configuration,
            "Union" => TypeCategory::Union,
            "Dynamic" => TypeCategory::Dynamic,
            _ => TypeCategory::Platform,
        }
    }

    pub fn get_certainty(&self) -> Certainty {
        if self.certainty == 100 {
            Certainty::Known
        } else if self.certainty == 0 {
            Certainty::Unknown
        } else {
            Certainty::Inferred(self.certainty as f32 / 100.0)
        }
    }

    pub fn is_flow_sensitive(&self) -> bool {
        self.flow_sensitive
    }

    pub fn get_facets(&self) -> Vec<FacetKind> {
        self.facets.iter().filter_map(|f| {
            match f.as_str() {
                "Manager" => Some(FacetKind::Manager),
                "Object" => Some(FacetKind::Object),
                "Reference" => Some(FacetKind::Reference),
                "Collection" => Some(FacetKind::Collection),
                "Metadata" => Some(FacetKind::Metadata),
                _ => None,
            }
        }).collect()
    }
}

/// Метрики системы типизации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMetrics {
    pub total_types: u32,
    pub known_types: u32,
    pub inferred_types: u32,
    pub unknown_types: u32,
    #[serde(default)]
    pub flow_sensitive_types: u32,
    #[serde(default)]
    pub cache_hit_rate: f32,
    #[serde(default)]
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
    pub page: usize,
    pub page_size: usize,
}

impl TypeFilters {
    pub fn new() -> Self {
        Self {
            search_query: None,
            category: None,
            certainty_level: None,
            facet: None,
            flow_sensitive_only: false,
            page: 1,
            page_size: 50,
        }
    }

    pub fn offset(&self) -> usize {
        (self.page - 1) * self.page_size
    }
}

/// Результат поиска типов (совместимо с backend API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSearchResult {
    pub types: Vec<TypeInfo>,
    pub categories: std::collections::HashMap<String, CategoryInfo>,
    pub metrics: TypeSummaryMetrics,
    pub connections: Vec<TypeConnection>,
    pub pagination: Option<PaginationInfo>,
}

/// Информация о пагинации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub current_page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new(current_page: usize, page_size: usize, total_items: usize) -> Self {
        let total_pages = (total_items + page_size - 1) / page_size;
        Self {
            current_page,
            page_size,
            total_items,
            total_pages,
            has_next: current_page < total_pages,
            has_prev: current_page > 1,
        }
    }
}

/// Информация о категории типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub color: String,
    pub icon: String,
    pub count: u32,
}

/// Сводные метрики для списка типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSummaryMetrics {
    #[serde(rename = "totalTypes")]
    pub total_types: u32,
    #[serde(rename = "certaintyHigh")]
    pub certainty_high: u32,
    #[serde(rename = "certaintyMedium")]
    pub certainty_medium: u32,
    #[serde(rename = "certaintyLow")]
    pub certainty_low: u32,
    #[serde(rename = "flowSensitive")]
    pub flow_sensitive: u32,
    #[serde(rename = "cacheHitRate")]
    pub cache_hit_rate: String,
    #[serde(rename = "analysisSpeed")]
    pub analysis_speed: String,
}

/// Вспомогательные методы для TypeSearchResult
impl TypeSearchResult {
    pub fn total_count(&self) -> u32 {
        self.metrics.total_types
    }

    pub fn filtered_count(&self) -> u32 {
        self.types.len() as u32
    }
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TypeGraph {
    pub nodes: Vec<TypeGraphNode>,
    pub connections: Vec<TypeConnection>,
}
