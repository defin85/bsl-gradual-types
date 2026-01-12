//! Интеграционный тест: типизация свойства из Syntax Helper (без хардкода)

mod support;

#[tokio::test]
async fn test_value_table_columns_property_type_is_resolved() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let code = r#"
Процедура Тест()
    ТаблЗнч = Новый ТаблицаЗначений;
    КолонкиТаблЗнч = ТаблЗнч.Колонки;
КонецПроцедуры
"#;

    let ir_program = support::ir_program_for_code(deps_bundle.as_ref(), "test.bsl", code);
    let result = ir_program.to_dto(false, true);

    let resolved = result
        .symbol_table
        .get("КолонкиТаблЗнч")
        .and_then(|v| v.resolved_type.as_ref())
        .expect("КолонкиТаблЗнч should have resolved_type");

    assert_eq!(
        resolved.name, "КоллекцияКолонокТаблицыЗначений",
        "ТаблицаЗначений.Колонки должен иметь тип КоллекцияКолонокТаблицыЗначений"
    );
    assert_eq!(
        resolved.certainty, "Known",
        "Тип свойства должен быть Known (из документации платформы)"
    );
}
