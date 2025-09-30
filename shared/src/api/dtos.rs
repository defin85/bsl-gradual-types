// shared/src/api/dtos.rs

//! Data Transfer Objects (DTOs) for the public API.
//! These structures define the contract between the core analysis engine and any consumer (backend, frontend, etc.).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The main structure representing the complete analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResultDto {
    pub types: Vec<TypeDto>,
    pub categories: HashMap<String, CategoryDto>,
    pub metrics: MetricsDto,
    pub connections: Vec<ConnectionDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationDto>,
}

/// Detailed information about a single type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDto {
    pub id: String,
    pub name: String,
    pub category: String,
    pub certainty: u8,
    pub certainty_text: String,
    pub facets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methods_count: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes_count: Option<usize>,
    pub source: String,
    pub flow_sensitive: bool,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub union_types: Option<Vec<UnionComponentDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_analysis: Option<FlowAnalysisDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<TypeConnectionsDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// A component of a union type with its probability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnionComponentDto {
    #[serde(rename = "type")]
    pub type_name: String,
    pub probability: u8,
}

/// Represents the state of a type through flow analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAnalysisDto {
    pub init: String,
    pub check: String,
    #[serde(rename = "final")]
    pub final_state: String, // 'final' is a reserved keyword
}

/// Represents incoming and outgoing connections for a type node in a graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeConnectionsDto {
    pub incoming: usize,
    pub outgoing: usize,
}

/// Visual and statistical information about a type category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub color: String,
    pub icon: String,
    pub count: usize,
}

/// General metrics about the analysis process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsDto {
    pub total_types: usize,
    pub certainty_high: usize,
    pub certainty_medium: usize,
    pub certainty_low: usize,
    pub flow_sensitive: usize,
    pub cache_hit_rate: String,
    pub analysis_speed: String,
}

/// Represents a single connection between two types in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDto {
    pub source: String, // id of the source type
    pub target: String, // id of the target type
    #[serde(rename = "type")]
    pub connection_type: String,
}

/// Pagination information for paged responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationDto {
    pub current_page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_prev: bool,
    pub has_next: bool,
}