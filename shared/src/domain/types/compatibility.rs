//! Type compatibility and comparison
//!
//! This module contains types for type compatibility checking:
//! - `TypeCompatibility`: Result of type compatibility check
//! - `TypeRef`: Lazy reference to a type in repository

use serde::{Deserialize, Serialize};

/// Reference to a type in TypeRepository (lazy lookup)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRef {
    /// Type name for lookup in repository
    pub lookup_key: String,
    /// Cached hash for fast comparison
    pub type_hash: u64,
}

impl TypeRef {
    /// Create a new type reference
    pub fn new(lookup_key: &str) -> Self {
        Self {
            lookup_key: lookup_key.to_string(),
            type_hash: Self::hash_type_name(lookup_key),
        }
    }

    fn hash_type_name(name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.to_lowercase().hash(&mut hasher);
        hasher.finish()
    }
}

/// Result of type compatibility comparison
#[derive(Debug, Clone, PartialEq)]
pub enum TypeCompatibility {
    /// Fully compatible
    Compatible,
    /// Incompatible with reason
    Incompatible { reason: String },
}

impl TypeCompatibility {
    /// Check if types are compatible
    pub fn is_compatible(&self) -> bool {
        matches!(self, TypeCompatibility::Compatible)
    }

    /// Get the reason string (empty for Compatible)
    pub fn reason(&self) -> String {
        match self {
            TypeCompatibility::Compatible => String::new(),
            TypeCompatibility::Incompatible { reason } => reason.clone(),
        }
    }
}
