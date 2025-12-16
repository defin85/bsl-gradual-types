//! SemanticValidationVisitor - main visitor struct and implementation
//!
//! This module contains the SemanticValidationVisitor struct and its
//! implementation of the SemanticVisitor trait.

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{DiagnosticSeverity, TypeDiagnostic};
use bsl_shared::domain::validators::{TypeValidator, TypeErrorKind};
use bsl_shared::domain::RuntimeExecutionContext;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::{
    FlowContext, MemberAccessKind, SemanticNode, SemanticNodeKind, SemanticProgram, SemanticVisitor,
};

use crate::application::semantic_validation_visitor::helpers::{
    collection_name_to_metadata_kind, is_metadata_collection_name,
};
use crate::application::semantic_validation_visitor::validators::{
    validate_method_call_context, validation_result_v2_to_diagnostic,
};

/// Semantic validation visitor for BSL code
///
/// Visits semantic nodes and performs type validation,
/// method/property existence checks, and metadata object validation.
pub struct SemanticValidationVisitor<'a> {
    validator: &'a TypeValidator<'a>,
    resolver: &'a TypeResolver,
    signature_index: &'a SignatureIndex,
    errors: Vec<TypeDiagnostic>,
    #[allow(dead_code)]
    program: &'a SemanticProgram,
    detail_level: DetailLevel,
    current_execution_context: RuntimeExecutionContext,
}

impl<'a> SemanticValidationVisitor<'a> {
    /// Creates a new SemanticValidationVisitor
    pub fn new(
        validator: &'a TypeValidator<'a>,
        program: &'a SemanticProgram,
        resolver: &'a TypeResolver,
        signature_index: &'a SignatureIndex,
    ) -> Self {
        Self {
            validator,
            resolver,
            signature_index,
            errors: Vec::new(),
            program,
            detail_level: DetailLevel::Full,  // Default for backward compatibility
            current_execution_context: RuntimeExecutionContext::new(),
        }
    }

    /// MILESTONE 3.6 Phase 3: Create visitor with configurable detail level
    pub fn with_detail_level(
        validator: &'a TypeValidator<'a>,
        program: &'a SemanticProgram,
        resolver: &'a TypeResolver,
        signature_index: &'a SignatureIndex,
        detail_level: DetailLevel,
    ) -> Self {
        Self {
            validator,
            resolver,
            signature_index,
            errors: Vec::new(),
            program,
            detail_level,
            current_execution_context: RuntimeExecutionContext::new(),
        }
    }

    /// Consumes the visitor and returns collected errors
    pub fn into_errors(self) -> Vec<TypeDiagnostic> {
        self.errors
    }

    /// MILESTONE 3.16: Validates metadata collection member access
    ///
    /// Checks if the metadata object exists when accessing:
    /// `Справочники.Контрагенты`, `Документы.ЗаказПокупателя`, etc.
    ///
    /// # Parameters
    ///
    /// * `object_type` - object type (e.g., "Справочники")
    /// * `member_name` - member name (e.g., "Контрагенты")
    /// * `variable_name` - variable name (for diagnostics)
    ///
    /// # Returns
    ///
    /// `Some(TypeErrorKind)` if object not found, `None` otherwise
    fn validate_metadata_member_access(
        &self,
        object_type: &str,
        member_name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        // Check if object_type is a metadata collection
        if !is_metadata_collection_name(object_type) {
            return None;
        }

        // Get metadata kind
        let kind = collection_name_to_metadata_kind(object_type)?;

        // Use method from TypeValidator for validation
        self.validator.validate_metadata_object_exists(kind, member_name, variable_name)
    }

    /// Validates variable declaration position (Var) in function/procedure body
    ///
    /// In 1C, variable declarations must be placed at the beginning of the function/procedure,
    /// before any executable code.
    fn validate_var_declaration_position(&mut self, body: &[usize], function_name: &str) {
        let mut found_executable = false;

        for &node_idx in body {
            if node_idx >= self.program.nodes.len() {
                continue;
            }
            let node = &self.program.nodes[node_idx];
            match &node.kind {
                SemanticNodeKind::VariableDeclaration { name, .. } => {
                    if found_executable {
                        let error = TypeErrorKind::VarDeclarationAfterExecutable {
                            variable_name: name.clone(),
                            function_name: function_name.to_string(),
                        };
                        let diagnostic = error.to_diagnostic_with_detail(node.span, self.detail_level);
                        self.errors.push(diagnostic);
                    }
                }
                _ => {
                    found_executable = true;
                }
            }
        }
    }

