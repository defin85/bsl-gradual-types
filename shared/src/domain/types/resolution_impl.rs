//! TypeResolution implementation
//!
//! This module contains all methods for TypeResolution

use super::certainty::{Certainty, ResolutionMetadata, ResolutionSource, UncertaintyReason};
use super::compatibility::TypeCompatibility;
use super::concrete::{ConcreteType, PlatformType};
use super::facets::FacetKind;
use super::generics::GenericType;
use super::metadata::{ConfigurationType, MetadataKind};
use super::primitives::{PrimitiveType, SpecialType};
use super::raw_data::RawTypeData;
use super::resolution::{ResolutionResult, TypeResolution};

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

    /// Get definition location for Go To Definition
    ///
    /// # Returns
    /// - `Some(TypeDefinitionLocation::Primitive)` for primitive types
    /// - `Some(TypeDefinitionLocation::Platform)` for platform types
    /// - `Some(TypeDefinitionLocation::Configuration)` for configuration types
    /// - `None` for Dynamic types
    pub fn get_definition_location(
        &self,
    ) -> Option<crate::domain::type_definition_location::TypeDefinitionLocation> {
        use crate::domain::type_definition_location::TypeDefinitionLocation;
        use std::path::PathBuf;

        match &self.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(_)) => {
                Some(TypeDefinitionLocation::primitive())
            }

            ResolutionResult::Concrete(ConcreteType::Platform(pt)) => {
                Some(TypeDefinitionLocation::platform(&pt.name))
            }

            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
                Some(TypeDefinitionLocation::configuration(PathBuf::from(
                    &type_key,
                )))
            }

            ResolutionResult::Concrete(ConcreteType::Special(_)) => {
                Some(TypeDefinitionLocation::primitive())
            }

            ResolutionResult::Concrete(ConcreteType::GlobalFunction(func)) => {
                Some(TypeDefinitionLocation::platform(&func.name))
            }

            ResolutionResult::Concrete(ConcreteType::TabularRow(tr)) => {
                let type_key = format!("{}.{}", tr.parent_type, tr.tabular_section_name);
                Some(TypeDefinitionLocation::configuration(PathBuf::from(
                    &type_key,
                )))
            }

            ResolutionResult::Generic(gen) => {
                Some(TypeDefinitionLocation::platform(&gen.base_type))
            }

            ResolutionResult::Nullable(inner) => {
                let inner_resolution = TypeResolution::known(*inner.clone());
                inner_resolution.get_definition_location()
            }

            ResolutionResult::Union(variants) => {
                if let Some(first) = variants.first() {
                    let first_resolution = TypeResolution::known(first.type_.clone());
                    first_resolution.get_definition_location()
                } else {
                    None
                }
            }

            ResolutionResult::Intersection(types) => {
                if let Some(first) = types.first() {
                    let first_resolution = TypeResolution::known(first.clone());
                    first_resolution.get_definition_location()
                } else {
                    None
                }
            }

            ResolutionResult::Dynamic => None,
        }
    }

    /// Get definition location with module paths (Milestone 3.14)
    pub fn get_definition_location_with_modules(
        &self,
        module_paths: Option<&crate::domain::type_definition_location::ModulePaths>,
    ) -> Option<crate::domain::type_definition_location::TypeDefinitionLocation> {
        use crate::domain::type_definition_location::TypeDefinitionLocation;
        use std::path::PathBuf;

        match &self.result {
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
                let metadata_path = PathBuf::from(&type_key);

                if let Some(paths) = module_paths {
                    Some(TypeDefinitionLocation::configuration_with_modules(
                        metadata_path,
                        paths.clone(),
                    ))
                } else {
                    Some(TypeDefinitionLocation::configuration(metadata_path))
                }
            }

            ResolutionResult::Concrete(ConcreteType::TabularRow(tr)) => {
                let type_key = format!("{}.{}", tr.parent_type, tr.tabular_section_name);
                let metadata_path = PathBuf::from(&type_key);

                if let Some(paths) = module_paths {
                    Some(TypeDefinitionLocation::configuration_with_modules(
                        metadata_path,
                        paths.clone(),
                    ))
                } else {
                    Some(TypeDefinitionLocation::configuration(metadata_path))
                }
            }

            _ => self.get_definition_location(),
        }
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

    /// Inferred type with confidence level (0.0 - 1.0)
    pub fn inferred(type_name: &str, confidence: f32) -> Self {
        let mut resolution = Self::primitive(type_name);
        resolution.certainty = Certainty::Inferred(confidence);
        resolution.source = ResolutionSource::Inferred;
        resolution
    }

    /// Metadata type (catalog, document, etc.) with optional facet
    pub fn metadata_type(kind: MetadataKind, name: &str, facet: Option<FacetKind>) -> Self {
        let available_facets = if facet.is_some() {
            vec![facet.unwrap()]
        } else {
            vec![]
        };

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
    pub fn generic(base_type: &str, type_params: &[&str], certainty: f32) -> Self {
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

        let cert = if certainty >= 1.0 {
            Certainty::Known
        } else {
            Certainty::Inferred(certainty)
        };

        Self {
            certainty: cert,
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

    /// Check if type is Unknown
    pub fn is_unknown(&self) -> bool {
        matches!(self.certainty, Certainty::Unknown)
    }

    /// Check if type is Dynamic
    pub fn is_dynamic(&self) -> bool {
        matches!(&self.result, ResolutionResult::Dynamic) || self.type_name() == "Dynamic"
    }

    /// Get type name as String
    pub fn type_name(&self) -> String {
        match &self.result {
            ResolutionResult::Concrete(concrete) => match concrete {
                ConcreteType::Primitive(pt) => pt.display_name().to_string(),
                ConcreteType::Platform(platform) => platform.name.clone(),
                ConcreteType::Configuration(cfg) => {
                    if let Some(ref facet) = self.active_facet {
                        format!("{}.{}", cfg.kind.faceted_type_prefix(facet), cfg.name)
                    } else {
                        format!("{}.{}", cfg.kind.to_prefix(), cfg.name)
                    }
                }
                ConcreteType::Special(special) => special.display_name().to_string(),
                ConcreteType::GlobalFunction(func) => func.name.clone(),
                ConcreteType::TabularRow(tr) => tr.get_full_name(),
            },
            ResolutionResult::Generic(gen) => {
                if gen.type_params.is_empty() {
                    gen.base_type.clone()
                } else {
                    let params: Vec<String> = gen
                        .type_params
                        .iter()
                        .map(|ct| {
                            let resolution = TypeResolution::known(ct.clone());
                            resolution.type_name()
                        })
                        .collect();
                    format!("{}<{}>", gen.base_type, params.join(", "))
                }
            }
            ResolutionResult::Union(variants) => {
                if variants.is_empty() {
                    "Dynamic".to_string()
                } else {
                    let types: Vec<String> = variants
                        .iter()
                        .map(|wt| {
                            let resolution = TypeResolution::known(wt.type_.clone());
                            resolution.type_name()
                        })
                        .collect();
                    types.join(" | ")
                }
            }
            ResolutionResult::Intersection(types) => {
                if types.is_empty() {
                    "Dynamic".to_string()
                } else {
                    let names: Vec<String> = types
                        .iter()
                        .map(|ct| {
                            let resolution = TypeResolution::known(ct.clone());
                            resolution.type_name()
                        })
                        .collect();
                    names.join(" & ")
                }
            }
            ResolutionResult::Nullable(inner) => {
                let inner_resolution = TypeResolution::known(*inner.clone());
                format!("{}?", inner_resolution.type_name())
            }
            ResolutionResult::Dynamic => "Dynamic".to_string(),
        }
    }

    // ============================================================================
    // Milestone 3.13: Object-Based Type Comparison
    // ============================================================================

    /// Object-based type comparison with semantic awareness
    pub fn is_compatible_with(&self, other: &TypeResolution) -> TypeCompatibility {
        // Dynamic/Unknown is compatible with everything (gradual typing)
        if matches!(self.result, ResolutionResult::Dynamic)
            || matches!(other.result, ResolutionResult::Dynamic)
        {
            return TypeCompatibility::Compatible;
        }

        match (&self.result, &other.result) {
            // Concrete types
            (ResolutionResult::Concrete(a), ResolutionResult::Concrete(b)) => {
                Self::check_concrete_compatibility(a, b, self.active_facet, other.active_facet)
            }

            // Union - actual must be compatible with at least one member
            (_, ResolutionResult::Union(variants)) => {
                for variant in variants {
                    let variant_resolution = TypeResolution::known(variant.type_.clone());
                    if self.is_compatible_with(&variant_resolution).is_compatible() {
                        return TypeCompatibility::Compatible;
                    }
                }
                TypeCompatibility::Incompatible {
                    reason: "Не совместим ни с одним вариантом Union".to_string(),
                }
            }

            // Generic - check base type and parameters
            (ResolutionResult::Generic(g1), ResolutionResult::Generic(g2)) => {
                Self::check_generic_compatibility(g1, g2)
            }

            // Nullable
            (ResolutionResult::Nullable(inner), other_result) => {
                let inner_resolution = TypeResolution::known(*inner.clone());
                inner_resolution.is_compatible_with(&TypeResolution {
                    result: other_result.clone(),
                    ..TypeResolution::unknown()
                })
            }

            // Intersection - actual must be compatible with ALL types
            (_, ResolutionResult::Intersection(types)) => {
                for concrete in types {
                    let type_resolution = TypeResolution::known(concrete.clone());
                    if !self.is_compatible_with(&type_resolution).is_compatible() {
                        return TypeCompatibility::Incompatible {
                            reason: "Не совместим со всеми типами Intersection".to_string(),
                        };
                    }
                }
                TypeCompatibility::Compatible
            }

            // Intersection as actual - must contain at least one compatible type
            (ResolutionResult::Intersection(types), _) => {
                for concrete in types {
                    let type_resolution = TypeResolution::known(concrete.clone());
                    if type_resolution.is_compatible_with(other).is_compatible() {
                        return TypeCompatibility::Compatible;
                    }
                }
                TypeCompatibility::Incompatible {
                    reason: "Ни один тип из Intersection не совместим".to_string(),
                }
            }

            _ => TypeCompatibility::Incompatible {
                reason: format!("Типы {:?} и {:?} несовместимы", self.result, other.result),
            },
        }
    }

    /// Check concrete type compatibility
    fn check_concrete_compatibility(
        from: &ConcreteType,
        to: &ConcreteType,
        from_facet: Option<FacetKind>,
        to_facet: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            // Primitives - exact match
            (ConcreteType::Primitive(a), ConcreteType::Primitive(b)) => {
                if a == b {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Примитивы {:?} и {:?} несовместимы", a, b),
                    }
                }
            }

            // Configuration types - consider facets!
            (ConcreteType::Configuration(cfg1), ConcreteType::Configuration(cfg2)) => {
                if cfg1.kind != cfg2.kind || !Self::names_equal(&cfg1.name, &cfg2.name) {
                    return TypeCompatibility::Incompatible {
                        reason: format!(
                            "Разные типы конфигурации: {}.{} vs {}.{}",
                            cfg1.kind.to_prefix(),
                            cfg1.name,
                            cfg2.kind.to_prefix(),
                            cfg2.name
                        ),
                    };
                }
                Self::check_facet_compatibility(from_facet.or(cfg1.facet), to_facet.or(cfg2.facet))
            }

            // Platform types - case-insensitive name comparison
            (ConcreteType::Platform(pt1), ConcreteType::Platform(pt2)) => {
                if Self::names_equal(&pt1.name, &pt2.name) {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!(
                            "Платформенные типы {} и {} несовместимы",
                            pt1.name, pt2.name
                        ),
                    }
                }
            }

            // Special types
            (ConcreteType::Special(s1), ConcreteType::Special(s2)) => {
                if s1 == s2 {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Специальные типы {:?} и {:?} несовместимы", s1, s2),
                    }
                }
            }

            // TabularRow - compare by parent_type and tabular_section_name
            (ConcreteType::TabularRow(tr1), ConcreteType::TabularRow(tr2)) => {
                if Self::names_equal(&tr1.parent_type, &tr2.parent_type)
                    && Self::names_equal(&tr1.tabular_section_name, &tr2.tabular_section_name)
                {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!(
                            "Разные строки табличных частей: {}.{} vs {}.{}",
                            tr1.parent_type, tr1.tabular_section_name,
                            tr2.parent_type, tr2.tabular_section_name
                        ),
                    }
                }
            }

            // GlobalFunction - functions as types are incompatible with each other
            (ConcreteType::GlobalFunction(f1), ConcreteType::GlobalFunction(f2)) => {
                if Self::names_equal(&f1.name, &f2.name) {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Разные глобальные функции: {} vs {}", f1.name, f2.name),
                    }
                }
            }

            _ => TypeCompatibility::Incompatible {
                reason: "Несовместимые категории типов".to_string(),
            },
        }
    }

    /// Check facet compatibility
    fn check_facet_compatibility(
        from: Option<FacetKind>,
        to: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            (None, _) | (_, None) => TypeCompatibility::Compatible,
            (Some(f1), Some(f2)) if f1 == f2 => TypeCompatibility::Compatible,
            // Object -> Reference: allowed (implicit conversion)
            (Some(FacetKind::Object), Some(FacetKind::Reference)) => TypeCompatibility::Compatible,
            // Reference -> Object: NOT allowed (need GetObject())
            (Some(FacetKind::Reference), Some(FacetKind::Object)) => {
                TypeCompatibility::Incompatible {
                    reason: "Ссылка не может быть неявно преобразована в Объект (используйте ПолучитьОбъект())".to_string(),
                }
            }
            // Manager -> any other: NOT allowed
            (Some(FacetKind::Manager), Some(other)) => TypeCompatibility::Incompatible {
                reason: format!("Менеджер несовместим с фасетом {:?}", other),
            },
            (Some(f1), Some(f2)) => TypeCompatibility::Incompatible {
                reason: format!("Фасет {:?} несовместим с {:?}", f1, f2),
            },
        }
    }

    /// Check generic type compatibility
    fn check_generic_compatibility(g1: &GenericType, g2: &GenericType) -> TypeCompatibility {
        // Check base type
        if !Self::names_equal(&g1.base_type, &g2.base_type) {
            return TypeCompatibility::Incompatible {
                reason: format!(
                    "Несовместимые базовые типы: {} vs {}",
                    g1.base_type, g2.base_type
                ),
            };
        }

        // Check parameters
        if g1.type_params.len() != g2.type_params.len() {
            return TypeCompatibility::Incompatible {
                reason: "Разное количество параметров Generic".to_string(),
            };
        }

        for (p1, p2) in g1.type_params.iter().zip(g2.type_params.iter()) {
            let r1 = TypeResolution::known(p1.clone());
            let r2 = TypeResolution::known(p2.clone());
            if !r1.is_compatible_with(&r2).is_compatible() {
                return TypeCompatibility::Incompatible {
                    reason: format!("Параметры Generic {:?} и {:?} несовместимы", p1, p2),
                };
            }
        }

        TypeCompatibility::Compatible
    }

    fn names_equal(a: &str, b: &str) -> bool {
        a.to_lowercase() == b.to_lowercase()
    }
}
