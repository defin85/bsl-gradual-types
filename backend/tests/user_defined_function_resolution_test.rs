//! Регрессия: вызовы пользовательских функций должны резолвиться
//! - тип переменной из `X = Функция()` должен выводиться по return'у функции
//! - неопределенные глобальные вызовы должны давать diagnostic

mod support;

use bsl_shared::domain::types::DiagnosticSeverity;
use bsl_shared::ir::SemanticNodeKind;

#[tokio::test]
async fn test_user_defined_function_call_infers_variable_type() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Процедура Тест()
    КакаяТоСтрока = ФункцияКотораяВозвращаетСтроку();
КонецПроцедуры

Функция ФункцияКотораяВозвращаетСтроку()
    Возврат "ТестоваяСтрока";
КонецФункции
"#;

    let program = support::ir_program_for_code(deps_bundle.as_ref(), "inline.bsl", code);

    // Находим scope процедуры и проверяем тип переменной внутри него.
    let mut proc_scope = None;
    for node in &program.nodes {
        if let SemanticNodeKind::ProcedureDeclaration { body_scope, .. } = node.kind {
            proc_scope = Some(body_scope);
            break;
        }
    }
    let proc_scope = proc_scope.expect("procedure scope");

    let var_type = program
        .symbols
        .get_variable_type(proc_scope, "КакаяТоСтрока")
        .expect("expected inferred variable type");

    assert_eq!(var_type.type_name(), "Строка", "тип должен быть Строка");
}

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