    /// Checks if a variable is initialized in the context
    ///
    /// Returns Some(TypeErrorKind) if the variable is not initialized.
    /// Used for validating FunctionCall.object_name and MemberAccess.object_name.
    fn check_uninitialized_variable(
        &self,
        variable_name: &Option<String>,
        context: &FlowContext,
    ) -> Option<TypeErrorKind> {
        if let Some(var_name) = variable_name {
            // Check if the variable is initialized
            if !context.is_initialized(var_name) {
                // Check if the variable exists in context (declared)
                // If variable doesn't exist - it's UndeclaredVariable, not Uninitialized
                if context.get_variable_type(var_name).is_some() {
                    return Some(TypeErrorKind::UninitializedVariableUsage {
                        variable_name: var_name.clone(),
                    });
                }
            }
        }
        None
    }

    /// MILESTONE 5.5: Extracts metadata collection name from MemberAccess
    ///
    /// # Algorithm
    /// 1. If `object_name` is set - return it (legacy path)
    /// 2. If `object_node` points to GlobalPropertyAccess - extract name
    /// 3. Otherwise - None (not metadata)
    fn extract_collection_name_for_metadata(
        &self,
        object_name: &Option<String>,
        object_node: Option<usize>,
    ) -> Option<String> {
        // Legacy path: object_name already contains collection name
        if let Some(name) = object_name {
            return Some(name.clone());
        }

        // New path (MILESTONE 5.5): extract from GlobalPropertyAccess
        if let Some(idx) = object_node {
            if let Some(node) = self.program.nodes.get(idx) {
                if let SemanticNodeKind::GlobalPropertyAccess { name, .. } = &node.kind {
                    return Some(name.clone());
                }
            }
        }

        None
    }
}

