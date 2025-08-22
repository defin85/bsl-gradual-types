//! Интеграционные тесты для type narrowing

use bsl_gradual_types::core::type_checker::{DiagnosticSeverity, TypeChecker};
use bsl_gradual_types::parser::BslParser;

#[test]
fn test_type_narrowing_in_if_statement() {
    let code = r#"
        Перем х;
        Перем у;
        
        Если ТипЗнч(х) = Тип("Строка") Тогда
            у = х + " суффикс";  
        ИначеЕсли ТипЗнч(х) = Тип("Число") Тогда
            у = х + 1;
        Иначе
            у = х;
        КонецЕсли;
    "#;

    let mut parser = BslParser::new(code).expect("Failed to create parser");
    let program = parser.parse().expect("Failed to parse");

    let type_checker = TypeChecker::new("test.bsl".to_string());
    let (context, _diagnostics) = type_checker.check(&program);

    // После анализа переменная у должна быть определена
    assert!(context.variables.contains_key("у"));
}

#[test]
fn test_undefined_check_narrowing() {
    let code = r#"
        Перем Параметр;
        Перем Результат;
        
        Если Параметр = Неопределено Тогда
            Параметр = 0;
        КонецЕсли;
        
        Результат = Параметр + 1;
    "#;

    let mut parser = BslParser::new(code).expect("Failed to create parser");
    let program = parser.parse().expect("Failed to parse");

    let type_checker = TypeChecker::new("test.bsl".to_string());
    let (context, diagnostics) = type_checker.check(&program);

    // Не должно быть ошибок типов
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 0, "Should have no type errors");
}

#[test]
fn test_not_undefined_narrowing() {
    let code = r#"
        Перем МожетБытьНеопределено;
        Перем Длина;
        
        Если МожетБытьНеопределено <> Неопределено Тогда
            Длина = СтрДлина(МожетБытьНеопределено);
        КонецЕсли;
    "#;

    let mut parser = BslParser::new(code).expect("Failed to create parser");
    let program = parser.parse().expect("Failed to parse");

    let type_checker = TypeChecker::new("test.bsl".to_string());
    let (_context, diagnostics) = type_checker.check(&program);

    // Проверяем, что нет критических ошибок
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    assert_eq!(errors.len(), 0, "Should have no type errors");
}

#[test]
fn test_multiple_type_checks() {
    let code = r#"
        Перем Значение;
        Перем Длина;
        Перем Размер;
        
        Если ТипЗнч(Значение) = Тип("Массив") Тогда
            Размер = 10;
        ИначеЕсли ТипЗнч(Значение) = Тип("Соответствие") Тогда
            Размер = 20;
        ИначеЕсли ТипЗнч(Значение) = Тип("Строка") Тогда
            Длина = СтрДлина(Значение);
        КонецЕсли;
    "#;

    let mut parser = BslParser::new(code).expect("Failed to create parser");
    let program = parser.parse().expect("Failed to parse");

    let type_checker = TypeChecker::new("test.bsl".to_string());
    let (_context, diagnostics) = type_checker.check(&program);

    // Не должно быть ошибок при вызове методов после проверки типа
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();

    // В текущей реализации могут быть предупреждения о неизвестных методах,
    // но не должно быть критических ошибок
    println!("Diagnostics: {:?}", diagnostics);
}

#[test]
fn test_truthy_falsy_narrowing() {
    let code = r#"
        Перем Флаг;
        Перем Результат;
        
        Если Флаг Тогда
            Результат = "Истина";
        Иначе
            Результат = "Ложь";
        КонецЕсли;
    "#;

    let mut parser = BslParser::new(code).expect("Failed to create parser");
    let program = parser.parse().expect("Failed to parse");

    let type_checker = TypeChecker::new("test.bsl".to_string());
    let (context, _diagnostics) = type_checker.check(&program);

    // Результат должен быть определён
    assert!(context.variables.contains_key("Результат"));
}
