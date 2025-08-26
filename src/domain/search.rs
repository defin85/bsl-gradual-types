//! Search types and structures for the domain layer

use crate::domain::types::{TypeResolution, ConcreteType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Advanced search query for type system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchQuery {
    /// Search pattern (can be regex)
    pub pattern: String,
    /// Limit results count
    pub limit: Option<usize>,
    /// Include system/built-in types
    pub include_system: bool,
    /// Include user-defined types
    pub include_user: bool,
    /// Filter by type kind
    pub type_filter: Option<TypeFilter>,
}

/// Filter for search by type kind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeFilter {
    Primitives,
    Objects,
    Functions,
    Enums,
    All,
}

/// Search results containing found types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Found type entries
    pub results: Vec<TypeSearchResult>,
    /// Total count (may be more than results.len() if limited)
    pub total_count: usize,
    /// Whether search was truncated
    pub truncated: bool,
}

/// Individual search result entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSearchResult {
    /// Type name
    pub name: String,
    /// Type resolution information
    pub type_resolution: TypeResolution,
    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f64,
    /// Raw data for the type (if available)
    pub raw_data: Option<RawTypeData>,
    /// Highlighted matches in name
    pub match_highlights: Vec<MatchHighlight>,
}

/// Raw type data for export/detailed view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTypeData {
    /// Type name
    pub name: String,
    /// Source file location
    pub source_location: Option<String>,
    /// Documentation/comments
    pub documentation: Option<String>,
    /// Associated methods/properties
    pub methods: Vec<RawMethodData>,
    /// Associated properties
    pub properties: Vec<RawPropertyData>,
    /// Type metadata
    pub metadata: HashMap<String, String>,
    /// Type source information
    pub source: crate::data::TypeSource,
    /// Russian name
    pub russian_name: String,
    /// English name  
    pub english_name: String,
    /// Category path
    pub category_path: Vec<String>,
}

/// Raw property data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPropertyData {
    /// Property name
    pub name: String,
    /// Property type
    pub type_name: String,
    /// Property description
    pub description: String,
    /// Whether property is read-only
    pub is_read_only: bool,
}

/// Parse metadata for configuration files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseMetadata {
    /// Source file path
    pub file_path: String,
    /// Parse timestamp
    pub parsed_at: String,
    /// Configuration version
    pub config_version: String,
    /// Number of parsed objects
    pub objects_count: usize,
}

/// Raw method/property data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMethodData {
    /// Method name
    pub name: String,
    /// Method signature
    pub signature: String,
    /// Method documentation
    pub documentation: Option<String>,
    /// Return type
    pub return_type: Option<String>,
    /// Method parameters
    pub parameters: Vec<RawParameterData>,
}

/// Raw parameter data  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawParameterData {
    /// Parameter name
    pub name: String,
    /// Parameter type name
    pub type_name: String,
    /// Parameter description
    pub description: String,
    /// Whether parameter is optional
    pub is_optional: bool,
    /// Whether passed by value
    pub is_by_value: bool,
}

/// Highlight match in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchHighlight {
    /// Start position in text
    pub start: usize,
    /// Length of match
    pub length: usize,
}

/// Type hierarchy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeHierarchy {
    /// Root types (no parents)
    pub roots: Vec<TypeHierarchyNode>,
    /// All types mapped by name
    pub types_map: HashMap<String, TypeHierarchyNode>,
}

/// Node in type hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeHierarchyNode {
    /// Type name
    pub name: String,
    /// Type information
    pub type_info: ConcreteType,
    /// Parent types
    pub parents: Vec<String>,
    /// Child types
    pub children: Vec<String>,
    /// Associated methods count
    pub methods_count: usize,
}

impl AdvancedSearchQuery {
    /// Create simple search query
    pub fn simple(pattern: String) -> Self {
        Self {
            pattern,
            limit: Some(50),
            include_system: true,
            include_user: true,
            type_filter: Some(TypeFilter::All),
        }
    }
}

impl SearchResults {
    /// Create empty search results
    pub fn empty() -> Self {
        Self {
            results: vec![],
            total_count: 0,
            truncated: false,
        }
    }
    
    /// Create search results with data
    pub fn with_results(results: Vec<TypeSearchResult>) -> Self {
        let count = results.len();
        Self {
            results,
            total_count: count,
            truncated: false,
        }
    }
}

impl TypeHierarchy {
    /// Create empty hierarchy
    pub fn empty() -> Self {
        Self {
            roots: vec![],
            types_map: HashMap::new(),
        }
    }
}
