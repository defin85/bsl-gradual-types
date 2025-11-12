//! Semantic Validation Visitor
use bsl_shared::domain::types::{Certainty, ConcreteType, ResolutionResult, TypeDiagnostic};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::ir::{
    FlowContext, SemanticNode, SemanticNodeKind, SemanticProgram, SemanticVisitor,
};

pub struct SemanticValidationVisitor<'a> {
    validator: &'a TypeValidator<'a>,
    errors: Vec<TypeDiagnostic>,
    #[allow(dead_code)]
    program: &'a SemanticProgram,
}

impl<'a> SemanticValidationVisitor<'a> {
    pub fn new(validator: &'a TypeValidator<'a>, program: &'a SemanticProgram) -> Self {
        Self {
            validator,
            errors: Vec::new(),
            program,
        }
    }

    pub fn into_errors(self) -> Vec<TypeDiagnostic> {
        self.errors
    }

    fn simple_resolution(type_name: &str) -> bsl_shared::domain::types::TypeResolution {
        use bsl_shared::domain::types::{
            PrimitiveType, ResolutionMetadata, ResolutionSource, TypeResolution,
        };

        let result = match type_name {
            "Число" | "Number" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number))
            }
            "Строка" | "String" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String))
            }
            "Булево" | "Boolean" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean))
            }
            "Дата" | "Date" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Date))
            }
            _ => {
                use bsl_shared::domain::types::PlatformType;
                ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                    name: type_name.to_string(),
                }))
            }
        };

        TypeResolution {
            certainty: Certainty::Known,
            result,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }
}

impl<'a> SemanticVisitor for SemanticValidationVisitor<'a> {
    fn visit_node(&mut self, node: &SemanticNode, _context: &mut FlowContext) {
        match &node.kind {
            SemanticNodeKind::FunctionCall {
                function_name,
                object_type: Some(obj_type),
                ..
            } => {
                let resolution = Self::simple_resolution(obj_type);
                if let Some(error_kind) = self
                    .validator
                    .validate_method_exists(&resolution, function_name)
                {
                    let diagnostic =
                        error_kind.to_diagnostic(node.span.start_line, node.span.start_column);
                    self.errors.push(diagnostic);
                }
            }
            SemanticNodeKind::MemberAccess {
                object_type,
                member_name,
                is_method: false,
                ..
            } => {
                let resolution = Self::simple_resolution(object_type);
                if let Some(error_kind) = self
                    .validator
                    .validate_property_exists(&resolution, member_name)
                {
                    let diagnostic =
                        error_kind.to_diagnostic(node.span.start_line, node.span.start_column);
                    self.errors.push(diagnostic);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::ir::{SemanticNode, SemanticNodeKind, Span};

    #[test]
    fn test_visitor_detects_nonexistent_method() {
        use std::sync::Arc;
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository);
        let validator = TypeValidator::new(&metadata);
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name: "НесуществующийМетод".to_string(),
                object_name: Some("МассивДанных".to_string()),
                object_type: Some("Массив".to_string()),
                arg_types: vec![],
            },
            span: Span::new(5, 10, 5, 40),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        assert!(
            !errors.is_empty(),
            "Должна быть ошибка для несуществующего метода"
        );
        assert!(errors[0].message.contains("НесуществующийМетод"));
    }

    #[test]
    fn test_visitor_detects_nonexistent_property() {
        use std::sync::Arc;
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository);
        let validator = TypeValidator::new(&metadata);
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_name: Some("МассивДанных".to_string()),
                object_type: "Массив".to_string(),
                member_name: "НесуществующееСвойство".to_string(),
                is_method: false,
            },
            span: Span::new(3, 5, 3, 35),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        assert!(
            !errors.is_empty(),
            "Должна быть ошибка для несуществующего свойства"
        );
        assert!(errors[0].message.contains("НесуществующееСвойство"));
    }
}
