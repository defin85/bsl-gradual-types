/// Тест для проблемы #1: Новый ТаблицаЗначений vs Новый ТаблицаЗначений()
///
/// В 1С допустимы ОБА синтаксиса для создания объектов:
/// - `Новый ТаблицаЗначений` (без скобок)
/// - `Новый ТаблицаЗначений()` (со скобками)
///
/// Проверяем что оба варианта корректно парсятся и сохраняют TypeResolution.

mod shared_test_fixtures;

use bsl_backend::application::TypeSystemService;
use shared_test_fixtures::get_test_service;

/// Helper: создать TypeSystemService для тестов WITH PLATFORM TYPES LOADED
fn create_test_service() -> &'static TypeSystemService {
    get_test_service()
}

#[tokio::test]
async fn test_constructor_without_parentheses() {
    // Arrange: создаём TypeSystemService с платформенными типами
    let service = create_test_service();

    // Act: парсим код БЕЗ скобок
    let code = r#"
Функция Тест()
    ТЗ = Новый ТаблицаЗначений;
    Возврат ТЗ.Количество();
КонецФункции
"#;

    let parse_result = service.parse_and_validate(code).unwrap();
    assert!(
        parse_result.is_empty(),
        "Не должно быть syntax errors для 'Новый ТаблицаЗначений'"
    );

    // Проверяем что semantic validation НЕ выдаёт ошибку
    let semantic_errors = service.validate_semantics(code, None).await.unwrap();

    // DEBUG: выводим ошибки если есть
    if !semantic_errors.is_empty() {
        for error in &semantic_errors {
            eprintln!("ERROR: {:?}", error);
        }
    }

    assert!(
        semantic_errors.is_empty(),
        "❌ Метод 'Количество' должен быть найден для 'ТаблицаЗначений'"
    );
}

#[tokio::test]
async fn test_constructor_with_parentheses() {
    // Arrange
    let service = create_test_service();

    // Act: парсим код СО скобками
    let code = r#"
Функция Тест()
    ТЗ = Новый ТаблицаЗначений();
    Возврат ТЗ.Количество();
КонецФункции
"#;

    let parse_result = service.parse_and_validate(code).unwrap();
    assert!(
        parse_result.is_empty(),
        "Не должно быть syntax errors для 'Новый ТаблицаЗначений()'"
    );

    // Проверяем что semantic validation НЕ выдаёт ошибку
    let semantic_errors = service.validate_semantics(code, None).await.unwrap();

    // DEBUG: выводим ошибки если есть
    if !semantic_errors.is_empty() {
        for error in &semantic_errors {
            eprintln!("ERROR: {:?}", error);
        }
    }

    assert!(
        semantic_errors.is_empty(),
        "✅ Метод 'Количество' должен быть найден для 'ТаблицаЗначений()'"
    );
}

#[tokio::test]
async fn test_both_constructors_in_one_function() {
    // Arrange
    let service = create_test_service();

    // Act: парсим код с ОБОИМИ вариантами
    let code = r#"
Функция Тест()
    ТЗ1 = Новый ТаблицаЗначений;     // БЕЗ скобок
    ТЗ2 = Новый ТаблицаЗначений();   // СО скобками

    Кол1 = ТЗ1.Количество();
    Кол2 = ТЗ2.Количество();

    Возврат Кол1 + Кол2;
КонецФункции
"#;

    let parse_result = service.parse_and_validate(code).unwrap();
    assert!(parse_result.is_empty(), "Не должно быть syntax errors");

    let semantic_errors = service.validate_semantics(code, None).await.unwrap();

    // DEBUG: выводим ошибки если есть
    if !semantic_errors.is_empty() {
        for error in &semantic_errors {
            eprintln!("ERROR: {:?}", error);
        }
    }

    assert!(
        semantic_errors.is_empty(),
        "❌ Оба конструктора должны работать одинаково!"
    );
}
