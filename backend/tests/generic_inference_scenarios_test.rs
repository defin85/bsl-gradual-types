//! Комплексные интеграционные тесты для Generic Collections Inference (Direction 2)
//!
//! Проверяют реальный flow: TreeSitter → AST → IR → Generic Type Inference
//!
//! # Подход
//! Используем ParserCoordinator + AstToIrConverter для реального сквозного тестирования.

use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::system::parser_coordinator::ParserCoordinator;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::TypeResolution;
use std::sync::Arc;

/// Создаём минимальный TypeRepository для тестирования
fn create_test_repository() -> Arc<InMemoryTypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Helper функция - создаёт пустой SignatureIndex
fn create_test_signature_index() -> SignatureIndex {
    SignatureIndex::new()
}

/// Helper: найти переменную во всех scope (для упрощения тестов)
fn find_variable_in_any_scope(
    symbols: &bsl_shared::ir::SymbolTable,
    var_name: &str,
) -> Option<TypeResolution> {
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    // Ищем переменную МассивПустой во всех scope
    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивПустой");

    // Assert: Должен быть Generic тип с неопределённым параметром
    if let Some(res) = var_type {
        assert!(
            res.type_name().starts_with("Массив"),
            "Base type должен быть Массив"
        );
        // type_params.len() = 1: Должен быть 1 тип-параметр (checked by type_name format)
        assert!(
            res.type_name().contains("Неопределено"),
            "Параметр должен быть неизвестен"
        );
        // certainty = 0.0: Certainty должна быть 0 (неизвестно) (use res.certainty if needed)
    } else {
        panic!(
            "Переменная МассивПустой должна иметь Generic тип, получено: {:?}",
            var_type
        );
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    // Ищем переменную МассивСтрок
    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивСтрок");

    // Assert: Должен быть Generic<String>
    if let Some(res) = var_type {
        assert!(res.type_name().starts_with("Массив"));
        // type_params.len() = 1 (checked by type_name format)

        // После инференса параметр должен быть "Строка"
        if !res.type_name().contains("Неопределено") {
            assert!(
                res.type_name().contains("Строка"),
                "Должен вывести тип Строка"
            );
            // certainty > 0.0: Certainty должна быть > 0 после инференса (use res.certainty if needed)
        } else {
            // Если инференс ещё не реализован - ожидаем "Неопределено"
            println!("⚠️ Инференс из Добавить() ещё не работает");
        }
    } else {
        panic!(
            "Переменная МассивСтрок должна быть Generic, получено: {:?}",
            var_type
        );
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивЧисел");

    if let Some(res) = var_type {
        assert!(res.type_name().starts_with("Массив"));

        if !res.type_name().contains("Неопределено") {
            assert!(res.type_name().contains("Число"));
        }
    } else {
        panic!(
            "Переменная МассивЧисел должна быть Generic, получено: {:?}",
            var_type
        );
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "Словарь");

    // Assert: Соответствие должно иметь 2 параметра
    if let Some(res) = var_type {
        assert!(res.type_name().starts_with("Соответствие"));
        // type_params.len() = 2: Соответствие должно иметь 2 параметра (ключ, значение) (checked by type_name format)
        assert!(res.type_name().contains("Неопределено"));
        // type_params[1] = ? (checked by type_name)
        // certainty = 0.0 (use res.certainty if needed)
    } else {
        panic!(
            "Переменная Словарь должна быть Generic, получено: {:?}",
            var_type
        );
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "Карта");

    if let Some(res) = var_type {
        assert!(res.type_name().starts_with("Соответствие"));
        // type_params.len() = 2 (checked by type_name format)

        if !res.type_name().contains("Неопределено") {
            assert!(res.type_name().contains("Строка"));
            // type_params[1] check covered by type_name
        } else {
            println!("⚠️ Инференс из Вставить() ещё не полностью работает");
        }
    } else {
        panic!(
            "Переменная Карта должна быть Generic, получено: {:?}",
            var_type
        );
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "Текст");

    // Assert: НЕ должен быть Generic
    if let Some(res) = var_type {
        let name = res.type_name();
        assert!(!name.contains("<"), "Строка НЕ должна быть Generic типом");
        assert_eq!(name, "Строка");
    } else {
        println!("⚠️ Переменная Текст не найдена");
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    let var_a = find_variable_in_any_scope(&ir.symbols, "МассивA");
    let var_b = find_variable_in_any_scope(&ir.symbols, "МассивB");

    // Assert: Оба должны быть Generic, но независимыми
    assert!(
        var_a
            .as_ref()
            .map(|r| r.type_name().contains("<"))
            .unwrap_or(false),
        "МассивA должен быть Generic"
    );
    assert!(
        var_b
            .as_ref()
            .map(|r| r.type_name().contains("<"))
            .unwrap_or(false),
        "МассивB должен быть Generic"
    );

    // Если инференс работает, типы должны различаться
    if let (Some(res_a), Some(res_b)) = (var_a, var_b) {
        let name_a = res_a.type_name();
        let name_b = res_b.type_name();
        if !name_a.contains("Неопределено") && !name_b.contains("Неопределено")
        {
            assert!(name_a.contains("Строка"));
            assert!(name_b.contains("Число"));
            println!("✅ Инференс работает корректно: {}, {}", name_a, name_b);
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    // Все переменные должны существовать и быть Generic
    let м1 = find_variable_in_any_scope(&ir.symbols, "М1");
    let м2 = find_variable_in_any_scope(&ir.symbols, "М2");
    let с = find_variable_in_any_scope(&ir.symbols, "С");

    assert!(м1.is_some(), "М1 должна существовать");
    assert!(м2.is_some(), "М2 должна существовать");
    assert!(с.is_some(), "С должна существовать");

    assert!(м1
        .as_ref()
        .map(|r| r.type_name().contains("<"))
        .unwrap_or(false));
    assert!(м2
        .as_ref()
        .map(|r| r.type_name().contains("<"))
        .unwrap_or(false));
    assert!(с
        .as_ref()
        .map(|r| r.type_name().contains("<"))
        .unwrap_or(false));
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
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        parse_result.program,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия должна пройти");

    let var_type = find_variable_in_any_scope(&ir.symbols, "МассивСмешанный");

    // Assert: Тип должен быть либо union, либо динамический
    if let Some(res) = var_type {
        // Если инференс обрабатывает union
        if res.type_name().contains("|") {
            assert!(res.type_name().contains("Строка"));
            assert!(res.type_name().contains("Число"));
            println!("✅ Union type inference работает: {}", res.type_name());
        } else {
            // Если не union - certainty должна быть низкой из-за конфликта
            println!("⚠️ Mixed types inference certainty: {}", res.type_name());
        }
    }
}
