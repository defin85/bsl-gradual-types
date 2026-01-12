//! Тесты для валидации конкатенации строк

mod support;

use bsl_shared::domain::types::DiagnosticSeverity;

#[tokio::test]
async fn test_invalid_string_concat_reports_error() {
    let deps_bundle = support::deps_bundle_v2_fallback();
    let code = r#"
Процедура Тест()
    Текст = "текст" + 1;
КонецПроцедуры
"#;

    let errors = support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "inline.bsl", code);

    let concat_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Конкатенация строк"))
        .collect();

    assert!(
        !concat_errors.is_empty(),
        "Должна быть ошибка для конкатенации строк с не-строкой"
    );
    assert_eq!(concat_errors[0].severity, DiagnosticSeverity::Error);
}
