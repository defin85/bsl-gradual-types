//! Call validation logic for semantic validation
//!
//! Contains validation of method call context and parameters.

use bsl_shared::domain::resolver::ValidationResult;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{DiagnosticSeverity, TypeDiagnostic};
use bsl_shared::domain::validators::TypeErrorKind;
use bsl_shared::domain::RuntimeExecutionContext;
use bsl_shared::ir::Span;

/// Validates method availability in the current runtime context.
/// Returns Some(TypeErrorKind) if the method is not available in the current context.
///
/// MILESTONE 3.11 Phase 3
pub fn validate_method_call_context(
    current_execution_context: &RuntimeExecutionContext,
    signature_index: &SignatureIndex,
    receiver_type: &str,
    method_name: &str,
    variable_name: Option<String>,
    _span: Span,
) -> Option<TypeErrorKind> {
    // Find method in SignatureIndex
    if let Some(signature) = signature_index.find_method(receiver_type, method_name) {
        // Check method availability in current context
        if !current_execution_context.can_call_method(&signature.context_requirements) {
            return Some(TypeErrorKind::MethodNotAvailableInContext {
                method_name: method_name.to_string(),
                object_type: receiver_type.to_string(),
                variable_name,
                current_context: current_execution_context.current_directive,  // Type-safe
                required_context: signature.context_requirements,               // Type-safe
            });
        }
    }
    None
}

/// Converts ValidationResult to TypeDiagnostic (Milestone 3.10)
/// TODO: Use in future for detailed parameter diagnostics
#[allow(dead_code)]
pub fn validation_result_to_diagnostic(
    result: &ValidationResult,
    span: Span,
) -> Option<TypeDiagnostic> {
    match result {
        ValidationResult::Ok(_) => None,
        ValidationResult::NotFound => None, // Already handled in validate_method_exists
        ValidationResult::MissingRequiredParam {
            param_name,
            param_index,
        } => Some(TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message: format!(
                "Недостаточно параметров: отсутствует обязательный параметр #{} '{}'",
                param_index + 1,
                param_name
            ),
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }),
        ValidationResult::TooManyArgs { expected, actual } => Some(TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message: format!(
                "Слишком много параметров: ожидается {}, получено {}",
                expected, actual
            ),
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }),
        ValidationResult::TypeMismatch {
            param_name,
            expected,
            actual,
        } => Some(TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message: format!(
                "Некорректный тип параметра '{}': ожидается {}, получено {}",
                param_name, expected, actual
            ),
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }),
    }
}
