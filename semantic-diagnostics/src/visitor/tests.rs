use super::*;
use crate::type_hints::SemanticTypeHints;
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::TypeRepository; // MILESTONE 3.16: Import trait for load_types
use bsl_shared::domain::types::{Certainty, FacetKind}; // Phase 4: Moved from main imports (used only in tests)
use bsl_shared::ir::{SemanticNode, SemanticNodeKind, Span};

#[test]
fn test_visitor_detects_nonexistent_method() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let call_span = Span::new(10, 40);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::FunctionCall {
            function_name: "НесуществующийМетод".to_string(),
            object_name: Some("МассивДанных".to_string()),
            object_node: None,
        },
        span: call_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints
        .call_receiver_type_by_span
        .insert(call_span, TypeResolution::explicit("Массив"));
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        !errors.is_empty(),
        "Should have error for non-existent method"
    );
    assert!(errors[0].message.contains("НесуществующийМетод"));
}

#[test]
fn test_visitor_detects_nonexistent_property() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let access_span = Span::new(5, 35);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: Some("МассивДанных".to_string()),
            member_name: "НесуществующееСвойство".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: access_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints
        .member_access_object_type_by_span
        .insert(access_span, TypeResolution::explicit("Массив"));
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        !errors.is_empty(),
        "Should have error for non-existent property"
    );
    assert!(errors[0].message.contains("НесуществующееСвойство"));
}

#[test]
fn test_dynamic_like_skips_nonexistent_method_validation() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let call_span = Span::new(10, 40);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::FunctionCall {
            function_name: "НесуществующийМетод".to_string(),
            object_name: Some("Объект".to_string()),
            object_node: None,
        },
        span: call_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints
        .call_receiver_type_by_span
        .insert(call_span, TypeResolution::explicit("Dynamic.Объект"));
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(errors.is_empty(), "expected no diagnostics for Dynamic.*");
}

#[test]
fn test_dynamic_like_skips_nonexistent_property_validation() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let access_span = Span::new(5, 35);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: Some("Объект".to_string()),
            member_name: "НесуществующееСвойство".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: access_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints
        .member_access_object_type_by_span
        .insert(access_span, TypeResolution::explicit("Dynamic.Объект"));
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(errors.is_empty(), "expected no diagnostics for Dynamic.*");
}

#[test]
fn test_unknown_type_access_is_warning_by_default() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let call_span = Span::new(10, 40);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::FunctionCall {
            function_name: "Метод".to_string(),
            object_name: Some("Объект".to_string()),
            object_node: None,
        },
        span: call_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints
        .call_receiver_type_by_span
        .insert(call_span, TypeResolution::unknown());
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    let diag = errors
        .iter()
        .find(|d| d.message.contains("Невозможно определить член"))
        .expect("expected UnknownTypeAccess diagnostic");
    assert_eq!(diag.severity, DiagnosticSeverity::Warning);
}

#[test]
fn test_unknown_type_access_suppressed_when_config_not_loaded() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let mut unknown = TypeResolution::unknown();
    unknown.metadata.uncertainty_reason = Some(UncertaintyReason::ConfigurationNotLoaded);

    let access_span = Span::new(5, 35);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: None,
            member_name: "Свойство".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: access_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints
        .member_access_object_type_by_span
        .insert(access_span, unknown);
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(errors.is_empty(), "expected graceful degradation");
}

#[test]
fn test_inferred_weak_method_validation_suppressed_when_config_not_loaded() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let mut inferred = TypeResolution::explicit("ДокументОбъект.Док1");
    inferred.certainty = Certainty::InferredWeak;
    inferred.metadata.uncertainty_reason = Some(UncertaintyReason::ConfigurationNotLoaded);

    let call_span = Span::new(10, 40);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::FunctionCall {
            function_name: "НесуществующийМетод".to_string(),
            object_name: Some("Объект".to_string()),
            object_node: None,
        },
        span: call_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints.call_receiver_type_by_span.insert(call_span, inferred);
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        errors.is_empty(),
        "expected no method existence diagnostics for InferredWeak + ConfigurationNotLoaded"
    );
}

#[test]
fn test_inferred_weak_property_validation_suppressed_when_config_not_loaded() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let mut inferred = TypeResolution::explicit("ДокументОбъект.Док1");
    inferred.certainty = Certainty::InferredWeak;
    inferred.metadata.uncertainty_reason = Some(UncertaintyReason::ConfigurationNotLoaded);

    let access_span = Span::new(5, 35);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: Some("Объект".to_string()),
            member_name: "НесуществующееСвойство".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: access_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints
        .member_access_object_type_by_span
        .insert(access_span, inferred);
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        errors.is_empty(),
        "expected no property existence diagnostics for InferredWeak + ConfigurationNotLoaded"
    );
}

#[test]
fn test_type_not_found_remains_error_on_unknown_member_access() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let mut unknown = TypeResolution::unknown();
    unknown.metadata.uncertainty_reason = Some(UncertaintyReason::TypeNotFound {
        name: "Foo".to_string(),
    });

    let call_span = Span::new(10, 40);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::FunctionCall {
            function_name: "Метод".to_string(),
            object_name: Some("Объект".to_string()),
            object_node: None,
        },
        span: call_span,
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut hints = SemanticTypeHints::default();
    hints.call_receiver_type_by_span.insert(call_span, unknown);
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(errors
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error));
}

