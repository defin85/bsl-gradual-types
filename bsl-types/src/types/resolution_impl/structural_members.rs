//! Structural member helpers for TypeResolution.

use crate::types::{StructuralMember, TypeResolution};

impl TypeResolution {
    pub fn structural_members(&self) -> &[StructuralMember] {
        &self.metadata.structural_members
    }

    pub fn find_structural_member(&self, member_name: &str) -> Option<&StructuralMember> {
        self.metadata
            .structural_members
            .iter()
            .find(|member| member.matches_name(member_name))
    }

    pub fn with_structural_member(mut self, member: StructuralMember) -> Self {
        self.add_structural_member(member);
        self
    }

    pub fn add_structural_member(&mut self, member: StructuralMember) {
        if let Some(existing) = self
            .metadata
            .structural_members
            .iter_mut()
            .find(|existing| existing.matches_name(&member.canonical_name))
        {
            let preserved_name = existing.canonical_name.clone();
            let preserved_member_id = existing.member_id.clone();
            *existing = member;
            existing.canonical_name = preserved_name;
            existing.member_id = preserved_member_id;
            return;
        }

        self.metadata.structural_members.push(member);
    }
}
