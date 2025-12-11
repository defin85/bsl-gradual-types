//! TypeResolution query methods
//!
//! Methods for querying type state and information.

use super::super::resolution::{ResolutionResult, TypeResolution};
use super::super::certainty::Certainty;

impl TypeResolution {
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
                super::super::concrete::ConcreteType::Primitive(pt) => {
                    pt.display_name().to_string()
                }
                super::super::concrete::ConcreteType::Platform(platform) => platform.name.clone(),
                super::super::concrete::ConcreteType::Configuration(cfg) => {
                    if let Some(ref facet) = self.active_facet {
                        format!("{}.{}", cfg.kind.faceted_type_prefix(facet), cfg.name)
                    } else {
                        format!("{}.{}", cfg.kind.to_prefix(), cfg.name)
                    }
                }
                super::super::concrete::ConcreteType::Special(special) => {
                    special.display_name().to_string()
                }
                super::super::concrete::ConcreteType::GlobalFunction(func) => func.name.clone(),
                super::super::concrete::ConcreteType::TabularRow(tr) => tr.get_full_name(),
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
}
