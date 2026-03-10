use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::types::TypeResolution;

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
    pub assignment_value_type_by_span: HashMap<Span, TypeResolution>,
    #[serde(default)]
    pub call_receiver_type_by_span: HashMap<Span, TypeResolution>,
    #[serde(default)]
    pub call_arg_types_by_span: HashMap<Span, Vec<TypeResolution>>,
    #[serde(default)]
    pub member_access_object_type_by_span: HashMap<Span, TypeResolution>,
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
}