impl<'a> SemanticVisitor for SemanticValidationVisitor<'a> {
    fn visit_node(&mut self, node: &SemanticNode, context: &mut FlowContext) {
        match &node.kind {
            // Context-Aware validation: update directive when entering function/procedure
            SemanticNodeKind::FunctionDeclaration { compiler_directive, name, body, .. } => {
                // Update runtime context based on directive from AST
                tracing::debug!(
                    "FunctionDeclaration '{}': compiler_directive = {:?}",
                    name, compiler_directive
                );
                if let Some(directive) = compiler_directive {
                    self.current_execution_context.current_directive = *directive;
                    self.current_execution_context.in_function = Some(name.clone());
                } else {
                    // No directive = Unknown context
                    self.current_execution_context.current_directive = bsl_shared::domain::CompilerDirective::Unknown;
                    self.current_execution_context.in_function = Some(name.clone());
                }
                // Validate variable declaration positions
                self.validate_var_declaration_position(body, name);
            }
            SemanticNodeKind::ProcedureDeclaration { compiler_directive, name, body, .. } => {
                // Update runtime context based on directive from AST
                tracing::debug!(
                    "ProcedureDeclaration '{}': compiler_directive = {:?}",
                    name, compiler_directive
                );
                if let Some(directive) = compiler_directive {
                    self.current_execution_context.current_directive = *directive;
                    self.current_execution_context.in_function = Some(name.clone());
                } else {
                    // No directive = Unknown context
                    self.current_execution_context.current_directive = bsl_shared::domain::CompilerDirective::Unknown;
                    self.current_execution_context.in_function = Some(name.clone());
                }
                // Validate variable declaration positions
                self.validate_var_declaration_position(body, name);
            }
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                // Phase 3: object_type is now TypeResolution
                object_type: Some(obj_type),
                arg_types,
                ..
            } => {
                // NEW: Check undeclared variables in arguments
                for (idx, arg_type) in arg_types.iter().enumerate() {
                    if let Some(var_name) = arg_type.is_undeclared_variable() {
                        let error_kind = TypeErrorKind::UndeclaredVariable {
                            variable_name: var_name.to_string(),
                            method_name: Some(function_name.clone()),
                            param_index: Some(idx + 1),
                        };
                        let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                        self.errors.push(diagnostic);
                    }
                }

                // Phase 4: Check undeclared variable in object_type
                // For call chains: `undeclared.Method1().Method2()`
                if let Some(var_name) = obj_type.is_undeclared_variable() {
                    let error_kind = TypeErrorKind::UndeclaredVariable {
                        variable_name: var_name.to_string(),
                        method_name: Some(function_name.clone()),
                        param_index: None,
                    };
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return; // No point continuing validation
                }

                // Check uninitialized variables (Warning, not Error)
                if let Some(error_kind) = self.check_uninitialized_variable(object_name, context) {
                    let diagnostic = error_kind.to_diagnostic_with_severity(
                        node.span,
                        self.detail_level,
                        DiagnosticSeverity::Warning
                    );
                    self.errors.push(diagnostic);
                    // DO NOT return - continue validation (Unknown type will be handled below)
                }

                // MILESTONE 5.1: Generate error for Unknown types
                if obj_type.is_unknown() {
                    let error_kind = TypeErrorKind::UnknownTypeAccess {
                        variable_name: object_name.clone(),
                        member_name: function_name.clone(),
                    };
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return;
                }

                // MILESTONE 5.3: Skip validation for Dynamic types
                // Dynamic means the previous method was not found and returned Dynamic.
                // Showing error "method doesn't exist for type Dynamic" is uninformative -
                // the real error was already shown earlier in the chain.
                if obj_type.is_dynamic() {
                    return;
                }

                // Phase 4: obj_type is already TypeResolution - use directly
                // metadata_lookup.get_methods() already handles Generic and facets correctly

                // 1. MILESTONE 3.6 Phase 3: Check method existence with variable_name
                if let Some(error_kind) = self
                    .validator
                    .validate_method_exists_with_variable(
                        obj_type,  // Phase 4: Direct use of TypeResolution
                        function_name,
                        object_name.clone(),  // Pass variable name
                    )
                {
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return; // No point checking parameters if method doesn't exist
                }

                // 1.5. MILESTONE 3.11 Phase 3: Check method availability in current context
                // Phase 3: Pass type_name() instead of String
                if let Some(error_kind) = validate_method_call_context(
                    &self.current_execution_context,
                    self.signature_index,
                    &obj_type.type_name(),
                    function_name,
                    object_name.clone(),
                    node.span,
                ) {
                    // Context warnings use WARNING severity, not Error
                    let diagnostic = error_kind.to_diagnostic_with_severity(
                        node.span,
                        self.detail_level,
                        DiagnosticSeverity::Warning
                    );
                    self.errors.push(diagnostic);
                    // DO NOT return - continue parameter checking
                }

                // 2. MILESTONE 3.13: Check parameter types with object comparison (v2)
                // Phase 3: Convert Vec<TypeResolution> -> Vec<String> for validate_call_v2
                let arg_types_str: Vec<String> = arg_types.iter()
                    .map(|tr| tr.type_name())
                    .collect();

                let validation_result = self.resolver.validate_call_v2(
                    Some(&obj_type.type_name()),
                    function_name,
                    &arg_types_str,
                    self.signature_index,
                );

                // Convert ValidationResultV2 to TypeDiagnostic
                if let Some(diagnostic) = validation_result_v2_to_diagnostic(&validation_result, node.span) {
                    self.errors.push(diagnostic);
                }
            }
            SemanticNodeKind::MemberAccess {
                object_node,  // MILESTONE 5.5: added for extracting name from GlobalPropertyAccess
                object_name,
                object_type,
                member_name,
                access_kind: MemberAccessKind::Property,
                ..
            } => {
                // MILESTONE 5.5 Fix: Extract collection name considering object_node
                let collection_name = self.extract_collection_name_for_metadata(object_name, *object_node);

                tracing::debug!(
                    "MemberAccess: collection_name={:?}, object_type={}, member_name={}",
                    collection_name, object_type.type_name(), member_name
                );

                if let Some(ref name) = collection_name {
                    tracing::debug!("Checking if '{}' is metadata collection: {}", name, is_metadata_collection_name(name));
                    if is_metadata_collection_name(name) {
                        // This is access to metadata collection - validate object
                        if let Some(error_kind) = self.validate_metadata_member_access(
                            name,
                            member_name,
                            collection_name.clone(),
                        ) {
                            let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                            self.errors.push(diagnostic);
                        }
                        // Regardless of result, don't check properties for metadata collections
                        // because this is not a regular property access
                        return;
                    }
                }

                // Phase 4: object_type is already TypeResolution - use directly
                // metadata_lookup.get_properties() already handles Generic and facets correctly

                // Check uninitialized variables (Warning, not Error)
                if let Some(error_kind) = self.check_uninitialized_variable(object_name, context) {
                    let diagnostic = error_kind.to_diagnostic_with_severity(
                        node.span,
                        self.detail_level,
                        DiagnosticSeverity::Warning
                    );
                    self.errors.push(diagnostic);
                    // DO NOT return - continue validation (Unknown type will be handled below)
                }

                // MILESTONE 5.1: Generate error for Unknown types
                if object_type.is_unknown() {
                    let error_kind = TypeErrorKind::UnknownTypeAccess {
                        variable_name: object_name.clone(),
                        member_name: member_name.clone(),
                    };
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return;
                }

                // MILESTONE 3.6 Phase 3: Pass variable name
                if let Some(error_kind) = self
                    .validator
                    .validate_property_exists_with_variable(
                        object_type,  // Phase 4: Direct use of TypeResolution
                        member_name,
                        object_name.clone(),  // Pass variable name
                    )
                {
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
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
    use bsl_shared::domain::repository::TypeRepository;  // MILESTONE 3.16: Import trait for load_types
    use bsl_shared::domain::types::FacetKind;  // Phase 4: Moved from main imports (used only in tests)
    use bsl_shared::ir::{SemanticNode, SemanticNodeKind, Span};

    #[test]
    fn test_visitor_detects_nonexistent_method() {
        use std::sync::Arc;
        use bsl_shared::domain::types::TypeResolution;
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name: "НесуществующийМетод".to_string(),
                object_name: Some("МассивДанных".to_string()),
                // Phase 3: object_type is now TypeResolution
                object_type: Some(TypeResolution::explicit("Массив")),
                // Phase 3: arg_types is now Vec<TypeResolution>
                arg_types: vec![],
                object_node: None,
                result_type: TypeResolution::unknown(),
            },
            span: Span::new(5, 10, 5, 40),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
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
        use std::sync::Arc;
        use bsl_shared::domain::types::TypeResolution;
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_node: None,
                object_name: Some("МассивДанных".to_string()),
                // Phase 3: object_type is now TypeResolution
                object_type: TypeResolution::explicit("Массив"),
                member_name: "НесуществующееСвойство".to_string(),
                access_kind: MemberAccessKind::Property,
                result_type: TypeResolution::unknown(),
            },
            span: Span::new(3, 5, 3, 35),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        assert!(
            !errors.is_empty(),
            "Should have error for non-existent property"
        );
        assert!(errors[0].message.contains("НесуществующееСвойство"));
    }

    // === MILESTONE 3.16: Metadata object validation tests ===

    #[test]
    fn test_visitor_validates_metadata_object_when_config_loaded() {
        use std::sync::Arc;
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        use bsl_shared::domain::types::{RawTypeData, RawDataSource, TypeResolution, MetadataKind};

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
                // Phase 3: object_type is now TypeResolution
                object_type: TypeResolution::explicit("СправочникМенеджер"),
                member_name: "НесуществующийСправочник".to_string(),
                access_kind: MemberAccessKind::Property,
                result_type: TypeResolution::unknown(),
            },
            span: Span::new(1, 0, 1, 35),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
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
        use std::sync::Arc;
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        use bsl_shared::domain::types::{RawTypeData, RawDataSource, TypeResolution, MetadataKind};

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
                // Phase 3: object_type is now TypeResolution
                object_type: TypeResolution::explicit("СправочникМенеджер"),
                member_name: "Контрагенты".to_string(),
                access_kind: MemberAccessKind::Property,
                result_type: TypeResolution::unknown(),
            },
            span: Span::new(1, 0, 1, 25),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
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
        use std::sync::Arc;
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        use bsl_shared::domain::types::TypeResolution;

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
                // Phase 3: object_type is now TypeResolution
                object_type: TypeResolution::explicit("СправочникМенеджер"),
                member_name: "НесуществующийСправочник".to_string(),
                access_kind: MemberAccessKind::Property,
                result_type: TypeResolution::unknown(),
            },
            span: Span::new(1, 0, 1, 35),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
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
}
