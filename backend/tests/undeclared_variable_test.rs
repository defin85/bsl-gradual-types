//! Diagnostics regression tests for undeclared variables (v2-only path).

mod support;

fn undeclared_messages_for_path(file_path: &str, code: &str) -> Vec<String> {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    support::semantic_diagnostics_for_code(deps_bundle.as_ref(), file_path, code)
        .into_iter()
        .map(|diag| diag.message)
        .filter(|message| message.contains("Необъявленная переменная"))
        .collect()
}

fn undeclared_messages(code: &str) -> Vec<String> {
    undeclared_messages_for_path("inline.bsl", code)
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
    assert!(
        !messages.is_empty(),
        "expected undeclared variable diagnostic"
    );
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
    assert!(
        messages.is_empty(),
        "unexpected diagnostics: {:?}",
        messages
    );
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
    assert!(
        messages.is_empty(),
        "unexpected diagnostics: {:?}",
        messages
    );
}

#[test]
fn local_function_call_before_declaration_is_not_undeclared_variable() {
    let code = r#"
Процедура Тест()
    Результат = ЛокальнаяФункция();
КонецПроцедуры

Функция ЛокальнаяФункция()
    Возврат 1;
КонецФункции
"#;

    let messages = undeclared_messages(code);
    assert!(
        messages.is_empty(),
        "unexpected undeclared variable diagnostics: {:?}",
        messages
    );
}

#[test]
fn form_module_implicit_arguments_are_not_reported_as_undeclared() {
    let code = r#"
Процедура ПриСозданииНаСервере()
    ПроверкаСостоянияДокументаПередЗаписьюСервер.ПриСозданииНаСервереДокумент(ЭтотОбъект, Параметры);
КонецПроцедуры
"#;
    let messages = undeclared_messages_for_path(
        "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl",
        code,
    );

    assert!(
        !messages.iter().any(|msg| msg.contains("ЭтотОбъект")),
        "unexpected undeclared diagnostics for ЭтотОбъект: {:?}",
        messages
    );
    assert!(
        !messages.iter().any(|msg| msg.contains("Параметры")),
        "unexpected undeclared diagnostics for Параметры: {:?}",
        messages
    );
}

#[test]
fn form_module_no_context_reports_this_object_as_undeclared() {
    let code = r#"
&НаСервереБезКонтекста
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(ЭтотОбъект);
КонецПроцедуры
"#;
    let messages = undeclared_messages_for_path(
        "Documents/РеализацияТоваровУслуг/Forms/ФормаДокументаОбщая/Ext/Form/Module.bsl",
        code,
    );
    assert!(
        messages.iter().any(|msg| msg.contains("ЭтотОбъект")),
        "expected undeclared diagnostics for ЭтотОбъект in *БезКонтекста, got: {:?}",
        messages
    );
}

#[test]
fn manager_module_implicit_arguments_are_not_reported_as_undeclared() {
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(ЭтотОбъект);
    М.НайтиПоКоду(Объект);
КонецПроцедуры
"#;
    let messages = undeclared_messages_for_path("Documents/Док1/Ext/ManagerModule.bsl", code);

    assert!(
        !messages.iter().any(|msg| msg.contains("ЭтотОбъект")),
        "unexpected undeclared diagnostics for ЭтотОбъект: {:?}",
        messages
    );
    assert!(
        !messages.iter().any(|msg| msg.contains("Объект")),
        "unexpected undeclared diagnostics for Объект: {:?}",
        messages
    );
}

#[test]
fn object_module_implicit_arguments_are_not_reported_as_undeclared() {
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(ЭтотОбъект);
    М.НайтиПоКоду(Объект);
КонецПроцедуры
"#;
    let messages = undeclared_messages_for_path("Documents/Док1/Ext/ObjectModule.bsl", code);

    assert!(
        !messages.iter().any(|msg| msg.contains("ЭтотОбъект")),
        "unexpected undeclared diagnostics for ЭтотОбъект: {:?}",
        messages
    );
    assert!(
        !messages.iter().any(|msg| msg.contains("Объект")),
        "unexpected undeclared diagnostics for Объект: {:?}",
        messages
    );
}

#[test]
fn recordset_module_implicit_arguments_are_not_reported_as_undeclared() {
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(ЭтотОбъект);
    М.НайтиПоКоду(Объект);
КонецПроцедуры
"#;
    let messages = undeclared_messages_for_path(
        "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl",
        code,
    );

    assert!(
        !messages.iter().any(|msg| msg.contains("ЭтотОбъект")),
        "unexpected undeclared diagnostics for ЭтотОбъект: {:?}",
        messages
    );
    assert!(
        !messages.iter().any(|msg| msg.contains("Объект")),
        "unexpected undeclared diagnostics for Объект: {:?}",
        messages
    );
}

#[test]
fn form_module_bare_owner_member_name_stays_undeclared() {
    let code = r#"
Процедура Тест()
    Проверка = ЗначениеЗаполнено(ДополнительныеСвойства);
КонецПроцедуры
"#;
    let messages =
        undeclared_messages_for_path("Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl", code);

    assert!(
        messages
            .iter()
            .any(|msg| msg.contains("ДополнительныеСвойства")),
        "expected undeclared diagnostics for bare owner member name in FormModule, got: {:?}",
        messages
    );
}
