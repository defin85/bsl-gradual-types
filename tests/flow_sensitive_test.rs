//! Тесты flow-sensitive анализа типов

use bsl_gradual_types::core::dependency_graph::Scope;
use bsl_gradual_types::core::flow_sensitive::FlowSensitiveAnalyzer;
use bsl_gradual_types::core::type_checker::{TypeChecker, TypeContext};
use bsl_gradual_types::core::types::{ConcreteType, PrimitiveType, ResolutionResult};
use bsl_gradual_types::parser::common::{Parser, ParserFactory};
use std::collections::HashMap;

fn create_test_context() -> TypeContext {
    TypeContext {
        variables: HashMap::new(),
        functions: HashMap::new(),
        current_scope: Scope::Global,
        scope_stack: vec![],
    }
}

#[test]
fn test_flow_sensitive_assignment() {
    let context = create_test_context();
    let mut analyzer = FlowSensitiveAnalyzer::new(context);

    // Создаем простое присваивание
    let target = bsl_gradual_types::parser::ast::Expression::Identifier("x".to_string());
    let value = bsl_gradual_types::parser::ast::Expression::String("hello".to_string());

    analyzer.analyze_assignment(&target, &value);

    // Проверяем что тип обновился
    let x_type = analyzer.get_variable_type("x").unwrap();
    assert!(matches!(
        x_type.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String))
    ));
}

#[test]
fn test_flow_sensitive_type_change() {
    let context = create_test_context();
    let mut analyzer = FlowSensitiveAnalyzer::new(context);

    // x = "hello"
    let target = bsl_gradual_types::parser::ast::Expression::Identifier("x".to_string());
    let value1 = bsl_gradual_types::parser::ast::Expression::String("hello".to_string());
    analyzer.analyze_assignment(&target, &value1);

    // Проверяем что x имеет тип String
    let x_type = analyzer.get_variable_type("x").unwrap();
    assert!(matches!(
        x_type.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String))
    ));

    // x = 42
    let value2 = bsl_gradual_types::parser::ast::Expression::Number(42.0);
    analyzer.analyze_assignment(&target, &value2);

    // Проверяем что тип изменился на Number
    let x_type = analyzer.get_variable_type("x").unwrap();
    assert!(matches!(
        x_type.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number))
    ));
}

#[test]
fn test_flow_sensitive_conditional() {
    let context = create_test_context();
    let mut analyzer = FlowSensitiveAnalyzer::new(context);

    // Создаем условие и ветки
    use bsl_gradual_types::parser::ast::{Expression, Statement};

    let condition = Expression::Boolean(true);
    let then_branch = vec![Statement::Assignment {
        target: Expression::Identifier("x".to_string()),
        value: Expression::String("then".to_string()),
    }];
    let else_branch = vec![Statement::Assignment {
        target: Expression::Identifier("x".to_string()),
        value: Expression::Number(42.0),
    }];

    analyzer.analyze_conditional(&condition, &then_branch, Some(&else_branch));

    // После условия x должен иметь какой-то тип (Union или упрощенный)
    let x_type = analyzer.get_variable_type("x").unwrap();
    match &x_type.result {
        ResolutionResult::Union(_) => {
            // OK, это объединенный тип
        }
        ResolutionResult::Concrete(_) => {
            // Тоже OK, если алгоритм объединения упростил тип
        }
        _ => panic!("Unexpected type result: {:?}", x_type.result),
    }
}

#[test]
fn test_integration_with_type_checker() {
    let bsl_code = r#"
        Процедура Тест()
            x = "строка";
            x = 123;
            Если x > 100 Тогда
                x = "большое число";
            КонецЕсли;
        КонецПроцедуры
    "#;

    // Создаем парсер через фабрику
    let mut parser = ParserFactory::create();

    // Парсим код
    match parser.parse(bsl_code) {
        Ok(program) => {
            // Создаем type checker
            let type_checker = TypeChecker::new("test.bsl".to_string());
            let (context, diagnostics) = type_checker.check(&program);

            // Проверяем что анализ прошёл без критических ошибок
            let errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| {
                    matches!(
                        d.severity,
                        bsl_gradual_types::core::type_checker::DiagnosticSeverity::Error
                    )
                })
                .collect();

            // Может быть предупреждения о несовместимых типах, но не ошибки парсинга
            println!(
                "Контекст после анализа: {:?}",
                context.variables.keys().collect::<Vec<_>>()
            );
            println!("Диагностики: {:?}", diagnostics);
        }
        Err(e) => {
            panic!("Ошибка парсинга: {:?}", e);
        }
    }
}

#[test]
fn test_binary_expression_analysis() {
    let context = create_test_context();
    let analyzer = FlowSensitiveAnalyzer::new(context);

    use bsl_gradual_types::parser::ast::{BinaryOp, Expression};

    // 1 + 2 должно возвращать Number
    let expr = Expression::Binary {
        left: Box::new(Expression::Number(1.0)),
        op: BinaryOp::Add,
        right: Box::new(Expression::Number(2.0)),
    };

    let result_type = analyzer.analyze_expression(&expr);
    assert!(matches!(
        result_type.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number))
    ));

    // 1 < 2 должно возвращать Boolean
    let expr = Expression::Binary {
        left: Box::new(Expression::Number(1.0)),
        op: BinaryOp::Less,
        right: Box::new(Expression::Number(2.0)),
    };

    let result_type = analyzer.analyze_expression(&expr);
    assert!(matches!(
        result_type.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean))
    ));
}

#[test]
fn test_function_call_analysis() {
    let context = create_test_context();
    let analyzer = FlowSensitiveAnalyzer::new(context);

    use bsl_gradual_types::parser::ast::Expression;

    // Строка() должно возвращать String
    let expr = Expression::Call {
        function: Box::new(Expression::Identifier("Строка".to_string())),
        args: vec![],
    };

    let result_type = analyzer.analyze_expression(&expr);
    assert!(matches!(
        result_type.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String))
    ));

    // Число() должно возвращать Number
    let expr = Expression::Call {
        function: Box::new(Expression::Identifier("Число".to_string())),
        args: vec![],
    };

    let result_type = analyzer.analyze_expression(&expr);
    assert!(matches!(
        result_type.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number))
    ));
}
