//! Type resolution structures
//!
//! This module contains core resolution types:
//! - `ResolutionResult`: Result of type resolution (Concrete, Union, etc.)
//! - `TypeResolution`: Complete resolution with certainty, facets, and metadata

use serde::{Deserialize, Serialize};

use super::certainty::{Certainty, ResolutionMetadata, ResolutionSource};
use super::concrete::ConcreteType;
use super::facets::FacetKind;
use super::generics::{GenericType, WeightedType};
use super::primitives::SpecialType;

/// Result of type resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolutionResult {
    /// Concrete type
    Concrete(ConcreteType),
    /// Union of weighted types
    Union(Vec<WeightedType>),
    /// Intersection of types
    Intersection(Vec<ConcreteType>),
    /// Generic type with parameters
    Generic(GenericType),
    /// Nullable type (T | Null)
    Nullable(Box<ConcreteType>),
    /// Dynamic/unknown type
    Dynamic,
}

impl ResolutionResult {
    /// Normalize Union types: deduplicate, simplify, and sort
    ///
    /// # Examples
    /// - `String | String` -> `String`
    /// - `Number | String | Number` -> `Number | String`
    /// - `String | Dynamic` -> `Dynamic`
    pub fn normalize_union(types: Vec<WeightedType>) -> Self {
        if types.is_empty() {
            return ResolutionResult::Dynamic;
        }

        // 1. Check for Dynamic - if present, return Dynamic
        if types
            .iter()
            .any(|wt| matches!(wt.type_, ConcreteType::Special(SpecialType::Undefined)))
        {
            return ResolutionResult::Dynamic;
        }

        // 2. Deduplicate and merge weights
        let mut type_map: std::collections::HashMap<String, (ConcreteType, f32)> =
            std::collections::HashMap::new();

        for weighted in types {
            let key = format!("{:?}", weighted.type_); // Simple key based on Debug representation
            type_map
                .entry(key)
                .and_modify(|(_, w)| *w += weighted.weight)
                .or_insert((weighted.type_, weighted.weight));
        }

        // 3. Convert back to Vec and sort by weight (descending)
        let mut normalized: Vec<WeightedType> = type_map
            .into_values()
            .map(|(t, w)| WeightedType {
                type_: t,
                weight: w,
            })
            .collect();

        normalized.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 4. If only one type remains, return Concrete
        if normalized.len() == 1 {
            if let Some(single) = normalized.into_iter().next() {
                return ResolutionResult::Concrete(single.type_);
            }
            return ResolutionResult::Dynamic;
        }

        ResolutionResult::Union(normalized)
    }

    /// Create an intersection type with validation
    pub fn intersection(types: Vec<ConcreteType>) -> Self {
        if types.is_empty() {
            return ResolutionResult::Dynamic;
        }

        if types.len() == 1 {
            if let Some(single) = types.into_iter().next() {
                return ResolutionResult::Concrete(single);
            }
            return ResolutionResult::Dynamic;
        }

        // Deduplicate
        let mut unique_types = Vec::new();
        for t in types {
            if !unique_types.contains(&t) {
                unique_types.push(t);
            }
        }

        if unique_types.len() == 1 {
            if let Some(single) = unique_types.into_iter().next() {
                return ResolutionResult::Concrete(single);
            }
            return ResolutionResult::Dynamic;
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
            ResolutionResult::Union(types) => types
                .iter()
                .any(|wt| matches!(wt.type_, ConcreteType::Special(SpecialType::Null))),
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

/// Complete type resolution with certainty, facets, and metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeResolution {
    /// Certainty level of resolution
    pub certainty: Certainty,
    /// Resolution result
    pub result: ResolutionResult,
    /// Source of resolution
    pub source: ResolutionSource,
    /// Additional metadata
    pub metadata: ResolutionMetadata,
    /// Currently active facet
    pub active_facet: Option<FacetKind>,
    /// All available facets for this type
    pub available_facets: Vec<FacetKind>,
}
