//! TypeResolution definition location methods
//!
//! Methods for Go To Definition functionality.

use super::super::concrete::ConcreteType;
use super::super::resolution::{ResolutionResult, TypeResolution};

impl TypeResolution {
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
}
