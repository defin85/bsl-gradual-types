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
