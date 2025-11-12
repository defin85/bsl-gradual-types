//! Semantic Validation Visitor
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::TypeDiagnostic;
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::ir::{
    FlowContext, SemanticNode, SemanticNodeKind, SemanticProgram, SemanticVisitor,
};

pub struct SemanticValidationVisitor<'a> {
    validator: &'a TypeValidator<'a>,
    resolver: &'a TypeResolver,
    errors: Vec<TypeDiagnostic>,
    #[allow(dead_code)]
    program: &'a SemanticProgram,
}

impl<'a> SemanticValidationVisitor<'a> {
    pub fn new(
        validator: &'a TypeValidator<'a>,
        resolver: &'a TypeResolver,
        program: &'a SemanticProgram,
    ) -> Self {
        Self {
            validator,
            resolver,
            errors: Vec::new(),
            program,
        }
    }

    pub fn into_errors(self) -> Vec<TypeDiagnostic> {
        self.errors
    }
}

impl<'a> SemanticVisitor for SemanticValidationVisitor<'a> {
    fn visit_node(&mut self, node: &SemanticNode, _context: &mut FlowContext) {
        match &node.kind {
            SemanticNodeKind::FunctionCall {
                function_name,
                object_type: Some(obj_type),
                method_span,
                ..
            } => {
                // Используем TypeResolver вместо simple_resolution для корректного определения active_facet
                let resolution = self.resolver.resolve_expression_sync(obj_type);
                if let Some(error_kind) = self
                    .validator
                    .validate_method_exists(&resolution, function_name)
                {
                    // ✅ MILESTONE 3.7: Используем method_span если есть для точного Diagnostic Range
                    let (line, column) = if let Some(m_span) = method_span {
                        (m_span.start_line, m_span.start_column)
                    } else {
                        // Fallback для обычных функций
                        (node.span.start_line, node.span.start_column)
                    };

                    let diagnostic = error_kind.to_diagnostic(line, column);
                    self.errors.push(diagnostic);
                }
            }
            SemanticNodeKind::MemberAccess {
                object_type,
                member_name,
                is_method: false,
                ..
            } => {
                // Используем TypeResolver для корректного определения active_facet
                let resolution = self.resolver.resolve_expression_sync(object_type);
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
    use std::sync::Arc;

    #[test]
    fn test_visitor_detects_nonexistent_method() {
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let metadata = TypeMetadataLookup::new(repository);
        let validator = TypeValidator::new(&metadata);
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name: "НесуществующийМетод".to_string(),
                object_name: Some("МассивДанных".to_string()),
                object_type: Some("Массив".to_string()),
                arg_types: vec![],
                method_span: None,  // Тест без method_span
            },
            span: Span::new(5, 10, 5, 40),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &resolver, &program);
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
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
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

        let mut visitor = SemanticValidationVisitor::new(&validator, &resolver, &program);
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
