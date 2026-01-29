//! Регрессия: вызовы пользовательских функций должны резолвиться
//! - неопределенные глобальные вызовы должны давать diagnostic

mod support;

use bsl_shared::domain::types::DiagnosticSeverity;
#[tokio::test]
async fn test_undefined_global_function_call_is_reported() {
    // Здесь важно использовать Syntax Helper: без него SignatureIndex может быть пустым,
    // и диагностика неопределенных глобальных функций будет подавлена (graceful degradation).
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let code = r#"
Процедура Тест()
    Результат = НеобъявленнаяФункцияКотораяВозвращаетСтроку();
КонецПроцедуры
"#;

    let diagnostics =
        support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "inline.bsl", code);

    assert!(
        diagnostics.iter().any(|d| {
            d.severity == DiagnosticSeverity::Error
                && d.message.contains("Неопределенная процедура или функция")
                && d.message
                    .contains("НеобъявленнаяФункцияКотораяВозвращаетСтроку")
        }),
        "ожидается diagnostic для неопределенной функции. Actual: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}
