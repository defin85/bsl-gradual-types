//! Structural members attached to resolved types.

use serde::{Deserialize, Serialize};

use super::certainty::Certainty;
use super::resolution::TypeResolution;

/// Snapshot-local source span for a structural member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralMemberSpan {
    pub start: u32,
    pub end: u32,
}

impl StructuralMemberSpan {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

/// Snapshot-local property/column/field attached to a resolved owner type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralMember {
    /// Canonical member name preserved for display.
    pub canonical_name: String,
    /// Resolved member type.
    pub member_type: Box<TypeResolution>,
    /// Source span where the member was introduced or updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_span: Option<StructuralMemberSpan>,
    /// Certainty of the member entry itself.
    pub certainty: Certainty,
}

impl StructuralMember {
    pub fn new(
        canonical_name: impl Into<String>,
        member_type: TypeResolution,
        source_span: Option<StructuralMemberSpan>,
        certainty: Certainty,
    ) -> Self {
        Self {
            canonical_name: canonical_name.into(),
            member_type: Box::new(member_type),
            source_span,
            certainty,
        }
    }

    pub fn known(canonical_name: impl Into<String>, member_type: TypeResolution) -> Self {
        Self::new(canonical_name, member_type, None, Certainty::Known)
    }

    pub fn matches_name(&self, member_name: &str) -> bool {
        crate::type_id::normalize(&self.canonical_name) == crate::type_id::normalize(member_name)
    }
}
