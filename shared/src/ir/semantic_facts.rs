use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::signature_index::{ConstructorSignature, MethodSignature};
use crate::domain::types::TypeResolution;
use crate::domain::TypeDefinitionLocation;

use super::Span;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticTypeEntry {
    pub span: Span,
    pub resolution: TypeResolution,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticFacts {
    #[serde(default)]
    pub type_entries: Vec<SemanticTypeEntry>,
    #[serde(default)]
    pub definition_locations_by_span: HashMap<Span, TypeDefinitionLocation>,
    #[serde(default)]
    pub assignment_value_type_by_span: HashMap<Span, TypeResolution>,
    #[serde(default)]
    pub call_receiver_type_by_span: HashMap<Span, TypeResolution>,
    #[serde(default)]
    pub call_arg_types_by_span: HashMap<Span, Vec<TypeResolution>>,
    #[serde(default)]
    pub member_access_object_type_by_span: HashMap<Span, TypeResolution>,
    #[serde(default)]
    pub call_method_targets_by_span: HashMap<Span, SemanticMethodTarget>,
    #[serde(default)]
    pub member_method_targets_by_span: HashMap<Span, SemanticMethodTarget>,
    #[serde(default)]
    pub constructor_targets_by_span: HashMap<Span, SemanticConstructorTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticMethodTarget {
    pub owner_type: Option<String>,
    pub method_name: String,
    pub signature: Option<MethodSignature>,
    pub definition_location: Option<TypeDefinitionLocation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticConstructorTarget {
    pub type_name: String,
    pub signature: Option<ConstructorSignature>,
}

impl SemanticFacts {
    pub fn type_for_exact_span(&self, span: Span) -> Option<TypeResolution> {
        self.type_entries
            .iter()
            .find(|entry| entry.span == span)
            .map(|entry| entry.resolution.clone())
    }

    pub fn type_at_byte_offset(&self, byte_offset: u32) -> Option<TypeResolution> {
        let find = |offset: u32| {
            self.type_entries
                .iter()
                .filter(|entry| entry.span.contains(offset))
                .min_by_key(|entry| entry.span.len())
                .map(|entry| entry.resolution.clone())
        };

        find(byte_offset).or_else(|| byte_offset.checked_sub(1).and_then(find))
    }

    pub fn type_resolution_for_span(&self, span: Span) -> Option<TypeResolution> {
        if let Some(exact) = self.type_for_exact_span(span) {
            return Some(exact);
        }
        if span.start == span.end {
            return self.type_at_byte_offset(span.start);
        }
        let end_inclusive = span.end.saturating_sub(1);
        self.type_at_byte_offset(end_inclusive)
            .or_else(|| self.type_at_byte_offset(span.start))
    }

    pub fn definition_location_for_exact_span(&self, span: Span) -> Option<TypeDefinitionLocation> {
        self.definition_locations_by_span.get(&span).cloned()
    }

    pub fn definition_location_at_byte_offset(
        &self,
        byte_offset: u32,
    ) -> Option<TypeDefinitionLocation> {
        let find = |offset: u32| {
            self.definition_locations_by_span
                .iter()
                .filter(|(span, _)| span.contains(offset))
                .min_by_key(|(span, _)| span.len())
                .map(|(_, location)| location.clone())
        };

        find(byte_offset).or_else(|| byte_offset.checked_sub(1).and_then(find))
    }

    pub fn definition_location_for_span(&self, span: Span) -> Option<TypeDefinitionLocation> {
        if let Some(exact) = self.definition_location_for_exact_span(span) {
            return Some(exact);
        }
        if span.start == span.end {
            return self.definition_location_at_byte_offset(span.start);
        }
        let end_inclusive = span.end.saturating_sub(1);
        self.definition_location_at_byte_offset(end_inclusive)
            .or_else(|| self.definition_location_at_byte_offset(span.start))
    }
}
