mod support;

#[tokio::test]
async fn test_examples_conf_test_completion_reports_unknown_type_for_typo() {
    let code = std::fs::read_to_string("examples/conf/test_completion.bsl")
        .or_else(|_| std::fs::read_to_string("../examples/conf/test_completion.bsl"))
        .expect("Failed to read examples/conf/test_completion.bsl");

    let coordinator = bsl_backend::system::ParserCoordinator::with_fallback();
    let result = coordinator
        .parse(&code)
        .expect("Tree-sitter parsing should succeed");

    assert!(
        !result.has_errors(),
        "examples/conf/test_completion.bsl должен парситься без синтаксических ошибок"
    );

    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let diagnostics =
        support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "test_completion.bsl", &code);

    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("Тип 'Масив1' не найден")),
        "Ожидается semantic diagnostic для опечатки типа 'Масив1'. Actual:\n{:#?}",
        diagnostics
    );

    let hover_text = support::hover_for_code(deps_bundle.as_ref(), "test_completion.bsl", &code, 20, 6)
        .expect("Hover должен вернуть информацию");

    assert!(
        hover_text.contains("Масив1"),
        "Hover должен содержать имя типа 'Масив1'. Actual hover:\n{}",
        hover_text
    );
    assert!(
        hover_text.contains("Unknown (0%)"),
        "Hover должен показывать Unknown (0%) для ненайденного типа. Actual hover:\n{}",
        hover_text
    );
}
