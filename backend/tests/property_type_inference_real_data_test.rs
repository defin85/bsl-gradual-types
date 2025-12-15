//! Интеграционный тест: типизация свойства из Syntax Helper (без хардкода)

mod shared_test_fixtures;

use shared_test_fixtures::get_test_service;

#[tokio::test]
async fn test_value_table_columns_property_type_is_resolved() {
    let service = get_test_service();

    let code = r#"
Процедура Тест()
    ТаблЗнч = Новый ТаблицаЗначений;
    КолонкиТаблЗнч = ТаблЗнч.Колонки;
КонецПроцедуры
"#;

    let result = service
        .get_semantic_tree(code, "test.bsl", false, true, true)
        .await
        .expect("Failed to get semantic tree");

    let resolved = result
        .symbol_table
        .get("КолонкиТаблЗнч")
        .and_then(|v| v.resolved_type.as_ref())
        .expect("КолонкиТаблЗнч should have resolved_type");

    assert_eq!(
        resolved.name,
        "КоллекцияКолонокТаблицыЗначений",
        "ТаблицаЗначений.Колонки должен иметь тип КоллекцияКолонокТаблицыЗначений"
    );
    assert_eq!(
        resolved.certainty,
        "Known",
        "Тип свойства должен быть Known (из документации платформы)"
    );
}

