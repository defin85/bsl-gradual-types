//! Unit тесты для проверки корректности извлечения `object_name` в `FunctionCall`
//!
//! Критичное замечание 2.1 из code review для Milestone 3.5 Phase 1-2
//! Проверяем, что:
//! - object_name = Some(name) для простых переменных (Identifier)
//! - object_name = None для сложных выражений (PropertyAccess, Call, New, etc.)
//!
//! ВАЖНО: В текущей архитектуре вызов метода (obj.Method()) создает FunctionCall,
//! а не MemberAccess. MemberAccess используется только для доступа к свойствам без вызова.

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::ir::SemanticNodeKind;
use std::sync::Arc;

/// Helper функция - создаёт TypeRepository с базовыми типами
fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

#[test]
fn test_function_call_extracts_object_name_from_identifier() {
    // Простой случай: Переменная.Метод()
    let source = "МассивДанных.Добавить(\"текст\");";

    let ast = Program {
        statements: vec![Statement::Call {
            expression: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "МассивДанных".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Добавить".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![Expression::String {
                    value: "текст".to_string(),
                    span: AstSpan::stub(),
                }],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
    )
    .unwrap();

    let func_node = ir
        .nodes
        .iter()
        .find(|n| matches!(n.kind, SemanticNodeKind::FunctionCall { .. }))
        .expect("FunctionCall node not found");

    if let SemanticNodeKind::FunctionCall {
        function_name,
        object_name,
        ..
    } = &func_node.kind
    {
        assert_eq!(function_name, "Добавить");
        assert_eq!(
            object_name,
            &Some("МассивДанных".to_string()),
            "✅ object_name should be Some(МассивДанных)"
        );
    } else {
        panic!("Expected FunctionCall node");
    }

    println!("✅ PASSED: test_function_call_extracts_object_name_from_identifier");
}

#[test]
fn test_function_call_extracts_object_name() {
    // Вызов метода: Переменная.Метод(args)
    let ast = Program {
        statements: vec![Statement::Call {
            expression: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "Словарь".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Вставить".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![Expression::String {
                    value: "ключ".to_string(),
                    span: AstSpan::stub(),
                }],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Словарь.Вставить(\"ключ\");".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
    )
    .unwrap();

    let func_node = ir
        .nodes
        .iter()
        .find(|n| matches!(n.kind, SemanticNodeKind::FunctionCall { .. }))
        .expect("FunctionCall node not found");

    if let SemanticNodeKind::FunctionCall {
        function_name,
        object_name,
        ..
    } = &func_node.kind
    {
        assert_eq!(function_name, "Вставить");
        assert_eq!(
            object_name,
            &Some("Словарь".to_string()),
            "✅ object_name should be Some(Словарь)"
        );
    } else {
        panic!("Expected FunctionCall node");
    }

    println!("✅ PASSED: test_function_call_extracts_object_name");
}

#[test]
fn test_function_call_complex_expression_returns_none() {
    // Сложный случай: obj.prop1.Метод()
    // object_name должен быть None, так как это не простая переменная

    let ast = Program {
        statements: vec![Statement::Call {
            expression: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "obj".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "prop1".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Метод".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "obj.prop1.Метод();".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
    )
    .unwrap();

    let func_node = ir
        .nodes
        .iter()
        .find(|n| matches!(n.kind, SemanticNodeKind::FunctionCall { .. }))
        .expect("FunctionCall node not found");

    if let SemanticNodeKind::FunctionCall { object_name, .. } = &func_node.kind {
        // Для сложного выражения object_name должен быть None
        assert_eq!(
            object_name, &None,
            "✅ object_name should be None for complex expressions"
        );
    }

    println!("✅ PASSED: test_function_call_complex_expression_returns_none");
}

#[test]
fn test_function_call_with_new_expression_returns_none() {
    // Случай: (Новый Массив()).Метод()
    // object_name должен быть None, так как это выражение конструктора

    let ast = Program {
        statements: vec![Statement::Call {
            expression: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::New {
                        type_name: "Массив".to_string(),
                        args: vec![],
                        span: AstSpan::stub(),
                    }),
                    property: "Добавить".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![Expression::String {
                    value: "текст".to_string(),
                    span: AstSpan::stub(),
                }],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "(Новый Массив()).Добавить(\"текст\");".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
    )
    .unwrap();

    let func_node = ir
        .nodes
        .iter()
        .find(|n| matches!(n.kind, SemanticNodeKind::FunctionCall { .. }))
        .expect("FunctionCall node not found");

    if let SemanticNodeKind::FunctionCall { object_name, .. } = &func_node.kind {
        assert_eq!(
            object_name, &None,
            "✅ object_name should be None for New expression"
        );
    }

    println!("✅ PASSED: test_function_call_with_new_expression_returns_none");
}

#[test]
fn test_function_call_with_function_call_returns_none() {
    // Случай: ПолучитьОбъект().Метод()
    // object_name должен быть None, так как это результат вызова функции

    let ast = Program {
        statements: vec![Statement::Call {
            expression: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::Call {
                        function: Box::new(Expression::Identifier {
                            name: "ПолучитьОбъект".to_string(),
                            span: AstSpan::stub(),
                        }),
                        args: vec![],
                        span: AstSpan::stub(),
                    }),
                    property: "Метод".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "ПолучитьОбъект().Метод();".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
    )
    .unwrap();

    let func_node = ir
        .nodes
        .iter()
        .find(|n| matches!(n.kind, SemanticNodeKind::FunctionCall { .. }))
        .expect("FunctionCall node not found");

    if let SemanticNodeKind::FunctionCall { object_name, .. } = &func_node.kind {
        assert_eq!(
            object_name, &None,
            "✅ object_name should be None for function call result"
        );
    }

    println!("✅ PASSED: test_function_call_with_function_call_returns_none");
}

#[test]
fn test_multiple_function_calls_in_sequence() {
    // Случай: несколько вызовов методов подряд
    let source = r#"
        Массив1.Добавить("1");
        Массив2.Добавить("2");
    "#;

    let ast = Program {
        statements: vec![
            Statement::Call {
                expression: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Массив1".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Добавить".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![Expression::String {
                        value: "1".to_string(),
                        span: AstSpan::stub(),
                    }],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            Statement::Call {
                expression: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Массив2".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Добавить".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![Expression::String {
                        value: "2".to_string(),
                        span: AstSpan::stub(),
                    }],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
    )
    .unwrap();

    // Должно быть 2 FunctionCall узла
    let func_nodes: Vec<_> = ir
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, SemanticNodeKind::FunctionCall { .. }))
        .collect();

    assert_eq!(func_nodes.len(), 2, "Should have 2 FunctionCall nodes");

    // Проверяем первый узел
    if let SemanticNodeKind::FunctionCall { object_name, .. } = &func_nodes[0].kind {
        assert_eq!(object_name, &Some("Массив1".to_string()));
    }

    // Проверяем второй узел
    if let SemanticNodeKind::FunctionCall { object_name, .. } = &func_nodes[1].kind {
        assert_eq!(object_name, &Some("Массив2".to_string()));
    }

    println!("✅ PASSED: test_multiple_function_calls_in_sequence");
}
