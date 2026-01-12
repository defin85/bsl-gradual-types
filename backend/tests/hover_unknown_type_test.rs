//! Интеграционный тест: Hover на переменных с конфигурационными типами
//!
//! Проверяет, что система честно сообщает об уровне уверенности (certainty) для типов,
//! когда метаданные конфигурации не загружены (Inferred 50% вместо Unknown).

mod support;

#[tokio::test]
async fn test_hover_shows_unknown_type_warning_for_configuration_types() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    // Тестовый код: переменная с типом из конфигурации (не загружен)
    let source = r#"
Функция ТестНесуществующегоТипа()
    Перем СправочникКонтрагенты;
    СправочникКонтрагенты = Справочники.Контрагенты;
    Возврат СправочникКонтрагенты;
КонецФункции
    "#;

    // Hover на переменной "СправочникКонтрагенты" в строке 3 (присваивание)
    let hover_text = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", source, 3, 10)
        .expect("Hover должен вернуть информацию");
    eprintln!("=== HOVER TEXT ===\n{}\n==================", hover_text);

    // ✅ НОВОЕ ПОВЕДЕНИЕ: Система показывает InferredWeak (50%) для конфигурационных типов без метаданных
    assert!(
        hover_text.contains("InferredWeak (50%)"),
        "Configuration type без метаданных должен показывать InferredWeak (50%). Actual hover:\n{}",
        hover_text
    );
    assert!(
        hover_text.contains("Детали типа недоступны"),
        "Hover должен сообщать, что детали типа недоступны"
    );
    assert!(
        hover_text.contains("Контрагенты"),
        "Hover должен показывать имя типа"
    );
    assert!(
        hover_text.contains("Возможные причины"),
        "Hover должен перечислять возможные причины отсутствия деталей"
    );

    // ❌ Проверяем, что hover НЕ содержит фантомную информацию (методы/свойства)
    assert!(
        !hover_text.contains("Методы (показано"),
        "Hover НЕ должен показывать список методов (метаданные не загружены)"
    );
    assert!(
        !hover_text.contains("Свойства (показано"),
        "Hover НЕ должен показывать список свойств (метаданные не загружены)"
    );
}

#[tokio::test]
async fn test_hover_shows_correct_info_for_platform_types() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    // Тестовый код: переменная с существующим Platform Type
    let source = r#"
Функция ТестМассива()
    Перем МассивДанных;
    МассивДанных = Новый Массив;
    Возврат МассивДанных;
КонецФункции
    "#;

    // Hover на переменной "МассивДанных" в строке 3
    let hover_text = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", source, 3, 10)
        .expect("Hover должен вернуть информацию");

    // ✅ Проверяем, что hover показывает корректную информацию для Platform Type
    assert!(
        hover_text.contains("Массив"),
        "Hover должен показывать тип Массив"
    );

    // ✅ Для Platform Type должны быть методы (если они загружены из Syntax Helper)
    if hover_text.contains("Методы (показано") {
        // Syntax Helper загружен → проверяем методы типа Массив
        assert!(
            hover_text.contains("Добавить") || hover_text.contains("Количество"),
            "Hover должен показывать методы Массива"
        );
    } else {
        // Syntax Helper НЕ загружен → должно быть предупреждение/сообщение о недоступности
        assert!(
            hover_text.contains("Методы недоступны")
                || hover_text.contains("Детали типа недоступны"),
            "Hover должен показывать предупреждение, если Syntax Helper не загружен"
        );
    }

    // ❌ НЕ должно быть предупреждения "Тип не найден" для Platform Type
    // (но это зависит от того, загружен ли Syntax Helper)
}

#[tokio::test]
async fn test_hover_differentiates_platform_and_configuration_types() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let source = r#"
Функция ТестРазныхТипов()
    Перем МассивДанных;
    Перем ДокументЗаказ;

    МассивДанных = Новый Массив;           // Platform Type
    ДокументЗаказ = Документы.ЗаказКлиента; // Configuration Type (не загружен)
КонецФункции
    "#;

    // 1. Hover на Platform Type (Массив)
    let platform_text = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", source, 5, 10)
        .expect("platform hover");

    // 2. Hover на Configuration Type (Документы)
    let config_text = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", source, 6, 10)
        .expect("config hover");

    // ✅ Проверяем, что hover показывает РАЗНУЮ информацию
    assert_ne!(
        platform_text, config_text,
        "Hover для Platform Type и Configuration Type должен быть разным"
    );

    // ✅ Configuration Type должен показывать Inferred (50%) и предупреждение
    assert!(
        config_text.contains("InferredWeak (50%)"),
        "Configuration Type должен показывать InferredWeak (50%). Actual:\n{}",
        config_text
    );
    assert!(
        config_text.contains("Детали типа недоступны"),
        "Configuration Type должен сообщать, что детали типа недоступны. Actual:\n{}",
        config_text
    );
}
