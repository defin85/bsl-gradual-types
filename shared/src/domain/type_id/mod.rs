//! Compatibility shim for `bsl_shared::domain::type_id`.
//!
//! Canonical implementation lives in `bsl-types`.

pub use bsl_types::type_id::TypeId;
pub use bsl_types::type_id::{camel_to_spaced, normalize, spaced_to_camel};

pub mod normalization {
    pub use bsl_types::type_id::normalization::*;
}
