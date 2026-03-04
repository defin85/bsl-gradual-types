//! Diagnostic converters for BSL -> LSP conversion
//!
//! This module converts ParseError and TypeDiagnostic from BSL shared types
//! to LSP Diagnostic format for display in VSCode.

use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    Position, Range, Url,
};

use bsl_line_index::LineIndex;
use bsl_shared::domain::types::DiagnosticSeverity as SharedSeverity;
use bsl_shared::domain::types::{ErrorType, ParseError, TypeDiagnostic};

/// Convert syntax errors to LSP Diagnostics (Milestone 2.18)
///
/// Transforms ParseError from parser to LSP Diagnostic for display in VSCode.
pub fn syntax_errors_to_diagnostics(
    errors: &[ParseError],
    uri: &Url,
    source: &str,
    line_index: &LineIndex,
) -> Vec<Diagnostic> {
    let pos = |offset: u32| {
        let (line, character) = line_index.byte_offset_to_utf16_position(source, offset as usize);
        Position::new(line, character)
    };

    errors
        .iter()
        .map(|error| {
            let severity = match error.error_type {
                ErrorType::ParseError | ErrorType::InvalidSyntax => DiagnosticSeverity::ERROR,
                ErrorType::MissingToken => DiagnosticSeverity::ERROR,
                ErrorType::UnexpectedToken => DiagnosticSeverity::WARNING,
            };

            let related_information = if error.related.is_empty() {
                None
            } else {
                Some(
                    error
                        .related
                        .iter()
                        .map(|related| DiagnosticRelatedInformation {
                            location: Location {
                                uri: uri.clone(),
                                range: Range::new(pos(related.span.start), pos(related.span.end)),
                            },
                            message: related.message.clone(),
                        })
                        .collect(),
                )
            };

            Diagnostic {
                range: Range::new(pos(error.span.start), pos(error.span.end)),
                severity: Some(severity),
                message: error.message.clone(),
                source: Some("bsl-syntax".to_string()),
                code: Some(NumberOrString::String(format!("{:?}", error.error_type))),
                related_information,
                ..Default::default()
            }
        })
        .collect()
}

/// Convert TypeDiagnostic to LSP Diagnostic
///
/// # Milestone 3.7: Semantic Diagnostics MVP
pub fn semantic_error_to_diagnostic(
    error: &TypeDiagnostic,
    source: &str,
    line_index: &LineIndex,
) -> Diagnostic {
    let start_pos = {
        let (line, character) =
            line_index.byte_offset_to_utf16_position(source, error.span.start as usize);
        Position::new(line, character)
    };
    let end_pos = {
        let (line, character) =
            line_index.byte_offset_to_utf16_position(source, error.span.end as usize);
        Position::new(line, character)
    };

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
        source: Some("bsl-analysis-v2".to_string()), // Different from "bsl-syntax"
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "diagnostics/tests.rs"]
mod tests;
