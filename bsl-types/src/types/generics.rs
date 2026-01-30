//! Generic type definitions
//!
//! This module contains generic and weighted type definitions:
//! - `GenericType`: Type with type parameters (Array<String>, Map<String, Number>)
//! - `WeightedType`: Type with probability weight for union types

use serde::{Deserialize, Serialize};

use super::concrete::ConcreteType;
use super::primitives::PrimitiveType;

/// Generic type with type parameters
///
/// # Examples
/// - `Массив<Строка>` (Array<String>)
/// - `Соответствие<Строка, Число>` (Map<String, Number>)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericType {
    /// Base type name (e.g., "Массив")
    pub base_type: String,
    /// Type parameters
    pub type_params: Vec<ConcreteType>,
}

impl GenericType {
    /// Create a typed array: Array<T>
    pub fn array(element_type: ConcreteType) -> Self {
        Self {
            base_type: "Массив".to_string(),
            type_params: vec![element_type],
        }
    }

    /// Create a typed map: Map<K, V>
    pub fn map(key_type: ConcreteType, value_type: ConcreteType) -> Self {
        Self {
            base_type: "Соответствие".to_string(),
            type_params: vec![key_type, value_type],
        }
    }

    /// Create a typed list: List<T>
    pub fn list(element_type: ConcreteType) -> Self {
        Self {
            base_type: "Список".to_string(),
            type_params: vec![element_type],
        }
    }

    /// Create a typed structure: Structure<...>
    pub fn structure(field_types: Vec<ConcreteType>) -> Self {
        Self {
            base_type: "Структура".to_string(),
            type_params: field_types,
        }
    }

    /// Get element type for collections (first parameter)
    pub fn element_type(&self) -> Option<&ConcreteType> {
        self.type_params.first()
    }

    /// Create a string array
    pub fn string_array() -> Self {
        Self::array(ConcreteType::Primitive(PrimitiveType::String))
    }

    /// Create a number array
    pub fn number_array() -> Self {
        Self::array(ConcreteType::Primitive(PrimitiveType::Number))
    }
}

/// Type with probability weight
///
/// Used in union types to indicate how likely each variant is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedType {
    /// The concrete type
    pub type_: ConcreteType,
    /// Probability weight (0.0 - 1.0)
    pub weight: f32,
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
