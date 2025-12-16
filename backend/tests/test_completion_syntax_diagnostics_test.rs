mod shared_test_fixtures;

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

    let service = shared_test_fixtures::get_test_service();

    let diagnostics = service
        .validate_semantics(&code, None)
        .await
        .expect("validate_semantics failed");

    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("Тип 'Масив1' не найден")),
        "Ожидается semantic diagnostic для опечатки типа 'Масив1'. Actual:\n{:#?}",
        diagnostics
    );

    let hover = service
        .get_hover_info(&code, 20, 6, None) // line 21 (0-based), column inside "МойМассив"
        .await
        .expect("get_hover_info failed");

    let hover_text = hover.expect("Hover должен вернуть информацию");

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
