mod shared_test_fixtures;

use bsl_backend::helpers::hover_formatter::HoverFormatConfig;
use shared_test_fixtures::get_test_service;

#[tokio::test]
async fn test_hover_on_property_name_shows_property_type() {
    let service = get_test_service();

    let code = "Процедура Тест()\n\
    ТаблЗнч = Новый ТаблицаЗначений;\n\
    КолонкиТаблЗнач = ТаблЗнч.Колонки;\n\
КонецПроцедуры";

    // line/column: 0-based, column UTF-16.
    // В строке 'КолонкиТаблЗнач = ТаблЗнч.Колонки;' имя свойства начинается с колонки 30.
    let hover = service
        .get_hover_info(code, 2, 30, Some(HoverFormatConfig::default()))
        .await
        .expect("hover request failed")
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
