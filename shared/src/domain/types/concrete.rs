//! Concrete type definitions
//!
//! This module contains concrete (resolved) type definitions:
//! - `ConcreteType`: Enum of all possible concrete types
//! - `PlatformType`: Platform type wrapper

use serde::{Deserialize, Serialize};
use std::fmt;

use super::global_functions::GlobalFunctionInfo;
use super::metadata::{ConfigurationType, TabularRowType};
use super::primitives::{PrimitiveType, SpecialType};

/// Concrete type in the BSL type system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConcreteType {
    /// Platform type (e.g., Array, ValueTable)
    Platform(PlatformType),
    /// Configuration type (Catalog, Document, etc.)
    Configuration(ConfigurationType),
    /// Primitive type (String, Number, Boolean, Date)
    Primitive(PrimitiveType),
    /// Special type (Undefined, Null, Type)
    Special(SpecialType),
    /// Global function as a type
    GlobalFunction(GlobalFunctionInfo),
    /// Tabular section row type
    TabularRow(TabularRowType),
}

impl ConcreteType {
    /// Check if this type is compatible with another for intersection
    pub fn is_intersection_compatible(&self, other: &Self) -> bool {
        // Primitive types cannot be intersected
        if matches!(self, ConcreteType::Primitive(_)) && matches!(other, ConcreteType::Primitive(_))
        {
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

    /// Create a primitive date type
    pub fn date() -> Self {
        ConcreteType::Primitive(PrimitiveType::Date)
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

impl fmt::Display for ConcreteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConcreteType::Platform(platform) => write!(f, "{}", platform.name),
            ConcreteType::Configuration(config) => {
                // If facet is specified - use faceted prefix (CatalogManager.Clients)
                // Otherwise use standard display_name (Catalogs.Clients)
                if let Some(facet) = &config.facet {
                    write!(
                        f,
                        "{}.{}",
                        config.kind.faceted_type_prefix(facet),
                        config.name
                    )
                } else {
                    write!(f, "{}.{}", config.kind.display_name(), config.name)
                }
            }
            ConcreteType::Primitive(primitive) => write!(f, "{}", primitive.display_name()),
            ConcreteType::Special(special) => write!(f, "{}", special.display_name()),
            ConcreteType::GlobalFunction(func) => write!(f, "{}()", func.name),
            ConcreteType::TabularRow(tr) => write!(f, "{}", tr.get_full_name()),
        }
    }
}

/// Platform type wrapper
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformType {
    /// Type name
    pub name: String,
}
