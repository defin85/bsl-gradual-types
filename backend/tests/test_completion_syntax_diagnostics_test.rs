#[test]
fn debug_tree_sitter_for_examples_conf_test_completion() {
    let code = std::fs::read_to_string("examples/conf/test_completion.bsl")
        .or_else(|_| std::fs::read_to_string("../examples/conf/test_completion.bsl"))
        .expect("Failed to read examples/conf/test_completion.bsl");

    let coordinator = bsl_backend::system::ParserCoordinator::with_fallback();
    let result = coordinator.parse(&code).expect("Tree-sitter parsing should succeed");

    assert!(
        result.has_errors(),
        "Expected syntax errors in examples/conf/test_completion.bsl, but syntax_errors is empty"
    );

    for e in &result.syntax_errors {
        println!(
            "syntax_error: {} @ {}:{}-{}:{} ({:?})",
            e.message,
            e.span.start_line,
            e.span.start_column,
            e.span.end_line,
            e.span.end_column,
            e.error_type
        );
    }
}
