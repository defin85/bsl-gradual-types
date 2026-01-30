//! bsl-types
//!
//! Foundational types shared across the workspace.

pub mod context_requirements;
pub mod facet_utils;
pub mod metadata_patterns;
pub mod type_definition_location;
pub mod type_id;
pub mod types;

pub use context_requirements::ContextRequirements;
pub use type_definition_location::{ModulePaths, TypeDefinitionLocation};
pub use type_id::{camel_to_spaced, normalize, spaced_to_camel, TypeId};
