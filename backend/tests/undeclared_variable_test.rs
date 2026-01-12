//! Diagnostics regression tests for undeclared variables (v2-only path).

mod support;

fn undeclared_messages(code: &str) -> Vec<String> {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "inline.bsl", code)
        .into_iter()
        .map(|diag| diag.message)
        .filter(|message| message.contains("Необъявленная переменная"))
        .collect()
}

#[test]
fn undeclared_variable_in_method_argument_is_reported() {
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(необъявленная);
КонецПроцедуры
"#;

    let messages = undeclared_messages(code);
    assert!(!messages.is_empty(), "expected undeclared variable diagnostic");
    assert!(
        messages.iter().any(|msg| msg.contains("необъявленная")),
        "expected variable name in message: {:?}",
        messages
    );
}

#[test]
fn declared_variable_is_not_reported() {
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    код = "001";
    М.НайтиПоКоду(код);
КонецПроцедуры
"#;

    let messages = undeclared_messages(code);
    assert!(messages.is_empty(), "unexpected diagnostics: {:?}", messages);
}

#[test]
fn function_parameter_is_treated_as_declared() {
    let code = r#"
Процедура Тест(ПараметрВходной)
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(ПараметрВходной);
КонецПроцедуры
"#;

    let messages = undeclared_messages(code);
    assert!(messages.is_empty(), "unexpected diagnostics: {:?}", messages);
}

