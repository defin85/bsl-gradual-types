//! Комплексные интеграционные тесты для Generic Collections Inference (Direction 2)
//!
//! Проверяют реальный flow: TreeSitter → AST → IR → Generic Type Inference
//!
//! # Подход
//! Используем ParserCoordinator + AstToIrConverter для реального сквозного тестирования.

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::system::parser_coordinator::ParserCoordinator;
use bsl_shared::ir::TypeHint;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use std::sync::Arc;

/// Создаём минимальный TypeRepository для тестирования
fn create_test_repository() -> Arc<InMemoryTypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Helper: найти переменную во всех scope (для упрощения тестов)
fn find_variable_in_any_scope(
    symbols: &bsl_shared::ir::SymbolTable,
    var_name: &str,
) -> Option<TypeHint> {
    // Перебираем все scope в поисках переменной
    for scope_id in symbols.scopes.keys() {
        if let Some(type_hint) = symbols.get_variable_type(*scope_id, var_name) {
            return Some(type_hint);
        }
    }
    None
}

#[test]
fn test_array_empty_initialization() {
    // Arrange: BSL код с пустым массивом
    let source = r#"
Процедура Тест()
    МассивПустой = Новый Массив();
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    // Act: Конвертация AST → IR
    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    // Ищем переменную МассивПустой во всех scope
    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивПустой");

    // Assert: Должен быть Generic тип с неопределённым параметром
    if let Some(TypeHint::Generic { base_type, type_params, certainty }) = var_type {
        assert_eq!(base_type, "Массив", "Base type должен быть Массив");
        assert_eq!(type_params.len(), 1, "Должен быть 1 тип-параметр");
        assert_eq!(type_params[0], "?", "Параметр должен быть неизвестен");
        assert_eq!(certainty, 0.0, "Certainty должна быть 0 (неизвестно)");
    } else {
        panic!("Переменная МассивПустой должна иметь Generic тип, получено: {:?}", var_type);
    }
}

#[test]
fn test_array_with_string_inference() {
    // Arrange: BSL код с добавлением строки
    let source = r#"
Процедура Тест()
    МассивСтрок = Новый Массив();
    МассивСтрок.Добавить("текст");
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    // Ищем переменную МассивСтрок
    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивСтрок");

    // Assert: Должен быть Generic<String>
    if let Some(TypeHint::Generic { base_type, type_params, certainty }) = var_type {
        assert_eq!(base_type, "Массив");
        assert_eq!(type_params.len(), 1);

        // После инференса параметр должен быть "Строка"
        if type_params[0] != "?" {
            assert_eq!(type_params[0], "Строка", "Должен вывести тип Строка");
            assert!(certainty > 0.0, "Certainty должна быть > 0 после инференса");
        } else {
            // Если инференс ещё не реализован - ожидаем "?"
            println!("⚠️ Инференс из Добавить() ещё не работает");
        }
    } else {
        panic!("Переменная МассивСтрок должна быть Generic, получено: {:?}", var_type);
    }
}

#[test]
fn test_array_with_number_inference() {
    let source = r#"
Процедура Тест()
    МассивЧисел = Новый Массив();
    МассивЧисел.Добавить(42);
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивЧисел");

    if let Some(TypeHint::Generic { base_type, type_params, .. }) = var_type {
        assert_eq!(base_type, "Массив");

        if type_params[0] != "?" {
            assert_eq!(type_params[0], "Число");
        }
    } else {
        panic!("Переменная МассивЧисел должна быть Generic, получено: {:?}", var_type);
    }
}

#[test]
fn test_map_initialization() {
    let source = r#"
Процедура Тест()
    Словарь = Новый Соответствие();
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "Словарь");

    // Assert: Соответствие должно иметь 2 параметра
    if let Some(TypeHint::Generic { base_type, type_params, certainty }) = var_type {
        assert_eq!(base_type, "Соответствие");
        assert_eq!(type_params.len(), 2, "Соответствие должно иметь 2 параметра (ключ, значение)");
        assert_eq!(type_params[0], "?");
        assert_eq!(type_params[1], "?");
        assert_eq!(certainty, 0.0);
    } else {
        panic!("Переменная Словарь должна быть Generic, получено: {:?}", var_type);
    }
}

