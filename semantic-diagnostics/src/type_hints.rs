use std::collections::HashMap;

use bsl_shared::domain::types::TypeResolution;
use bsl_shared::ir::Span;

#[derive(Default)]
pub struct SemanticTypeHints {
    pub assignment_value_type_by_span: HashMap<Span, TypeResolution>,
    pub call_receiver_type_by_span: HashMap<Span, TypeResolution>,
    pub call_arg_types_by_span: HashMap<Span, Vec<TypeResolution>>,
    pub member_access_object_type_by_span: HashMap<Span, TypeResolution>,
}

impl SemanticTypeHints {
    pub fn assignment_value_type(&self, span: Span) -> Option<&TypeResolution> {
        self.assignment_value_type_by_span.get(&span)
    }

    pub fn call_receiver_type(&self, span: Span) -> Option<&TypeResolution> {
        self.call_receiver_type_by_span.get(&span)
    }

    pub fn call_arg_types(&self, span: Span) -> Option<&[TypeResolution]> {
        self.call_arg_types_by_span.get(&span).map(Vec::as_slice)
    }

    pub fn member_access_object_type(&self, span: Span) -> Option<&TypeResolution> {
        self.member_access_object_type_by_span.get(&span)
    }
}

