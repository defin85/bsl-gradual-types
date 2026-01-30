//! bsl-types
//!
//! Foundational types shared across the workspace.

pub mod type_id;

pub use type_id::{camel_to_spaced, normalize, spaced_to_camel, TypeId};
