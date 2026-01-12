use bsl_backend::helpers::hover_formatter::HoverFormatConfig;

mod support;

#[tokio::test]
async fn test_hover_on_property_name_shows_property_type() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let code = "Процедура Тест()\n\
    ТаблЗнч = Новый ТаблицаЗначений;\n\
    КолонкиТаблЗнач = ТаблЗнч.Колонки;\n\
КонецПроцедуры";

    // line/column: 0-based, column UTF-16.
    // В строке 'КолонкиТаблЗнач = ТаблЗнч.Колонки;' имя свойства начинается с колонки 30.
    let hover = support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        "inline.bsl",
        code,
        2,
        30,
        Some(HoverFormatConfig::default()),
    )
    .expect("hover should exist");

    assert!(
        hover.contains("**Свойство:**"),
        "hover должен быть для свойства, а не для переменной объекта: {}",
        hover
    );
    assert!(
        hover.contains("КоллекцияКолонокТаблицыЗначений"),
        "должен показываться тип свойства ТаблицаЗначений.Колонки: {}",
        hover
    );
}
