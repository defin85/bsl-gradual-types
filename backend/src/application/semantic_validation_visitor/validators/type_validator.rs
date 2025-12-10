//! Type validation logic for semantic validation
//!
//! Contains validation of method existence, property existence,
//! and metadata object access.

use bsl_shared::domain::resolver::ValidationResultV2;
use bsl_shared::domain::types::{DiagnosticSeverity, TypeDiagnostic};
use bsl_shared::ir::Span;

/// Converts ValidationResultV2 to TypeDiagnostic (Milestone 3.13)
/// Uses object comparison of types with detailed incompatibility reasons
pub fn validation_result_v2_to_diagnostic(
    result: &ValidationResultV2,
    span: Span,
) -> Option<TypeDiagnostic> {
    match result {
        ValidationResultV2::Ok(_) => None,
        ValidationResultV2::NotFound => None, // Handled separately in validate_method_exists
        ValidationResultV2::MissingRequiredParam { param_name, param_index } => {
            Some(TypeDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "Отсутствует обязательный параметр '{}' (позиция {})",
                    param_name, param_index + 1
                ),
                line: span.start_line,
                column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            })
        }
        ValidationResultV2::TooManyArgs { expected, actual } => {
            Some(TypeDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "Слишком много аргументов: ожидается {}, передано {}",
                    expected, actual
                ),
                line: span.start_line,
                column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            })
        }
        ValidationResultV2::TypeMismatch { param_name, param_index, expected, actual, reason } => {
            let msg = if reason.is_empty() {
                format!(
                    "Параметр '{}' (позиция {}): ожидается {}, получено {}",
                    param_name, param_index + 1, expected, actual
                )
            } else {
                format!(
                    "Параметр '{}' (позиция {}): ожидается {}, получено {} ({})",
                    param_name, param_index + 1, expected, actual, reason
                )
            };
            Some(TypeDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: msg,
                line: span.start_line,
                column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            })
        }
    }
}
