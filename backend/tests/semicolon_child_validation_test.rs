//! Интеграционный тест для проверки исправленного semicolon validation
//! (переход от текстового поиска к проверке дочерних узлов)

use bsl_backend::system::parser_coordinator::ParserCoordinator;

#[test]
fn test_semicolon_as_child_node_correct_code() {
    let coordinator = ParserCoordinator::with_fallback();

    // ✅ Корректный код с точками с запятой
    let source = r#"
Функция Тест()
    МассивДанных = Новый Массив();
    МассивДанных.Добавить(42);
    Возврат МассивДанных;
КонецФункции
"#;

    let result = coordinator.parse(source).unwrap();

    // НЕ должно быть ошибок про отсутствие точек с запятой
    assert!(
        !result.has_errors(),
        "Корректный код НЕ должен вызывать ошибки semicolon validation"
    );
}

#[test]
fn test_semicolon_as_child_node_missing_semicolons() {
    let coordinator = ParserCoordinator::with_fallback();

    // ❌ Код БЕЗ точек с запятой (должен вызывать ошибки)
    let source = r#"
Функция Тест()
    МассивДанных = Новый Массив()
    МассивДанных.Добавить(42)
    Возврат МассивДанных
КонецФункции
"#;

    let result = coordinator.parse(source).unwrap();

    // ДОЛЖНЫ быть ошибки про отсутствие точек с запятой
    assert!(
        result.has_errors(),
        "Код без точек с запятой ДОЛЖЕН вызывать ошибки"
    );

    // Проверяем что это именно ошибки про точки с запятой
    let semicolon_errors: Vec<_> = result
        .syntax_errors
        .iter()
        .filter(|e| e.message.contains("точка с запятой") || e.message.contains("точки с запятой"))
        .collect();

    assert!(
        !semicolon_errors.is_empty(),
        "Должны быть сообщения об отсутствии точек с запятой"
    );

    println!(
        "✅ Найдено {} ошибок про точки с запятой",
        semicolon_errors.len()
    );
    for error in &semicolon_errors {
        println!("  - {}", error.message);
    }
}

#[test]
fn test_semicolon_detection_with_tree_sitter_structure() {
    let coordinator = ParserCoordinator::with_fallback();

    // Проверяем что точка с запятой действительно является дочерним узлом
    let source = r#"
Число = 42;
"#;

    let result = coordinator.parse(source).unwrap();

    // Этот код корректен - не должно быть ошибок
    assert!(
        !result.has_errors(),
        "Простое присваивание с ';' НЕ должно вызывать ошибки. Ошибки: {:?}",
        result.syntax_errors
    );
}

#[test]
fn test_last_statement_before_endfunction_can_skip_semicolon() {
    let coordinator = ParserCoordinator::with_fallback();

    // Последний statement перед КонецФункции может не иметь точку с запятой
    let source = r#"
Функция Тест()
    Переменная = 123;
    Возврат Переменная
КонецФункции
"#;

    let result = coordinator.parse(source).unwrap();

    // НЕ должно быть ошибок - последний statement может быть без ';'
    let semicolon_errors: Vec<_> = result
        .syntax_errors
        .iter()
        .filter(|e| e.message.contains("точка с запятой") || e.message.contains("точки с запятой"))
        .collect();

    assert!(
        semicolon_errors.is_empty(),
        "Последний statement перед КонецФункции может не иметь ';'. Ошибки: {:?}",
        semicolon_errors
    );
}
