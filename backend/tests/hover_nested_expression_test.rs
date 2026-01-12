mod support;

fn utf16_column(line: &str, needle: &str) -> u32 {
    let byte_idx = line.find(needle).expect("needle not found in line");
    line[..byte_idx].encode_utf16().count() as u32
}

#[tokio::test]
async fn test_hover_variable_inside_condition_expression() {
    let code = "Процедура Тест()\n\
    Число = 1;\n\
    Если Число > 0 Тогда\n\
        Сообщить(\"ok\");\n\
    КонецЕсли;\n\
КонецПроцедуры";

    let line_idx = 2u32;
    let line_text = code.lines().nth(line_idx as usize).expect("line exists");
    let column = utf16_column(line_text, "Число");

    let deps_bundle = support::deps_bundle_v2_fallback();
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, line_idx, column)
        .unwrap_or_default();

    assert!(
        hover.contains("**Переменная:**"),
        "ожидали hover переменной, получено: {}",
        hover
    );
    assert!(
        hover.contains("Число"),
        "hover должен содержать имя/тип переменной: {}",
        hover
    );
    assert!(
        !hover.contains("Если ... Тогда"),
        "hover не должен сваливаться в IfStatement: {}",
        hover
    );
}

#[tokio::test]
async fn test_hover_variable_inside_call_argument() {
    let code = "Процедура Тест()\n\
    Число = 1;\n\
    СтрокаЗнач = \"x\" + Строка(Число);\n\
КонецПроцедуры";

    let line_idx = 2u32;
    let line_text = code.lines().nth(line_idx as usize).expect("line exists");
    let column = utf16_column(line_text, "Число");

    let deps_bundle = support::deps_bundle_v2_fallback();
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, line_idx, column)
        .unwrap_or_default();

    assert!(
        hover.contains("**Переменная:**"),
        "ожидали hover переменной, получено: {}",
        hover
    );
    assert!(
        hover.contains("Число"),
        "hover должен содержать имя/тип переменной: {}",
        hover
    );
}

#[tokio::test]
async fn test_hover_function_inside_binary_expression() {
    let code = "Процедура Тест()\n\
    Число = 1;\n\
    СтрокаЗнач = \"x\" + Строка(Число);\n\
КонецПроцедуры";

    let line_idx = 2u32;
    let line_text = code.lines().nth(line_idx as usize).expect("line exists");
    let column = utf16_column(line_text, "Строка(");

    let deps_bundle = support::deps_bundle_v2_fallback();
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, line_idx, column)
        .unwrap_or_default();

    assert!(
        hover.contains("FunctionCall"),
        "ожидали hover для вызова функции, получено: {}",
        hover
    );
    assert!(
        hover.contains("Строка"),
        "hover должен содержать имя функции: {}",
        hover
    );
}
