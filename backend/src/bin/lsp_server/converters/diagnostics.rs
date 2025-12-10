//! Diagnostic converters for BSL -> LSP conversion
//!
//! This module converts ParseError and TypeDiagnostic from BSL shared types
//! to LSP Diagnostic format for display in VSCode.

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

use bsl_shared::domain::types::{ErrorType, ParseError, TypeDiagnostic};
use bsl_shared::domain::types::DiagnosticSeverity as SharedSeverity;

/// Convert syntax errors to LSP Diagnostics (Milestone 2.18)
///
/// Transforms ParseError from parser to LSP Diagnostic for display in VSCode.
/// Error coordinates are already in UTF-16 thanks to Task 1 (Milestone 2.18).
pub fn syntax_errors_to_diagnostics(errors: &[ParseError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| {
            let severity = match error.error_type {
                ErrorType::ParseError | ErrorType::InvalidSyntax => DiagnosticSeverity::ERROR,
                ErrorType::MissingToken => DiagnosticSeverity::ERROR,
                ErrorType::UnexpectedToken => DiagnosticSeverity::WARNING,
            };

            Diagnostic {
                range: Range::new(
                    Position::new(error.span.start_line, error.span.start_column),
                    Position::new(error.span.end_line, error.span.end_column),
                ),
                severity: Some(severity),
                message: error.message.clone(),
                source: Some("bsl-syntax".to_string()),
                code: Some(NumberOrString::String(format!("{:?}", error.error_type))),
                ..Default::default()
            }
        })
        .collect()
}

/// Convert TypeDiagnostic to LSP Diagnostic
///
/// # Milestone 3.7: Semantic Diagnostics MVP
pub fn semantic_error_to_diagnostic(error: &TypeDiagnostic) -> Diagnostic {
    let start_pos = Position::new(error.line, error.column);
    let end_pos = Position::new(error.end_line, error.end_column);

    let severity = match error.severity {
        SharedSeverity::Error => Some(DiagnosticSeverity::ERROR),
        SharedSeverity::Warning => Some(DiagnosticSeverity::WARNING),
        SharedSeverity::Info => Some(DiagnosticSeverity::INFORMATION),
        SharedSeverity::Hint => Some(DiagnosticSeverity::HINT),
    };

    Diagnostic {
        range: Range::new(start_pos, end_pos),
        severity,
        message: error.message.clone(),
        source: Some("bsl-semantic".to_string()), // Different from "bsl-syntax"
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::ir::Span;

    #[test]
    fn test_syntax_error_to_diagnostic() {
        let errors = vec![ParseError {
            message: "Unexpected token".to_string(),
            error_type: ErrorType::UnexpectedToken,
            span: Span {
                start_line: 1,
                start_column: 5,
                end_line: 1,
                end_column: 10,
            },
        }];

        let diagnostics = syntax_errors_to_diagnostics(&errors);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Unexpected token");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(diagnostics[0].source, Some("bsl-syntax".to_string()));
    }

    #[test]
    fn test_semantic_error_to_diagnostic() {
        let error = TypeDiagnostic {
            message: "Type mismatch".to_string(),
            severity: SharedSeverity::Error,
            line: 10,
            column: 5,
            end_line: 10,
            end_column: 15,
        };

        let diagnostic = semantic_error_to_diagnostic(&error);

        assert_eq!(diagnostic.message, "Type mismatch");
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source, Some("bsl-semantic".to_string()));
    }
}
