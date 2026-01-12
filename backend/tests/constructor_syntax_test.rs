/// Тест для проблемы #1: Новый ТаблицаЗначений vs Новый ТаблицаЗначений()
///
/// В 1С допустимы ОБА синтаксиса для создания объектов:
/// - `Новый ТаблицаЗначений` (без скобок)
/// - `Новый ТаблицаЗначений()` (со скобками)
///
/// Проверяем что оба варианта корректно парсятся и сохраняют TypeResolution.
mod support;

use bsl_shared::domain::types::DiagnosticSeverity;

#[tokio::test]
async fn test_constructor_without_parentheses() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    // Act: парсим код БЕЗ скобок
    let code = r#"
Функция Тест()
    ТЗ = Новый ТаблицаЗначений;
    Возврат ТЗ.Количество();
КонецФункции
"#;

    let parse_result = support::syntax_diagnostics_for_code(deps_bundle.as_ref(), "test.bsl", code);
    assert!(
        parse_result.is_empty(),
        "Не должно быть syntax errors для 'Новый ТаблицаЗначений'"
    );

    // Проверяем что semantic validation НЕ выдаёт ошибку
    let semantic_diagnostics =
        support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "test.bsl", code);
    let errors: Vec<_> = semantic_diagnostics
        .iter()
        .filter(|diag| diag.severity == DiagnosticSeverity::Error)
        .collect();

    assert!(
        errors.is_empty(),
        "❌ Метод 'Количество' должен быть найден для 'ТаблицаЗначений'"
    );
}

#[tokio::test]
async fn test_constructor_with_parentheses() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    // Act: парсим код СО скобками
    let code = r#"
Функция Тест()
    ТЗ = Новый ТаблицаЗначений();
    Возврат ТЗ.Количество();
КонецФункции
"#;

    let parse_result = support::syntax_diagnostics_for_code(deps_bundle.as_ref(), "test.bsl", code);
    assert!(
        parse_result.is_empty(),
        "Не должно быть syntax errors для 'Новый ТаблицаЗначений()'"
    );

    // Проверяем что semantic validation НЕ выдаёт ошибку
    let semantic_diagnostics =
        support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "test.bsl", code);
    let errors: Vec<_> = semantic_diagnostics
        .iter()
        .filter(|diag| diag.severity == DiagnosticSeverity::Error)
        .collect();

    assert!(
        errors.is_empty(),
        "✅ Метод 'Количество' должен быть найден для 'ТаблицаЗначений()'"
    );
}

#[tokio::test]
async fn test_both_constructors_in_one_function() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

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

    let parse_result = support::syntax_diagnostics_for_code(deps_bundle.as_ref(), "test.bsl", code);
    assert!(parse_result.is_empty(), "Не должно быть syntax errors");

    let semantic_diagnostics =
        support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "test.bsl", code);
    let errors: Vec<_> = semantic_diagnostics
        .iter()
        .filter(|diag| diag.severity == DiagnosticSeverity::Error)
        .collect();

    assert!(
        errors.is_empty(),
        "❌ Оба конструктора должны работать одинаково!"
    );
}
