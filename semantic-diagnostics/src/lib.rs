//! Semantic diagnostics for BSL code (domain-level).
//!
//! Extracted from `bsl-backend` to keep `bsl-analysis-v2` free of backend dependencies.

pub mod helpers;
mod validators;
mod visitor;

pub use visitor::SemanticValidationVisitor;
