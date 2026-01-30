//! Global function types for BSL
//!
//! This module contains types for global functions:
//! - `GlobalFunctionInfo`: Complete function information
//! - `ParameterInfo`: Function parameter information

use serde::{Deserialize, Serialize};

/// Information about a global function (defined in Domain Layer)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalFunctionInfo {
    /// Function name (Russian)
    pub name: String,
    /// Function name (English)
    pub english_name: Option<String>,
    /// Function description
    pub description: Option<String>,
    /// Function parameters
    pub parameters: Vec<ParameterInfo>,
    /// Return type
    pub return_type: Option<String>,
    /// Return value description
    pub return_description: Option<String>,
    /// Whether function is polymorphic
    pub polymorphic: bool,
    /// Whether function is pure (no side effects)
    pub pure: bool,
    /// Available execution contexts
    pub contexts: Vec<String>,
    /// Function category
    pub category: Option<String>,
}

/// Information about a function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub type_name: Option<String>,
    /// Whether parameter is optional
    pub is_optional: bool,
    /// Default value
    pub default_value: Option<String>,
    /// Parameter description
    pub description: Option<String>,
}
