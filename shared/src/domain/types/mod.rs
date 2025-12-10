//! BSL Type System
//!
//! This module contains the core type system for BSL (1C Language).
//! Based on the gradual typing approach from Balyuk & Popova (2021).
//!
//! # Module Structure
//!
//! - `primitives`: Primitive types (String, Number, Boolean, Date)
//! - `certainty`: Type resolution certainty and metadata
//! - `metadata`: 1C configuration metadata types
//! - `facets`: Facet system for configuration types
//! - `concrete`: Concrete type definitions
//! - `generics`: Generic types with parameters
//! - `resolution`: Type resolution structures
//! - `compatibility`: Type compatibility checking
//! - `diagnostics`: Type-related diagnostics
//! - `raw_data`: Raw type data from parsers
//! - `global_functions`: Global function types

mod certainty;
mod compatibility;
mod concrete;
mod diagnostics;
mod facets;
mod generics;
mod global_functions;
mod metadata;
mod primitives;
mod raw_data;
mod resolution;
mod resolution_impl;

#[cfg(test)]
mod tests;

// Re-exports for public API
pub use certainty::*;
pub use compatibility::*;
pub use concrete::*;
pub use diagnostics::*;
pub use facets::*;
pub use generics::*;
pub use global_functions::*;
pub use metadata::*;
pub use primitives::*;
pub use raw_data::*;
pub use resolution::*;
