//! Semantic Validation Visitor Module
//!
//! This module provides semantic validation for BSL code.
//! It validates type correctness, method/property existence,
//! and metadata object access.
//!
//! # Module Structure
//!
//! - `visitor` - Main SemanticValidationVisitor struct and SemanticVisitor implementation
//! - `validators` - Validation logic (type validation, call validation)
//! - `helpers` - Helper functions for metadata collection detection

pub mod helpers;
mod validators;
mod visitor;

// Re-export main types for public API
pub use visitor::SemanticValidationVisitor;

// Re-export helpers for use in tests
#[cfg(test)]
pub use helpers::{collection_name_to_metadata_kind, is_metadata_collection_name};