// === MILESTONE 3.16: Metadata object validation tests ===

#[test]
fn test_visitor_validates_metadata_object_when_config_loaded() {
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::types::{MetadataKind, RawDataSource, RawTypeData};
    use std::sync::Arc;

    // Create repository with configuration types
    let repository = Arc::new(InMemoryTypeRepository::new());

    // Add catalog "Контрагенты"
    let catalog = RawTypeData {
        name: "Справочники.Контрагенты".to_string(),
        english_name: "Catalogs.Contractors".to_string(),
        description: "Справочник контрагентов".to_string(),
        category: "Справочники".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Manager, FacetKind::Object],
        kind: Some(MetadataKind::Catalog),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };
    repository.load_types(vec![catalog]).unwrap();

    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    // Test access to non-existent catalog
    // Справочники.НесуществующийСправочник
    // IMPORTANT: object_name should be Some("Справочники") - this is how it's formed in ast_to_ir.rs
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: Some("Справочники".to_string()),
            member_name: "НесуществующийСправочник".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: Span::new(0, 35),
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        !errors.is_empty(),
        "Should have error for non-existent catalog"
    );
    assert!(errors[0].message.contains("Справочник"));
    assert!(errors[0].message.contains("не найден"));
}

#[test]
fn test_visitor_no_error_for_existing_metadata_object() {
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::types::{MetadataKind, RawDataSource, RawTypeData};
    use std::sync::Arc;

    let repository = Arc::new(InMemoryTypeRepository::new());

    // Add catalog "Контрагенты"
    let catalog = RawTypeData {
        name: "Справочники.Контрагенты".to_string(),
        english_name: "Catalogs.Contractors".to_string(),
        description: "Справочник контрагентов".to_string(),
        category: "Справочники".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Manager, FacetKind::Object],
        kind: Some(MetadataKind::Catalog),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };
    repository.load_types(vec![catalog]).unwrap();

    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    // Test access to existing catalog
    // Справочники.Контрагенты
    // IMPORTANT: object_name should be Some("Справочники") - this is how it's formed in ast_to_ir.rs
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: Some("Справочники".to_string()),
            member_name: "Контрагенты".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: Span::new(0, 25),
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        errors.is_empty(),
        "Should have no errors for existing catalog"
    );
}

#[test]
fn test_visitor_no_error_when_config_not_loaded() {
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use std::sync::Arc;

    // Repository WITHOUT configuration types
    let repository = Arc::new(InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    // Test access to non-existent catalog
    // When config is not loaded, no error should appear (graceful degradation)
    // IMPORTANT: object_name should be Some("Справочники") to pass through is_metadata_collection_name
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: Some("Справочники".to_string()),
            member_name: "НесуществующийСправочник".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: Span::new(0, 35),
        scope_id: program.symbols.root_scope,
    });

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    // When config is not loaded, skip validation
    // But there may be "property doesn't exist" error for type "Справочники"
    // This is expected behavior - graceful degradation
    assert!(
        errors.is_empty() || !errors[0].message.contains("не найден в конфигурации"),
        "Should not have 'not found in configuration' error when config is not loaded"
    );
}

#[test]
fn test_visitor_hints_override_ir_types_for_method_call() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;

    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let call_span = Span::new(10, 40);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::FunctionCall {
            function_name: "НесуществующийМетод".to_string(),
            object_name: Some("МассивДанных".to_string()),
            object_node: None,
        },
        span: call_span,
        scope_id: program.symbols.root_scope,
    });

    let mut hints = crate::SemanticTypeHints::default();
    hints
        .call_receiver_type_by_span
        .insert(call_span, TypeResolution::explicit("Массив"));

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        !errors.is_empty(),
        "Should have error for non-existent method via hinted receiver type"
    );
    assert!(errors[0].message.contains("НесуществующийМетод"));
}

#[test]
fn test_visitor_hints_override_ir_types_for_member_access() {
    use bsl_shared::domain::types::TypeResolution;
    use std::sync::Arc;

    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata);
    let resolver = TypeResolver::new(repository);
    let signature_index = SignatureIndex::new();
    let mut program = SemanticProgram::new();

    let access_span = Span::new(5, 35);
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::MemberAccess {
            object_node: None,
            object_name: Some("МассивДанных".to_string()),
            member_name: "НесуществующееСвойство".to_string(),
            access_kind: MemberAccessKind::Property,
        },
        span: access_span,
        scope_id: program.symbols.root_scope,
    });

    let mut hints = crate::SemanticTypeHints::default();
    hints
        .member_access_object_type_by_span
        .insert(access_span, TypeResolution::explicit("Массив"));

    let mut visitor =
        SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
    visitor.set_type_hints(Some(&hints));
    let mut context = FlowContext::new(program.symbols.root_scope);
    visitor.visit_node(&program.nodes[0], &mut context);

    let errors = visitor.into_errors();
    assert!(
        !errors.is_empty(),
        "Should have error for non-existent property via hinted object type"
    );
    assert!(errors[0].message.contains("НесуществующееСвойство"));
}
