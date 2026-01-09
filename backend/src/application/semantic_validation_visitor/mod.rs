//! Compatibility module: semantic diagnostics visitor.
//!
//! The implementation is extracted into `bsl-semantic-diagnostics` crate to keep `bsl-analysis-v2`
//! independent from backend.

pub use bsl_semantic_diagnostics::SemanticValidationVisitor;

pub mod helpers {
    pub use bsl_semantic_diagnostics::helpers::{
        collection_name_to_metadata_kind, is_metadata_collection_name,
    };
}
