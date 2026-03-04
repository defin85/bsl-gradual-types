use super::*;

#[test]
fn generalized_for_picks_first_garbage_token_not_last() {
    let between = " 0 abc def ";
    let (start, end) = first_unexpected_token_span_in_to_clause(between).expect("token span");
    assert_eq!(&between[start..end], "abc");
}

#[test]
fn masking_comment_stops_at_eol_and_keeps_next_line() {
    let input = "// Шаг -1\nabc Цикл";
    let masked = mask_line_for_rules(input);
    assert!(masked.starts_with("//"));
    assert!(
        !masked.contains("Шаг"),
        "comment text should be masked, got: {masked:?}"
    );
    assert!(
        masked.contains("\nabc Цикл"),
        "next line should remain visible, got: {masked:?}"
    );
}

#[test]
fn line_cap_prefers_parser_origin_even_if_message_looks_heuristic() {
    let source = "x y";
    let line_index = LineIndex::new(source);

    let parser_error = ParseError {
        error_type: ErrorType::UnexpectedToken,
        message: "Отсутствует тип после 'Новый'".to_string(),
        span: Span::new(0, 0),
        related: Vec::new(),
    };
    let heuristic_error = ParseError {
        error_type: ErrorType::InvalidSyntax,
        message: "parser-ish".to_string(),
        span: Span::new(1, 1),
        related: Vec::new(),
    };

    let out = normalize_syntax_errors(
        source,
        &line_index,
        vec![parser_error.clone()],
        vec![heuristic_error],
    );

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].span, parser_error.span);
    assert_eq!(out[0].message, parser_error.message);
    assert_eq!(out[0].error_type, parser_error.error_type);
}
