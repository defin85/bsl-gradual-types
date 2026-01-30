//! TypeResolution constructors
//!
//! Factory methods for creating TypeResolution instances.

use super::super::certainty::{Certainty, ResolutionMetadata, ResolutionSource, UncertaintyReason};
use super::super::concrete::{ConcreteType, PlatformType};
use super::super::facets::FacetKind;
use super::super::generics::GenericType;
use super::super::metadata::{ConfigurationType, MetadataKind};
use super::super::primitives::{PrimitiveType, SpecialType};
use super::super::raw_data::RawTypeData;
use super::super::resolution::{ResolutionResult, TypeResolution};

impl TypeResolution {
    /// Create an unknown resolution
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

    /// Create Unknown resolution for undeclared variable
    pub fn undeclared_variable(name: &str) -> Self {
        Self {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                uncertainty_reason: Some(UncertaintyReason::UndeclaredVariable {
                    name: name.to_string(),
                }),
                ..Default::default()
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Check if this is an undeclared variable
    pub fn is_undeclared_variable(&self) -> Option<&str> {
        match &self.metadata.uncertainty_reason {
            Some(UncertaintyReason::UndeclaredVariable { name }) => Some(name),
            _ => None,
        }
    }

    /// Create a known resolution with concrete type
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

    /// Create TypeResolution from RawTypeData preserving all metadata (including facets)
    pub fn from_raw_type(raw_type: &RawTypeData) -> Self {
        let mut resolution = Self::known(ConcreteType::Platform(PlatformType {
            name: raw_type.name.clone(),
        }));
        // Copy facets from RawTypeData
        resolution.available_facets = raw_type.facets.clone();
        resolution
    }

    // ============================================================================
    // Milestone 3.18 Phase 1: Constructors and Utility Methods
    // ============================================================================

    /// Known primitive type (Number, String, Boolean, Date)
    ///
    /// For unrecognized names creates Platform type with that name.
    pub fn primitive(type_name: &str) -> Self {
        let lower = type_name.to_lowercase();
        let concrete = match lower.as_str() {
            "число" | "number" => ConcreteType::Primitive(PrimitiveType::Number),
            "строка" | "string" => ConcreteType::Primitive(PrimitiveType::String),
            "булево" | "boolean" => ConcreteType::Primitive(PrimitiveType::Boolean),
            "дата" | "date" => ConcreteType::Primitive(PrimitiveType::Date),
            _ => ConcreteType::Platform(PlatformType {
                name: type_name.to_string(),
            }),
        };

        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(concrete),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Inferred type with confident inference (80% certainty)
    ///
    /// Use for strong inference from context, method returns, constructors.
    /// For weak inference, use `inferred_weak()`.
    pub fn inferred(type_name: &str) -> Self {
        let mut resolution = Self::primitive(type_name);
        resolution.certainty = Certainty::Inferred;
        resolution.source = ResolutionSource::Inferred;
        resolution
    }

    /// Weakly inferred type (50% certainty)
    ///
    /// Use for fallback inference, configuration not loaded, uncertain context.
    pub fn inferred_weak(type_name: &str) -> Self {
        let mut resolution = Self::primitive(type_name);
        resolution.certainty = Certainty::InferredWeak;
        resolution.source = ResolutionSource::Inferred;
        resolution
    }

    /// Metadata type (catalog, document, etc.) with optional facet
    pub fn metadata_type(kind: MetadataKind, name: &str, facet: Option<FacetKind>) -> Self {
        let available_facets = facet.map_or_else(Vec::new, |f| vec![f]);

        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: name.to_string(),
                facet,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: facet,
            available_facets,
        }
    }

    /// Explicitly specified type (100% certainty)
    ///
    /// Semantically equivalent to primitive(), but with source: Static.
    pub fn explicit(type_name: &str) -> Self {
        Self::primitive(type_name)
    }

    /// Generic type with parameters (Array<String>, Map<String, Number>)
    ///
    /// # Arguments
    /// * `base_type` - Base type name (e.g., "Массив", "Соответствие")
    /// * `type_params` - Type parameters (use "?" for unknown)
    /// * `certainty` - Certainty level for the generic type
    pub fn generic(base_type: &str, type_params: &[&str], certainty: Certainty) -> Self {
        let params: Vec<ConcreteType> = type_params
            .iter()
            .map(|p| {
                if *p == "?" {
                    ConcreteType::Special(SpecialType::Undefined)
                } else {
                    Self::string_to_concrete(p)
                }
            })
            .collect();

        Self {
            certainty,
            result: ResolutionResult::Generic(GenericType {
                base_type: base_type.to_string(),
                type_params: params,
            }),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Helper: String -> ConcreteType
    pub fn string_to_concrete(type_name: &str) -> ConcreteType {
        match type_name.to_lowercase().as_str() {
            "строка" | "string" => ConcreteType::Primitive(PrimitiveType::String),
            "число" | "number" => ConcreteType::Primitive(PrimitiveType::Number),
            "булево" | "boolean" => ConcreteType::Primitive(PrimitiveType::Boolean),
            "дата" | "date" => ConcreteType::Primitive(PrimitiveType::Date),
            _ => ConcreteType::Platform(PlatformType {
                name: type_name.to_string(),
            }),
        }
    }
}
