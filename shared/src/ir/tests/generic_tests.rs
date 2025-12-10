//! Тесты для Generic типов в IR

use crate::domain::types::{Certainty, ResolutionResult};
use crate::ir::{SemanticProgram, SymbolTable};

#[test]
fn test_initialize_as_generic() {
    let mut table = SymbolTable::new();

    table.initialize_as_generic(
        table.root_scope,
        "МассивСтрок".to_string(),
        "Массив".to_string(),
        1,
    );

    let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");

    assert!(resolution.is_some(), "Variable should exist");
    let res = resolution.unwrap();

    // Проверяем, что это Generic тип
    if let ResolutionResult::Generic(gen) = &res.result {
        assert_eq!(gen.base_type, "Массив");
        assert_eq!(gen.type_params.len(), 1);
    } else {
        panic!("Expected Generic resolution, got {:?}", res.result);
    }

    // Проверяем certainty = Inferred(0.0) (параметры неизвестны)
    assert!(matches!(res.certainty, Certainty::Inferred(c) if (c - 0.0).abs() < 0.001));
}

#[test]
fn test_update_generic_param() {
    let mut table = SymbolTable::new();

    // Инициализируем как Generic
    table.initialize_as_generic(
        table.root_scope,
        "МассивСтрок".to_string(),
        "Массив".to_string(),
        1,
    );

    // Обновляем параметр
    let success =
        table.update_generic_param(table.root_scope, "МассивСтрок", 0, "Строка".to_string());

    assert!(success, "update_generic_param должна вернуть true");

    let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");

    assert!(resolution.is_some());
    let res = resolution.unwrap();

    // Проверяем имя типа
    assert_eq!(res.type_name(), "Массив<Строка>");

    // Проверяем certainty = Known (все параметры известны)
    assert!(matches!(res.certainty, Certainty::Known));
}

#[test]
fn test_update_map_generic_params() {
    let mut table = SymbolTable::new();

    // Инициализируем Соответствие с 2 параметрами
    table.initialize_as_generic(
        table.root_scope,
        "Словарь".to_string(),
        "Соответствие".to_string(),
        2,
    );

    // Обновляем первый параметр (ключ)
    table.update_generic_param(table.root_scope, "Словарь", 0, "Строка".to_string());

    // Обновляем второй параметр (значение)
    table.update_generic_param(table.root_scope, "Словарь", 1, "Число".to_string());

    let resolution = table.get_variable_type(table.root_scope, "Словарь");

    assert!(resolution.is_some());
    let res = resolution.unwrap();

    // Проверяем имя типа
    assert_eq!(res.type_name(), "Соответствие<Строка, Число>");

    // Проверяем certainty = Known (все параметры известны)
    assert!(matches!(res.certainty, Certainty::Known));
}

#[test]
fn test_partial_generic_params() {
    let mut table = SymbolTable::new();

    // Инициализируем с 3 параметрами (не существует такого типа, но для теста)
    table.initialize_as_generic(
        table.root_scope,
        "МойТип".to_string(),
        "МойКонтейнер".to_string(),
        3,
    );

    // Обновляем только первый параметр
    table.update_generic_param(table.root_scope, "МойТип", 0, "Строка".to_string());

    let resolution = table.get_variable_type(table.root_scope, "МойТип");
    assert!(resolution.is_some());
    let res = resolution.unwrap();

    // Проверяем что тип - Generic с частично заполненными параметрами
    assert_eq!(
        res.type_name(),
        "МойКонтейнер<Строка, Неопределено, Неопределено>"
    );

    // Проверяем certainty - промежуточная уверенность (не все параметры заполнены)
    match &res.certainty {
        Certainty::Inferred(c) => assert!((*c - 0.5).abs() < 0.01),
        _ => panic!("Expected Inferred certainty for partial generic"),
    }
}

#[test]
fn test_generic_scope_hierarchy() {
    let mut program = SemanticProgram::new();
    let child_scope = program.symbols.create_scope(program.symbols.root_scope);

    // Регистрируем Generic переменную в root scope
    program.symbols.initialize_as_generic(
        program.symbols.root_scope,
        "МассивВРуте".to_string(),
        "Массив".to_string(),
        1,
    );

    // Регистрируем Generic переменную в child scope
    program.symbols.initialize_as_generic(
        child_scope,
        "МассивВОтпроцедуре".to_string(),
        "Массив".to_string(),
        1,
    );

    // Обновляем параметр в child scope
    program.symbols.update_generic_param(
        child_scope,
        "МассивВОтпроцедуре",
        0,
        "Число".to_string(),
    );

    // Проверяем видимость из child scope
    let resolution = program
        .symbols
        .get_variable_type(child_scope, "МассивВРуте");

    // Должна быть видна переменная из root scope
    assert!(
        resolution.is_some(),
        "Should see Generic variable from parent scope"
    );
    // Проверяем что это действительно Generic (начинается с "Массив<")
    assert!(resolution.unwrap().type_name().starts_with("Массив<"));

    // Проверяем, что child переменная не видна из root scope
    let hint = program
        .symbols
        .get_variable_type(program.symbols.root_scope, "МассивВОтпроцедуре");
    assert!(
        hint.is_none(),
        "Child scope variable should not be visible from parent"
    );
}
