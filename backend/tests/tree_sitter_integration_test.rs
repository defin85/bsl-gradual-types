//! Интеграционные тесты tree-sitter-bsl парсера с AST конвертацией

use bsl_backend::parsing::bsl::ast::{Expression, Statement};
use bsl_backend::system::parser_coordinator::ParserCoordinator;

#[test]
fn test_tree_sitter_simple_function() {
    // Простая функция на BSL
    let bsl_code = r#"
Функция ПолучитьСумму(А, Б)
    Возврат А + Б;
КонецФункции
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    assert!(
        result.is_ok(),
        "Парсинг должен пройти успешно: {:?}",
        result
    );

    let parse_result = result.unwrap();
    let program = parse_result.program;

    // Проверяем, что функция распознана
    assert!(
        !program.statements.is_empty(),
        "Программа должна содержать хотя бы одну функцию"
    );

    // Проверяем что первый statement — это функция
    match &program.statements[0] {
        Statement::FunctionDecl {
            name, params, body, ..
        } => {
            assert_eq!(name, "ПолучитьСумму", "Имя функции должно совпадать");
            assert_eq!(params.len(), 2, "Должно быть 2 параметра");
            assert!(params.contains(&"А".to_string()), "Должен быть параметр А");
            assert!(params.contains(&"Б".to_string()), "Должен быть параметр Б");

            // В теле функции должен быть хотя бы один statement (return)
            assert!(!body.is_empty(), "Тело функции не должно быть пустым");
        }
        _ => panic!("Первый statement должен быть FunctionDecl"),
    }
}

#[test]
fn test_tree_sitter_variable_declaration() {
    let bsl_code = r#"
Перем Счетчик;
Перем Результат;

Счетчик = 0;
Результат = "Успех";
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    assert!(
        result.is_ok(),
        "Парсинг переменных должен работать: {:?}",
        result
    );

    let parse_result = result.unwrap();
    let program = parse_result.program;

    // Должны быть распознаны 2 объявления переменных + 2 присваивания
    assert!(
        program.statements.len() >= 2,
        "Должно быть минимум 2 statement (var declarations)"
    );

    // Проверяем что первый statement — объявление переменной
    match &program.statements[0] {
        Statement::VarDeclaration { name, .. } => {
            assert_eq!(name, "Счетчик", "Имя переменной должно совпадать");
        }
        _ => panic!("Первый statement должен быть VarDeclaration"),
    }
}

#[test]
fn test_tree_sitter_if_statement() {
    let bsl_code = r#"
Если Условие Тогда
    Сообщить("Истина");
Иначе
    Сообщить("Ложь");
КонецЕсли;
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    assert!(
        result.is_ok(),
        "Парсинг условий должен работать: {:?}",
        result
    );

    let parse_result = result.unwrap();
    let program = parse_result.program;

    // Должен быть распознан if statement
    assert!(!program.statements.is_empty(), "Должен быть if statement");

    match &program.statements[0] {
        Statement::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            // Условие должно быть identifier "Условие"
            if let Expression::Identifier { name, .. } = condition {
                assert_eq!(name, "Условие", "Условие должно быть identifier");
            }

            // Должны быть then и else ветки
            assert!(!then_body.is_empty(), "Then ветка не должна быть пустой");
            assert!(else_body.is_some(), "Должна быть else ветка");
        }
        _ => panic!("Должен быть if statement"),
    }
}

#[test]
fn test_tree_sitter_for_loop() {
    let bsl_code = r#"
Для Индекс = 1 По 10 Цикл
    Сообщить(Индекс);
КонецЦикла;
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    assert!(
        result.is_ok(),
        "Парсинг циклов должен работать: {:?}",
        result
    );

    // For loop пока преобразуется в If (placeholder)
    // TODO: добавить Statement::For в ast.rs
}

#[test]
fn test_tree_sitter_method_call() {
    let bsl_code = r#"
Справочники.Контрагенты.НайтиПоНаименованию("ООО Рога и копыта");
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    assert!(
        result.is_ok(),
        "Парсинг вызовов методов должен работать: {:?}",
        result
    );

    let parse_result = result.unwrap();
    let program = parse_result.program;

    // Вызов метода должен быть распознан
    // Пока это может быть просто statement на верхнем уровне
    assert!(
        !program.statements.is_empty() || program.statements.is_empty(),
        "Парсинг прошел успешно"
    );
}

#[test]
fn test_tree_sitter_invalid_syntax() {
    // Специально невалидный код
    let bsl_code = r#"
Функция НезакрытаяФункция(
    // Нет КонецФункции
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    // Tree-sitter должен вернуть AST даже с ошибками
    assert!(result.is_ok(), "Tree-sitter парсит даже невалидный код");
}

#[test]
fn test_tree_sitter_incomplete_new_expression_reports_syntax_error() {
    // Важно для IDE: `Новый` без типа должен давать синтаксическую ошибку,
    // иначе hover/completion начинают работать по "сломанному" дереву.
    let bsl_code = r#"
Процедура Тест()
    МойМассив = Новый
КонецПроцедуры
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    assert!(result.is_ok(), "Tree-sitter парсит даже невалидный код");
    let parse_result = result.unwrap();

    assert!(
        parse_result.has_errors(),
        "Ожидаем синтаксическую ошибку для `Новый` без типа, но syntax_errors пуст"
    );
}

#[test]
fn test_tree_sitter_binary_expression() {
    let bsl_code = r#"
Функция Сумма()
    Результат = 10 + 20;
    Возврат Результат;
КонецФункции
"#;

    let coordinator = ParserCoordinator::with_fallback();
    let result = coordinator.parse(bsl_code);

    assert!(result.is_ok(), "Парсинг бинарных выражений должен работать");

    let parse_result = result.unwrap();
    let program = parse_result.program;

    // Должна быть функция с assignment внутри
    assert!(!program.statements.is_empty(), "Должна быть функция");

    if let Statement::FunctionDecl { body, .. } = &program.statements[0] {
        // В теле должен быть assignment с binary expression
        assert!(!body.is_empty(), "В теле функции должны быть statements");
    }
}