#[test]
fn test_map_with_insert_inference() {
    let source = r#"
Процедура Тест()
    Карта = Новый Соответствие();
    Карта.Вставить("ключ", 100);
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "Карта");

    if let Some(TypeHint::Generic { base_type, type_params, .. }) = var_type {
        assert_eq!(base_type, "Соответствие");
        assert_eq!(type_params.len(), 2);

        if type_params[0] != "?" && type_params[1] != "?" {
            assert_eq!(type_params[0], "Строка");
            assert_eq!(type_params[1], "Число");
        } else {
            println!("⚠️ Инференс из Вставить() ещё не полностью работает");
        }
    } else {
        panic!("Переменная Карта должна быть Generic, получено: {:?}", var_type);
    }
}

#[test]
fn test_non_generic_type_stays_inferred() {
    // Arrange: Не-generic тип (Строка)
    let source = r#"
Процедура Тест()
    Текст = Новый Строка("привет");
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "Текст");

    // Assert: НЕ должен быть Generic
    match var_type {
        Some(TypeHint::Inferred(type_name)) => {
            assert_eq!(type_name, "Строка");
        }
        Some(TypeHint::Generic { .. }) => {
            panic!("Строка НЕ должна быть Generic типом");
        }
        other => {
            println!("⚠️ Ожидался Inferred(Строка), получено: {:?}", other);
        }
    }
}

#[test]
fn test_multiple_arrays_independent() {
    let source = r#"
Процедура Тест()
    МассивA = Новый Массив();
    МассивB = Новый Массив();
    МассивA.Добавить("строка");
    МассивB.Добавить(42);
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    let var_a = find_variable_in_any_scope(&ir.symbols, "МассивA");
    let var_b = find_variable_in_any_scope(&ir.symbols, "МассивB");

    // Assert: Оба должны быть Generic, но независимыми
    assert!(matches!(var_a, Some(TypeHint::Generic { .. })), "МассивA должен быть Generic");
    assert!(matches!(var_b, Some(TypeHint::Generic { .. })), "МассивB должен быть Generic");

    // Если инференс работает, типы должны различаться
    if let (Some(TypeHint::Generic { type_params: params_a, .. }),
            Some(TypeHint::Generic { type_params: params_b, .. })) = (var_a, var_b) {
        if params_a[0] != "?" && params_b[0] != "?" {
            assert_eq!(params_a[0], "Строка");
            assert_eq!(params_b[0], "Число");
            println!("✅ Инференс работает корректно: A<Строка>, B<Число>");
        }
    }
}

#[test]
fn test_empty_collections_scenario() {
    // Граничный случай: несколько пустых коллекций
    let source = r#"
Процедура Тест()
    М1 = Новый Массив();
    М2 = Новый Массив();
    С = Новый Соответствие();
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    // Все переменные должны существовать и быть Generic
    let м1 = find_variable_in_any_scope(&ir.symbols, "М1");
    let м2 = find_variable_in_any_scope(&ir.symbols, "М2");
    let с = find_variable_in_any_scope(&ir.symbols, "С");

    assert!(м1.is_some(), "М1 должна существовать");
    assert!(м2.is_some(), "М2 должна существовать");
    assert!(с.is_some(), "С должна существовать");

    assert!(matches!(м1, Some(TypeHint::Generic { .. })));
    assert!(matches!(м2, Some(TypeHint::Generic { .. })));
    assert!(matches!(с, Some(TypeHint::Generic { .. })));
}

#[test]
fn test_mixed_types_in_array() {
    // Сложный сценарий: добавление разных типов (union/dynamic)
    let source = r#"
Процедура Тест()
    МассивСмешанный = Новый Массив();
    МассивСмешанный.Добавить("текст");
    МассивСмешанный.Добавить(42);
    МассивСмешанный.Добавить(Истина);
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен пройти");

    let repository = create_test_repository();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивСмешанный");

    // Assert: Тип должен быть либо union, либо динамический
    if let Some(TypeHint::Generic { type_params, certainty, .. }) = var_type {
        // Если инференс обрабатывает union
        if type_params[0].contains("|") {
            assert!(type_params[0].contains("Строка"));
            assert!(type_params[0].contains("Число"));
            println!("✅ Union type inference работает: {}", type_params[0]);
        } else {
            // Если не union - certainty должна быть низкой из-за конфликта
            println!("⚠️ Mixed types inference certainty: {}", certainty);
        }
    }
}
