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
mod tests {
    use super::*;
    use bsl_shared::domain::types::RelatedInformation;
    use bsl_shared::ir::Span;

    #[test]
    fn test_syntax_error_to_diagnostic() {
        let source = "0123456789";
        let line_index = LineIndex::new(source);
        let errors = vec![ParseError {
            message: "Unexpected token".to_string(),
            error_type: ErrorType::UnexpectedToken,
            span: Span::new(5, 10),
            related: Vec::new(),
        }];

        let uri = Url::parse("file:///test.bsl").expect("url");
        let diagnostics = syntax_errors_to_diagnostics(&errors, &uri, source, &line_index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Unexpected token");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(diagnostics[0].source, Some("bsl-syntax".to_string()));
        assert_eq!(diagnostics[0].range.start.line, 0);
        assert_eq!(diagnostics[0].range.start.character, 5);
        assert_eq!(diagnostics[0].range.end.line, 0);
        assert_eq!(diagnostics[0].range.end.character, 10);
    }

    #[test]
    fn test_missing_token_related_information() {
        let source = "line0\nline1\nline2";
        let line_index = LineIndex::new(source);
        let err_offset = source.find("line2").expect("line2") as u32;
        let related_offset = (source.find("line0").expect("line0") + 2) as u32;
        let errors = vec![ParseError {
            message: "Missing required element: ENDIF_KEYWORD".to_string(),
            error_type: ErrorType::MissingToken,
            span: Span::new(err_offset, err_offset),
            related: vec![RelatedInformation {
                message: "Block start: IF".to_string(),
                span: Span::new(related_offset, related_offset),
            }],
        }];

        let uri = Url::parse("file:///test.bsl").expect("url");
        let diagnostics = syntax_errors_to_diagnostics(&errors, &uri, source, &line_index);

        assert_eq!(diagnostics.len(), 1);
        let related = diagnostics[0]
            .related_information
            .as_ref()
            .expect("related information");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].message, "Block start: IF");
        assert_eq!(related[0].location.uri, uri);
        assert_eq!(related[0].location.range.start.line, 0);
        assert_eq!(related[0].location.range.start.character, 2);
    }

    #[test]
    fn test_semantic_error_to_diagnostic() {
        let error = TypeDiagnostic {
            message: "Type mismatch".to_string(),
            severity: SharedSeverity::Error,
            span: Span::new(2, 4),
        };

        let source = "abcdef";
        let line_index = LineIndex::new(source);
        let diagnostic = semantic_error_to_diagnostic(&error, source, &line_index);

        assert_eq!(diagnostic.message, "Type mismatch");
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source, Some("bsl-analysis-v2".to_string()));
        assert_eq!(diagnostic.range.start.line, 0);
        assert_eq!(diagnostic.range.start.character, 2);
        assert_eq!(diagnostic.range.end.line, 0);
        assert_eq!(diagnostic.range.end.character, 4);
    }
}
