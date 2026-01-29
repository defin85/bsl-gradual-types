//! Semantic diagnostics for BSL code (domain-level).
//!
//! Extracted from `bsl-backend` to keep `bsl-analysis-v2` free of backend dependencies.

pub mod helpers;
mod type_hints;
mod validators;
mod visitor;

pub use type_hints::SemanticTypeHints;
pub use visitor::SemanticValidationVisitor;
