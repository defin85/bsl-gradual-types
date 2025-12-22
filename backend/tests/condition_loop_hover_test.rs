#[path = "shared_test_fixtures.rs"]
mod shared_test_fixtures;

use shared_test_fixtures::get_test_service;

const CODE: &str = "Процедура Тест()\n\
    МассивДанных = Новый Массив();\n\
    Если 1 < 2 Тогда\n\
        Сообщить(\"ok\");\n\
    КонецЕсли;\n\
    Пока Флаг Цикл\n\
        Прервать;\n\
    КонецЦикла;\n\
    Для i = 1 По 10 Цикл\n\
        Сообщить(i);\n\
    КонецЦикла;\n\
    Для Каждого Элемент Из МассивДанных Цикл\n\
        Сообщить(Элемент);\n\
    КонецЦикла;\n\
КонецПроцедуры";

async fn hover_at(line: u32, column: u32) -> String {
    let service = get_test_service();
    service
        .get_hover_info(CODE, line, column, None)
        .await
        .expect("hover request failed")
        .expect("hover should exist")
}

fn assert_has_certainty(hover: &str) {
    assert!(
        hover.contains("*Уверенность:*"),
        "hover должен показывать уверенность: {}",
        hover
    );
}

#[tokio::test]
async fn test_hover_if_shows_expected_type_and_certainty() {
    let hover = hover_at(2, 4).await;

    assert!(
        hover.contains("Если ... Тогда"),
        "hover должен быть для IfStatement: {}",
        hover
    );
    assert!(
        hover.contains("*Ожидаемый тип:* Булево"),
        "hover должен показывать ожидаемый тип Булево: {}",
        hover
    );
    assert!(
        hover.contains("*Фактический тип:* Булево"),
        "hover должен показывать фактический тип Булево: {}",
        hover
    );
    assert_has_certainty(&hover);
}

#[tokio::test]
async fn test_hover_while_shows_uncertainty_reason() {
    let hover = hover_at(5, 4).await;

    assert!(
        hover.contains("Пока ... Цикл"),
        "hover должен быть для WhileLoop: {}",
        hover
    );
    assert!(
        hover.contains("*Ожидаемый тип:* Булево"),
        "hover должен показывать ожидаемый тип Булево: {}",
        hover
    );
    assert!(
        hover.contains("*Уверенность:* Unknown"),
        "hover должен показывать уверенность Unknown: {}",
        hover
    );
    assert!(
        hover.contains("Переменная \"Флаг\" не объявлена"),
        "hover должен показывать причину неопределенности: {}",
        hover
    );
}

#[tokio::test]
async fn test_hover_for_loop_shows_range_type() {
    let hover = hover_at(8, 4).await;

    assert!(
        hover.contains("Для i = ... По ... Цикл"),
        "hover должен быть для ForLoop: {}",
        hover
    );
    assert!(
        hover.contains("*Ожидаемый тип:* Число"),
        "hover должен показывать ожидаемый тип Число: {}",
        hover
    );
    assert!(
        hover.contains("*Фактический тип:* Число"),
        "hover должен показывать фактический тип Число: {}",
        hover
    );
    assert_has_certainty(&hover);
}

#[tokio::test]
async fn test_hover_foreach_shows_collection_type() {
    let hover = hover_at(11, 4).await;

    assert!(
        hover.contains("Для Каждого Элемент Из ... Цикл"),
        "hover должен быть для ForEachLoop: {}",
        hover
    );
    assert!(
        hover.contains("*Ожидаемый тип:* Коллекция"),
        "hover должен показывать ожидаемый тип Коллекция: {}",
        hover
    );
    assert!(
        hover.contains("*Фактический тип:* Массив"),
        "hover должен показывать фактический тип Массив: {}",
        hover
    );
    assert_has_certainty(&hover);
}
