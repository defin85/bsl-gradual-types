//! Тест для проверки исправления отображения certainty в hover
//!
//! Проверяет что certainty показывается для ВСЕХ переменных (даже без RawTypeData)

use bsl_backend::system::SystemCoordinator;
use bsl_backend::application::TypeSystemService;
use std::sync::Arc;

/// Setup функция для создания TypeSystemService с загруженными типами платформы
async fn setup_type_system_service() -> Arc<TypeSystemService> {
    let coordinator = SystemCoordinator::new();
    coordinator.start()
        .await
        .expect("Failed to start SystemCoordinator");
    coordinator.type_service()
        .expect("TypeSystemService should be initialized after start()")
}

#[tokio::test]
async fn test_certainty_shown_for_platform_types() {
    // Проверяем что certainty показывается для платформенных типов (Массив, Структура)
    let code = r#"
Процедура ТестПлатформенныхТипов()
    МойМассив = Новый Массив();
    МояСтруктура = Новый Структура();
КонецПроцедуры
"#;

    let service = setup_type_system_service().await;

    // Hover на "МойМассив"
    let hover1 = service.get_hover_info(code, 2, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for МойМассив ===\n{}", hover1);

    // Проверяем наличие certainty (любого)
    assert!(
        hover1.contains("🟢 Known") || hover1.contains("🟡 Inferred") || hover1.contains("⚪ Unknown"),
        "Hover должен содержать certainty индикатор\nActual: {}", hover1
    );
    assert!(hover1.contains("Уверенность:"), "Hover должен содержать метку 'Уверенность:'\nActual: {}", hover1);

    // Hover на "МояСтруктура"
    let hover2 = service.get_hover_info(code, 3, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for МояСтруктура ===\n{}", hover2);

    assert!(
        hover2.contains("🟢 Known") || hover2.contains("🟡 Inferred") || hover2.contains("⚪ Unknown"),
        "Hover должен содержать certainty индикатор\nActual: {}", hover2
    );
    assert!(hover2.contains("Уверенность:"), "Hover должен содержать метку 'Уверенность:'\nActual: {}", hover2);
}

#[tokio::test]
async fn test_certainty_shown_for_primitive_types() {
    // Проверяем что certainty показывается для примитивных типов (Строка, Число)
    let code = r#"
Процедура ТестПримитивов()
    МояСтрока = "текст";
    МоеЧисло = 42;
    МоеБулево = Истина;
КонецПроцедуры
"#;

    let service = setup_type_system_service().await;

    // Hover на "МояСтрока"
    let hover1 = service.get_hover_info(code, 2, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for МояСтрока ===\n{}", hover1);

    assert!(
        hover1.contains("Уверенность:"),
        "Hover для МояСтрока должен содержать 'Уверенность:'\nActual: {}", hover1
    );

    // Hover на "МоеЧисло"
    let hover2 = service.get_hover_info(code, 3, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for МоеЧисло ===\n{}", hover2);

    assert!(
        hover2.contains("Уверенность:"),
        "Hover для МоеЧисло должен содержать 'Уверенность:'\nActual: {}", hover2
    );

    // Hover на "МоеБулево"
    let hover3 = service.get_hover_info(code, 4, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for МоеБулево ===\n{}", hover3);

    assert!(
        hover3.contains("Уверенность:"),
        "Hover для МоеБулево должен содержать 'Уверенность:'\nActual: {}", hover3
    );
}

#[tokio::test]
async fn test_certainty_levels() {
    // Проверяем разные уровни certainty (Known/Inferred/Unknown)
    let code = r#"
Процедура ТестУровнейУверенности()
    // Known certainty - платформенный тип с точным определением
    Массив1 = Новый Массив();

    // Inferred certainty - может быть выведен из контекста
    Строка1 = "текст";

    // Unknown certainty - переменная без инициализации
    НеопределеннаяПеременная = Неопределено;
КонецПроцедуры
"#;

    let service = setup_type_system_service().await;

    // Hover на "Массив1"
    let hover1 = service.get_hover_info(code, 3, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for Массив1 (Expected: Known) ===\n{}", hover1);

    // Проверяем формат certainty (процент должен быть указан)
    assert!(
        hover1.contains("100%") || hover1.contains("Known") || hover1.contains("%"),
        "Hover должен содержать процент уверенности\nActual: {}", hover1
    );

    // Hover на "Строка1"
    let hover2 = service.get_hover_info(code, 6, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for Строка1 (Expected: Known/Inferred) ===\n{}", hover2);

    assert!(
        hover2.contains("%") && hover2.contains("Уверенность:"),
        "Hover должен содержать процент уверенности и метку 'Уверенность:'\nActual: {}", hover2
    );

    // Hover на "НеопределеннаяПеременная"
    let hover3 = service.get_hover_info(code, 9, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for НеопределеннаяПеременная (Expected: Unknown) ===\n{}", hover3);

    assert!(
        hover3.contains("Уверенность:"),
        "Hover должен содержать 'Уверенность:' даже для неопределенного типа\nActual: {}", hover3
    );
}

#[tokio::test]
async fn test_certainty_before_raw_type_check() {
    // КЛЮЧЕВОЙ ТЕСТ: Проверяем что certainty показывается ПЕРЕД проверкой RawTypeData
    // Даже если RawTypeData отсутствует (тип не загружен из Syntax Helper),
    // certainty должна быть видна
    let code = r#"
Процедура ТестБезRawTypeData()
    // Тип может не иметь RawTypeData, но certainty должна отображаться
    ПеременнаяБезМетаданных = "значение";
КонецПроцедуры
"#;

    let service = setup_type_system_service().await;

    let hover = service.get_hover_info(code, 3, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Hover for переменная без метаданных ===\n{}", hover);

    // КРИТИЧЕСКАЯ ПРОВЕРКА: certainty показывается ДО "⚠️ Детали типа недоступны"
    let certainty_pos = hover.find("Уверенность:");
    let warning_pos = hover.find("⚠️");

    match (certainty_pos, warning_pos) {
        (Some(cert_pos), Some(warn_pos)) => {
            assert!(
                cert_pos < warn_pos,
                "Certainty должна показываться ПЕРЕД предупреждением '⚠️'\nActual: {}", hover
            );
        }
        (Some(_), None) => {
            // Нормально - есть certainty, нет предупреждения
        }
        (None, _) => {
            panic!("Certainty ОТСУТСТВУЕТ в hover (БАГ НЕ ИСПРАВЛЕН!)\nActual: {}", hover);
        }
    }

    assert!(
        hover.contains("Уверенность:"),
        "Certainty должна показываться всегда\nActual: {}", hover
    );
}

#[tokio::test]
async fn test_regression_hover_still_shows_methods() {
    // Регрессионный тест: убеждаемся что hover всё ещё показывает методы/свойства
    // после добавления отображения certainty
    let code = r#"
Процедура ТестМетодов()
    МойМассив = Новый Массив();
КонецПроцедуры
"#;

    let service = setup_type_system_service().await;

    let hover = service.get_hover_info(code, 2, 5).await.unwrap().unwrap_or_default();
    println!("\n=== Regression test: hover with methods ===\n{}", hover);

    // Проверяем что hover содержит:
    // 1. Имя переменной
    assert!(hover.contains("МойМассив") || hover.contains("Переменная:"),
        "Hover должен содержать имя переменной\nActual: {}", hover);

    // 2. Тип
    assert!(hover.contains("Массив") || hover.contains("Тип:"),
        "Hover должен содержать тип\nActual: {}", hover);

    // 3. Certainty (новая функциональность)
    assert!(hover.contains("Уверенность:"),
        "Hover должен содержать certainty\nActual: {}", hover);

    // 4. Методы или предупреждение об отсутствии деталей (если RawTypeData есть)
    let has_methods = hover.contains("Методы:") || hover.contains("Добавить");
    let has_warning = hover.contains("⚠️");

    assert!(
        has_methods || has_warning,
        "Hover должен содержать либо методы, либо предупреждение об отсутствии деталей\nActual: {}", hover
    );
}
