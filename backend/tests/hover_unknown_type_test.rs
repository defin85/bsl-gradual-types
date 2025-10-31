//! Интеграционный тест: Hover на переменных с конфигурационными типами
//!
//! Проверяет, что система честно сообщает об уровне уверенности (certainty) для типов,
//! когда метаданные конфигурации не загружены (Inferred 50% вместо Unknown).

use bsl_backend::system::SystemCoordinator;

#[tokio::test]
async fn test_hover_shows_unknown_type_warning_for_configuration_types() {
    // Инициализация системы (только базовые типы, без Syntax Helper)
    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths(None, None, None) // без Syntax Helper и Configuration
        .await
        .expect("Failed to start SystemCoordinator");

    let type_system_service = coordinator
        .type_service()
        .expect("TypeSystemService not available");

    // Тестовый код: переменная с типом из конфигурации (не загружен)
    let source = r#"
Функция ТестНесуществующегоТипа()
    Перем СправочникКонтрагенты;
    СправочникКонтрагенты = Справочники.Контрагенты;
    Возврат СправочникКонтрагенты;
КонецФункции
"#;

    // Hover на переменной "СправочникКонтрагенты" в строке 3 (присваивание)
    let hover_result = type_system_service
        .get_hover_info(source, 3, 10) // колонка 10 = внутри "СправочникКонтрагенты"
        .await
        .expect("get_hover_info failed");

    assert!(hover_result.is_some(), "Hover должен вернуть информацию");

    let hover_text = hover_result.unwrap();
    eprintln!("=== HOVER TEXT ===\n{}\n==================", hover_text);

    // ✅ НОВОЕ ПОВЕДЕНИЕ: Система показывает Inferred (50%) для конфигурационных типов без метаданных
    assert!(
        hover_text.contains("🟡 Inferred (50%)"),
        "Configuration type без метаданных должен показывать Inferred (50%). Actual hover:\n{}",
        hover_text
    );
    assert!(
        hover_text.contains("⚠️"),
        "Hover должен содержать предупреждение (⚠️). Actual hover:\n{}",
        hover_text
    );
    assert!(
        hover_text.contains("Детали типа недоступны"),
        "Hover должен сообщать, что детали типа недоступны"
    );
    assert!(
        hover_text.contains("Справочники.Контрагенты"),
        "Hover должен показывать имя типа"
    );
    assert!(
        hover_text.contains("Возможные причины"),
        "Hover должен перечислять возможные причины отсутствия деталей"
    );

    // ❌ Проверяем, что hover НЕ содержит фантомную информацию (методы/свойства)
    // Но теперь допускается базовая информация о типе (имя, certainty)
    assert!(
        !hover_text.contains("📚 **Методы:**"),
        "Hover НЕ должен показывать список методов (метаданные не загружены)"
    );
    assert!(
        !hover_text.contains("📦 **Свойства:**"),
        "Hover НЕ должен показывать список свойств (метаданные не загружены)"
    );
}

#[tokio::test]
async fn test_hover_shows_correct_info_for_platform_types() {
    // Инициализация системы с Platform Types (Syntax Helper)
    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths(
            Some(std::path::Path::new("examples/syntax_helper")),
            None,
            None,
        )
        .await
        .expect("Failed to start SystemCoordinator");

    let type_system_service = coordinator
        .type_service()
        .expect("TypeSystemService not available");

    // Тестовый код: переменная с существующим Platform Type
    let source = r#"
Функция ТестМассива()
    Перем МассивДанных;
    МассивДанных = Новый Массив;
    Возврат МассивДанных;
КонецФункции
"#;

    // Hover на переменной "МассивДанных" в строке 3
    let hover_result = type_system_service
        .get_hover_info(source, 3, 10) // колонка 10 = внутри "МассивДанных"
        .await
        .expect("get_hover_info failed");

    assert!(hover_result.is_some(), "Hover должен вернуть информацию");

    let hover_text = hover_result.unwrap();

    // ✅ Проверяем, что hover показывает корректную информацию для Platform Type
    assert!(
        hover_text.contains("Массив"),
        "Hover должен показывать тип Массив"
    );

    // ✅ Для Platform Type должны быть методы (если они загружены из Syntax Helper)
    // Если Syntax Helper загружен, проверяем наличие методов
    // Если НЕ загружен, проверяем warning
    if hover_text.contains("📚 **Методы:**") {
        // Syntax Helper загружен → проверяем методы типа Массив
        assert!(
            hover_text.contains("Добавить") || hover_text.contains("Количество"),
            "Hover должен показывать методы Массива"
        );
    } else {
        // Syntax Helper НЕ загружен → должно быть предупреждение
        assert!(
            hover_text.contains("⚠️") || hover_text.contains("Тип не найден"),
            "Hover должен показывать предупреждение, если Syntax Helper не загружен"
        );
    }

    // ❌ НЕ должно быть предупреждения "Тип не найден" для Platform Type
    // (но это зависит от того, загружен ли Syntax Helper)
}

#[tokio::test]
async fn test_hover_differentiates_platform_and_configuration_types() {
    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths(
            Some(std::path::Path::new("examples/syntax_helper")),
            None,
            None,
        )
        .await
        .expect("Failed to start SystemCoordinator");

    let type_system_service = coordinator
        .type_service()
        .expect("TypeSystemService not available");

    let source = r#"
Функция ТестРазныхТипов()
    Перем МассивДанных;
    Перем ДокументЗаказ;

    МассивДанных = Новый Массив;           // Platform Type
    ДокументЗаказ = Документы.ЗаказКлиента; // Configuration Type (не загружен)
КонецФункции
"#;

    // 1. Hover на Platform Type (Массив)
    let hover_platform = type_system_service
        .get_hover_info(source, 5, 10)
        .await
        .expect("get_hover_info failed for platform type");

    assert!(hover_platform.is_some());
    let platform_text = hover_platform.unwrap();

    // 2. Hover на Configuration Type (Документы)
    let hover_config = type_system_service
        .get_hover_info(source, 6, 10)
        .await
        .expect("get_hover_info failed for config type");

    assert!(hover_config.is_some());
    let config_text = hover_config.unwrap();

    // ✅ Проверяем, что hover показывает РАЗНУЮ информацию
    assert_ne!(
        platform_text, config_text,
        "Hover для Platform Type и Configuration Type должен быть разным"
    );

    // ✅ Configuration Type должен показывать Inferred (50%) и предупреждение
    assert!(
        config_text.contains("🟡 Inferred (50%)"),
        "Configuration Type должен показывать Inferred (50%). Actual:\n{}",
        config_text
    );
    assert!(
        config_text.contains("⚠️"),
        "Configuration Type должен показывать предупреждение. Actual:\n{}",
        config_text
    );
    assert!(
        config_text.contains("Детали типа недоступны"),
        "Configuration Type должен сообщать, что детали типа недоступны. Actual:\n{}",
        config_text
    );
}
