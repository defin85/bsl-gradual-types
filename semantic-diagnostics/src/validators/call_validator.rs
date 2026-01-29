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
        let mut requirements = signature.context_requirements;
        if requirements == bsl_shared::domain::ContextRequirements::Universal {
            if let Some(inferred) = infer_context_requirements(method_name) {
                requirements = inferred;
            }
        }
        // Check method availability in current context
        if !current_execution_context.can_call_method(&requirements) {
            return Some(TypeErrorKind::MethodNotAvailableInContext {
                method_name: method_name.to_string(),
                object_type: receiver_type.to_string(),
                variable_name,
                current_context: current_execution_context.current_directive, // Type-safe
                required_context: requirements,                               // Type-safe
            });
        }
        return None;
    }

    if let Some(requirements) = infer_context_requirements(method_name) {
        if !current_execution_context.can_call_method(&requirements) {
            return Some(TypeErrorKind::MethodNotAvailableInContext {
                method_name: method_name.to_string(),
                object_type: receiver_type.to_string(),
                variable_name,
                current_context: current_execution_context.current_directive,
                required_context: requirements,
            });
        }
    }
    None
}

/// Validates global function availability in the current runtime context.
pub fn validate_global_function_call_context(
    current_execution_context: &RuntimeExecutionContext,
    signature_index: &SignatureIndex,
    function_name: &str,
) -> Option<TypeErrorKind> {
    if let Some(signature) = signature_index.find_global_function(function_name) {
        if !current_execution_context.can_call_method(&signature.context_requirements) {
            return Some(TypeErrorKind::MethodNotAvailableInContext {
                method_name: function_name.to_string(),
                object_type: "Глобальный контекст".to_string(),
                variable_name: None,
                current_context: current_execution_context.current_directive,
                required_context: signature.context_requirements,
            });
        }
    }

    None
}

fn infer_context_requirements(
    method_name: &str,
) -> Option<bsl_shared::domain::ContextRequirements> {
    use bsl_shared::domain::ContextRequirements;

    let lower = method_name.to_lowercase();

    if lower.starts_with("создать")
        || lower.starts_with("create")
        || lower == "скопировать"
        || lower == "copy"
        || lower.starts_with("найтипо")
        || lower.starts_with("findby")
        || lower == "найти"
        || lower == "find"
        || lower == "выбрать"
        || lower == "select"
        || lower == "записать"
        || lower == "write"
        || lower == "провести"
        || lower == "post"
        || lower == "отменитьпроведение"
        || lower == "unpost"
        || lower == "удалить"
        || lower == "delete"
        || lower == "получитьобъект"
        || lower == "getobject"
    {
        return Some(ContextRequirements::ServerOnly);
    }

    if lower == "пустая" || lower == "isempty" || lower == "пустаяссылка" || lower == "emptyref"
    {
        return Some(ContextRequirements::Universal);
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
            span,
        }),
        ValidationResult::TooManyArgs { expected, actual } => Some(TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message: format!(
                "Слишком много параметров: ожидается {}, получено {}",
                expected, actual
            ),
            span,
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
            span,
        }),
    }
}
