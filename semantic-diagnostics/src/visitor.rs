//! SemanticValidationVisitor - main visitor struct and implementation
//!
//! This module contains the SemanticValidationVisitor struct and its
//! implementation of the SemanticVisitor trait.

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{
    Certainty, DiagnosticSeverity, TypeDiagnostic, TypeResolution, UncertaintyReason,
};
use bsl_shared::domain::validators::{TypeErrorKind, TypeValidator};
use bsl_shared::domain::RuntimeExecutionContext;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::{
    FlowContext, MemberAccessKind, SemanticNode, SemanticNodeKind, SemanticProgram, SemanticVisitor,
};

use crate::helpers::{collection_name_to_metadata_kind, is_metadata_collection_name};
use crate::type_hints::SemanticTypeHints;
use crate::validators::{
    validate_global_function_call_context, validate_method_call_context,
    validation_result_v2_to_diagnostic,
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
    platform_signatures_loaded: bool,
    type_hints: Option<&'a SemanticTypeHints>,
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
            detail_level: DetailLevel::Full, // Default for backward compatibility
            current_execution_context: RuntimeExecutionContext::new(),
            platform_signatures_loaded: false,
            type_hints: None,
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
            platform_signatures_loaded: false,
            type_hints: None,
        }
    }

    pub fn set_platform_signatures_loaded(&mut self, loaded: bool) {
        self.platform_signatures_loaded = loaded;
    }

    pub fn set_type_hints(&mut self, hints: Option<&'a SemanticTypeHints>) {
        self.type_hints = hints;
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
        self.validator
            .validate_metadata_object_exists(kind, member_name, variable_name)
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
                        let diagnostic =
                            error.to_diagnostic_with_detail(node.span, self.detail_level);
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
                if context.is_declared(var_name) {
                    return Some(TypeErrorKind::UninitializedVariableUsage {
                        variable_name: var_name.clone(),
                    });
                }
            }
        }
        None
    }

    /// Graceful degradation for contextual types when configuration metadata is unavailable.
    ///
    /// Some descriptor-based contextual resolutions return a concrete fallback type with
    /// `InferredWeak + ConfigurationNotLoaded`. In this state member existence checks are
    /// not reliable and must be suppressed, unless contextual descriptor notes provide
    /// an explicit member contract (e.g. `FormModule.Объект`).
    fn should_skip_member_validation_for_missing_configuration(
        type_resolution: &TypeResolution,
    ) -> bool {
        let missing_configuration = matches!(type_resolution.certainty, Certainty::InferredWeak)
            && matches!(
                type_resolution.metadata.uncertainty_reason,
                Some(UncertaintyReason::ConfigurationNotLoaded)
            );
        if !missing_configuration {
            return false;
        }

        let has_contextual_contract = type_resolution
            .metadata
            .notes
            .iter()
            .any(|note| note.starts_with("contextual:"));

        !has_contextual_contract
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
                if let SemanticNodeKind::GlobalPropertyAccess { name } = &node.kind {
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
            SemanticNodeKind::FunctionDeclaration {
                compiler_directive,
                name,
                body,
                ..
            } => {
                // Update runtime context based on directive from AST
                tracing::debug!(
                    "FunctionDeclaration '{}': compiler_directive = {:?}",
                    name,
                    compiler_directive
                );
                if let Some(directive) = compiler_directive {
                    self.current_execution_context.current_directive = *directive;
                    self.current_execution_context.in_function = Some(name.clone());
                } else {
                    // No directive = Unknown context
                    self.current_execution_context.current_directive =
                        bsl_shared::domain::CompilerDirective::Unknown;
                    self.current_execution_context.in_function = Some(name.clone());
                }
                // Validate variable declaration positions
                self.validate_var_declaration_position(body, name);
            }
            SemanticNodeKind::ProcedureDeclaration {
                compiler_directive,
                name,
                body,
                ..
            } => {
                // Update runtime context based on directive from AST
                tracing::debug!(
                    "ProcedureDeclaration '{}': compiler_directive = {:?}",
                    name,
                    compiler_directive
                );
                if let Some(directive) = compiler_directive {
                    self.current_execution_context.current_directive = *directive;
                    self.current_execution_context.in_function = Some(name.clone());
                } else {
                    // No directive = Unknown context
                    self.current_execution_context.current_directive =
                        bsl_shared::domain::CompilerDirective::Unknown;
                    self.current_execution_context.in_function = Some(name.clone());
                }
                // Validate variable declaration positions
                self.validate_var_declaration_position(body, name);
            }
            SemanticNodeKind::Assignment { variable, .. } => {
                let Some(value_type) = self
                    .type_hints
                    .and_then(|hints| hints.assignment_value_type(node.span))
                else {
                    return;
                };
                // Если тип значения помечен как Unknown с конкретной причиной (например, TypeNotFound),
                // генерируем диагностическую ошибку на месте присваивания.
                if let Some(mut kind) = self.validator.validate_from_resolution(value_type) {
                    if let TypeErrorKind::UnknownType {
                        ref mut variable_name,
                        ..
                    } = kind
                    {
                        *variable_name = Some(variable.clone());
                    }
                    let diagnostic = kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                }
            }
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                object_node,
                ..
            } => {
                let arg_types: &[bsl_shared::domain::types::TypeResolution] = self
                    .type_hints
                    .and_then(|hints| hints.call_arg_types(node.span))
                    .unwrap_or(&[]);

                // NEW: Check undeclared variables in arguments
                for (idx, arg_type) in arg_types.iter().enumerate() {
                    if let Some(var_name) = arg_type.is_undeclared_variable() {
                        let error_kind = TypeErrorKind::UndeclaredVariable {
                            variable_name: var_name.to_string(),
                            method_name: Some(function_name.clone()),
                            param_index: Some(idx + 1),
                        };
                        let diagnostic =
                            error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                        self.errors.push(diagnostic);
                    }
                }

                let is_method_call = object_name.is_some() || object_node.is_some();

                if is_method_call {
                    // Check uninitialized variables (Warning, not Error)
                    if let Some(error_kind) =
                        self.check_uninitialized_variable(object_name, context)
                    {
                        let diagnostic = error_kind.to_diagnostic_with_severity(
                            node.span,
                            self.detail_level,
                            DiagnosticSeverity::Warning,
                        );
                        self.errors.push(diagnostic);
                        // DO NOT return - continue validation (Unknown type will be handled below)
                    }

                    let Some(obj_type) = self
                        .type_hints
                        .and_then(|hints| hints.call_receiver_type(node.span))
                    else {
                        // Без type hints (v2) мы не можем валидировать методы/параметры.
                        return;
                    };

                    // For call chains: `undeclared.Method1().Method2()`
                    if let Some(var_name) = obj_type.is_undeclared_variable() {
                        let error_kind = TypeErrorKind::UndeclaredVariable {
                            variable_name: var_name.to_string(),
                            method_name: Some(function_name.clone()),
                            param_index: None,
                        };
                        let diagnostic =
                            error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                        self.errors.push(diagnostic);
                        return;
                    }

                    // MILESTONE 5.1: Generate error for Unknown types
                    if obj_type.is_unknown() {
                        if matches!(
                            obj_type.metadata.uncertainty_reason,
                            Some(UncertaintyReason::ConfigurationNotLoaded)
                        ) {
                            return;
                        }
                        if let Some(mut kind) = self.validator.validate_from_resolution(obj_type) {
                            if let TypeErrorKind::UnknownType {
                                ref mut variable_name,
                                ..
                            } = kind
                            {
                                *variable_name = object_name.clone();
                            }
                            let diagnostic =
                                kind.to_diagnostic_with_detail(node.span, self.detail_level);
                            self.errors.push(diagnostic);
                            return;
                        }
                        let error_kind = TypeErrorKind::UnknownTypeAccess {
                            variable_name: object_name.clone(),
                            member_name: function_name.clone(),
                        };
                        let diagnostic = error_kind.to_diagnostic_with_severity(
                            node.span,
                            self.detail_level,
                            DiagnosticSeverity::Warning,
                        );
                        self.errors.push(diagnostic);
                        return;
                    }

                    // Skip validation for Dynamic-like types (Dynamic / Dynamic.*).
                    if obj_type.is_dynamic() {
                        return;
                    }

                    if Self::should_skip_member_validation_for_missing_configuration(obj_type) {
                        return;
                    }

                    // 1. Check method existence with variable_name
                    if let Some(error_kind) = self.validator.validate_method_exists_with_variable(
                        obj_type,
                        function_name,
                        object_name.clone(),
                    ) {
                        let diagnostic =
                            error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                        self.errors.push(diagnostic);
                        return;
                    }

                    // 1.5. Check method availability in current context
                    if let Some(error_kind) = validate_method_call_context(
                        &self.current_execution_context,
                        self.signature_index,
                        &obj_type.type_name(),
                        function_name,
                        object_name.clone(),
                        node.span,
                    ) {
                        let diagnostic = error_kind.to_diagnostic_with_severity(
                            node.span,
                            self.detail_level,
                            DiagnosticSeverity::Warning,
                        );
                        self.errors.push(diagnostic);
                    }

                    // 2. Check parameter types with object comparison (v2)
                    let arg_types_str: Vec<String> =
                        arg_types.iter().map(|tr| tr.type_name()).collect();

                    let validation_result = self.resolver.validate_call_v2(
                        Some(&obj_type.type_name()),
                        function_name,
                        &arg_types_str,
                        self.signature_index,
                    );

                    if let Some(diagnostic) =
                        validation_result_v2_to_diagnostic(&validation_result, node.span)
                    {
                        self.errors.push(diagnostic);
                    }
                } else {
                    // Глобальный вызов.
                    if self.platform_signatures_loaded {
                        let is_known = self
                            .signature_index
                            .find_global_function(function_name)
                            .is_some()
                            || self.program.symbols.find_function(function_name).is_some()
                            || self.program.symbols.find_procedure(function_name).is_some();

                        if !is_known {
                            let error_kind = TypeErrorKind::UndefinedFunctionOrProcedure {
                                name: function_name.clone(),
                            };
                            let diagnostic =
                                error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                            self.errors.push(diagnostic);
                        }
                    }

                    if let Some(error_kind) = validate_global_function_call_context(
                        &self.current_execution_context,
                        self.signature_index,
                        function_name,
                    ) {
                        let diagnostic = error_kind.to_diagnostic_with_severity(
                            node.span,
                            self.detail_level,
                            DiagnosticSeverity::Warning,
                        );
                        self.errors.push(diagnostic);
                    }
                }
            }
            SemanticNodeKind::MemberAccess {
                object_node, // MILESTONE 5.5: added for extracting name from GlobalPropertyAccess
                object_name,
                member_name,
                access_kind: MemberAccessKind::Property,
                ..
            } => {
                let object_type_opt = self
                    .type_hints
                    .and_then(|hints| hints.member_access_object_type(node.span));
                // MILESTONE 5.5 Fix: Extract collection name considering object_node
                let collection_name =
                    self.extract_collection_name_for_metadata(object_name, *object_node);

                if let Some(object_type) = object_type_opt {
                    tracing::debug!(
                        "MemberAccess: collection_name={:?}, object_type={}, member_name={}",
                        collection_name,
                        object_type.type_name(),
                        member_name
                    );
                } else {
                    tracing::debug!(
                        "MemberAccess: collection_name={:?}, object_type=<none>, member_name={}",
                        collection_name,
                        member_name
                    );
                }

                if let Some(ref name) = collection_name {
                    tracing::debug!(
                        "Checking if '{}' is metadata collection: {}",
                        name,
                        is_metadata_collection_name(name)
                    );
                    if is_metadata_collection_name(name) {
                        // This is access to metadata collection - validate object
                        if let Some(error_kind) = self.validate_metadata_member_access(
                            name,
                            member_name,
                            collection_name.clone(),
                        ) {
                            let diagnostic =
                                error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                            self.errors.push(diagnostic);
                        }
                        // Regardless of result, don't check properties for metadata collections
                        // because this is not a regular property access
                        return;
                    }
                }

                // Check uninitialized variables (Warning, not Error)
                if let Some(error_kind) = self.check_uninitialized_variable(object_name, context) {
                    let diagnostic = error_kind.to_diagnostic_with_severity(
                        node.span,
                        self.detail_level,
                        DiagnosticSeverity::Warning,
                    );
                    self.errors.push(diagnostic);
                    // DO NOT return - continue validation (Unknown type will be handled below)
                }

                let Some(object_type) = object_type_opt else {
                    return;
                };

                // MILESTONE 5.1: Generate error for Unknown types
                if object_type.is_unknown() {
                    if let Some(var_name) = object_type.is_undeclared_variable() {
                        let error_kind = TypeErrorKind::UndeclaredVariable {
                            variable_name: var_name.to_string(),
                            method_name: None,
                            param_index: None,
                        };
                        let diagnostic =
                            error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                        self.errors.push(diagnostic);
                        return;
                    }
                    if matches!(
                        object_type.metadata.uncertainty_reason,
                        Some(UncertaintyReason::ConfigurationNotLoaded)
                    ) {
                        return;
                    }
                    if let Some(mut kind) = self.validator.validate_from_resolution(object_type) {
                        if let TypeErrorKind::UnknownType {
                            ref mut variable_name,
                            ..
                        } = kind
                        {
                            *variable_name = object_name.clone();
                        }
                        let diagnostic =
                            kind.to_diagnostic_with_detail(node.span, self.detail_level);
                        self.errors.push(diagnostic);
                        return;
                    }
                    let error_kind = TypeErrorKind::UnknownTypeAccess {
                        variable_name: object_name.clone(),
                        member_name: member_name.clone(),
                    };
                    let diagnostic = error_kind.to_diagnostic_with_severity(
                        node.span,
                        self.detail_level,
                        DiagnosticSeverity::Warning,
                    );
                    self.errors.push(diagnostic);
                    return;
                }

                // Skip validation for Dynamic-like types (Dynamic / Dynamic.*).
                if object_type.is_dynamic() {
                    return;
                }

                if Self::should_skip_member_validation_for_missing_configuration(object_type) {
                    return;
                }

                // MILESTONE 3.6 Phase 3: Pass variable name
                if let Some(error_kind) = self.validator.validate_property_exists_with_variable(
                    object_type, // Phase 4: Direct use of TypeResolution
                    member_name,
                    object_name.clone(), // Pass variable name
                ) {
                    let diagnostic =
                        error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
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
}
