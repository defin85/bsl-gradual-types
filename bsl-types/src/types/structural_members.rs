//! Structural members attached to resolved types.

use serde::{Deserialize, Deserializer, Serialize};

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

/// Stable snapshot-local identifier for a structural member entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct StructuralMemberId {
    pub key: String,
}

impl StructuralMemberId {
    pub fn new(canonical_name: impl AsRef<str>, source_span: Option<StructuralMemberSpan>) -> Self {
        let normalized_name = crate::type_id::normalize(canonical_name.as_ref());
        let key = match source_span {
            Some(span) => format!("{normalized_name}@{}:{}", span.start, span.end),
            None => normalized_name,
        };

        Self { key }
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }
}

/// Snapshot-local property/column/field attached to a resolved owner type.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StructuralMember {
    /// Stable snapshot-local identity for cross-consumer comparison.
    #[serde(default)]
    pub member_id: StructuralMemberId,
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

#[derive(Deserialize)]
struct StructuralMemberRepr {
    #[serde(default)]
    member_id: StructuralMemberId,
    canonical_name: String,
    member_type: Box<TypeResolution>,
    #[serde(default)]
    source_span: Option<StructuralMemberSpan>,
    certainty: Certainty,
}

impl StructuralMember {
    pub fn new(
        canonical_name: impl Into<String>,
        member_type: TypeResolution,
        source_span: Option<StructuralMemberSpan>,
        certainty: Certainty,
    ) -> Self {
        let canonical_name = canonical_name.into();
        Self {
            member_id: StructuralMemberId::new(&canonical_name, source_span),
            canonical_name,
            member_type: Box::new(member_type),
            source_span,
            certainty,
        }
    }

    pub fn known(canonical_name: impl Into<String>, member_type: TypeResolution) -> Self {
        Self::new(canonical_name, member_type, None, Certainty::Known)
    }

    pub fn with_member_id(mut self, member_id: StructuralMemberId) -> Self {
        self.member_id = member_id;
        self
    }

    pub fn matches_name(&self, member_name: &str) -> bool {
        crate::type_id::normalize(&self.canonical_name) == crate::type_id::normalize(member_name)
    }
}

impl<'de> Deserialize<'de> for StructuralMember {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = StructuralMemberRepr::deserialize(deserializer)?;
        let member_id = if repr.member_id.is_empty() {
            StructuralMemberId::new(&repr.canonical_name, repr.source_span)
        } else {
            repr.member_id
        };

        Ok(Self {
            member_id,
            canonical_name: repr.canonical_name,
            member_type: repr.member_type,
            source_span: repr.source_span,
            certainty: repr.certainty,
        })
    }
}
