//! Certainty and resolution metadata for type system
//!
//! This module contains types for tracking type resolution confidence:
//! - `Certainty`: Known, Inferred, Unknown
//! - `UncertaintyReason`: Why a type is unknown
//! - `ResolutionSource`: Where the type came from
//! - `ResolutionMetadata`: Additional resolution information

use serde::{Deserialize, Serialize};

use super::metadata::MetadataKind;

/// Certainty level of type resolution
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Certainty {
    /// Type is definitively known (100% confidence)
    Known,
    /// Type is inferred with given confidence (0.0 - 1.0)
    Inferred(f32),
    /// Type is unknown
    Unknown,
}

/// Reason why type resolution resulted in Unknown certainty
/// Used for precise error messages and validation decisions
///
/// # MILESTONE 3.16: Unknown Certainty Semantics
///
/// This enum captures WHY a type is unknown, enabling:
/// - Precise error messages (e.g., "Document 'Контрогенты' not found")
/// - Graceful degradation when configuration is not loaded
/// - Suggestions for typo fixes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UncertaintyReason {
    /// Configuration metadata is not loaded yet
    /// In this case, we don't report errors (graceful degradation)
    ConfigurationNotLoaded,

    /// Metadata object was not found in the loaded configuration
    /// This indicates a potential typo or missing object
    MetadataObjectNotFound {
        /// Kind of metadata (Document, Catalog, etc.)
        kind: MetadataKind,
        /// Name that was not found
        name: String,
    },

    /// Other reason for uncertainty
    Other(String),

    /// Variable not found in scope
    UndeclaredVariable {
        /// Variable name
        name: String,
    },
}

/// Source of the type resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionSource {
    /// Type determined statically from code analysis
    Static,
    /// Type inferred from context
    Inferred,
    /// Type specified by annotation
    Annotated,
    /// Type determined at runtime (dynamic)
    Runtime,
    /// Type predicted by heuristics
    Predicted,
}

/// Additional metadata about type resolution
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResolutionMetadata {
    /// Source file path
    pub file: Option<String>,
    /// Line number in source
    pub line: Option<u32>,
    /// Column number in source
    pub column: Option<u32>,
    /// Additional notes about resolution
    pub notes: Vec<String>,
    /// MILESTONE 3.16: Reason why type resolution resulted in Unknown/Inferred certainty
    /// Used for precise error messages and validation decisions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncertainty_reason: Option<UncertaintyReason>,
}
