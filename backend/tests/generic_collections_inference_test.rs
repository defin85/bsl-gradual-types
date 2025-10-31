//! Integration тесты для Generic Collections Inference
//!
//! Проверяют flow-sensitive вывод Generic параметров коллекций.

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::ir::TypeHint;
use std::sync::Arc;

#[test]
fn test_array_initialization_as_generic() {
    let source = r#"МассивСтрок = Новый Массив();"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "МассивСтрок".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::New {
                type_name: "Массив".to_string(),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let repository = Arc::new(InMemoryTypeRepository::new());
    let program =
        AstToIrConverter::convert(ast, source.to_string(), "test.bsl".to_string(), repository)
            .expect("Failed to convert AST to IR");

    // Ищем root scope и проверяем переменную МассивСтрок
    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "МассивСтрок");

    // Проверяем, что это Generic тип
    match var_type {
        Some(TypeHint::Generic {
            base_type,
            type_params,
            certainty,
        }) => {
            assert_eq!(base_type, "Массив", "Base type должен быть Массив");
            assert_eq!(type_params.len(), 1, "Должен быть 1 параметр");
            assert_eq!(type_params[0], "?", "Параметр должен быть неизвестен");
            assert_eq!(
                certainty, 0.0,
                "Certainty должна быть 0 (параметры неизвестны)"
            );
        }
        Some(hint) => panic!("Expected Generic hint, got {:?}", hint),
        None => panic!("Variable МассивСтрок not found in scope"),
    }
}

#[test]
fn test_map_initialization_as_generic() {
    let source = r#"Словарь = Новый Соответствие();"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Словарь".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::New {
                type_name: "Соответствие".to_string(),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let repository = Arc::new(InMemoryTypeRepository::new());
    let program =
        AstToIrConverter::convert(ast, source.to_string(), "test.bsl".to_string(), repository)
            .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "Словарь");

    // Проверяем, что это Generic тип с 2 параметрами
    match var_type {
        Some(TypeHint::Generic {
            base_type,
            type_params,
            certainty,
        }) => {
            assert_eq!(
                base_type, "Соответствие",
                "Base type должен быть Соответствие"
            );
            assert_eq!(type_params.len(), 2, "Должно быть 2 параметра");
            assert_eq!(
                type_params[0], "?",
                "Первый параметр (ключ) должен быть неизвестен"
            );
            assert_eq!(
                type_params[1], "?",
                "Второй параметр (значение) должен быть неизвестен"
            );
            assert_eq!(certainty, 0.0, "Certainty должна быть 0");
        }
        Some(hint) => panic!("Expected Generic hint, got {:?}", hint),
        None => panic!("Variable Словарь not found in scope"),
    }
}

#[test]
fn test_list_initialization_as_generic() {
    let source = r#"Список = Новый Список();"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Список".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::New {
                type_name: "Список".to_string(),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let repository = Arc::new(InMemoryTypeRepository::new());
    let program =
        AstToIrConverter::convert(ast, source.to_string(), "test.bsl".to_string(), repository)
            .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "Список");

    match var_type {
        Some(TypeHint::Generic {
            base_type,
            type_params,
            ..
        }) => {
            assert_eq!(base_type, "Список");
            assert_eq!(type_params.len(), 1);
            assert_eq!(type_params[0], "?");
        }
        Some(hint) => panic!("Expected Generic hint, got {:?}", hint),
        None => panic!("Variable Список not found"),
    }
}

#[test]
fn test_non_generic_types_not_converted() {
    let source = r#"Х = Новый Число();"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Х".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::New {
                type_name: "Число".to_string(),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let repository = Arc::new(InMemoryTypeRepository::new());
    let program =
        AstToIrConverter::convert(ast, source.to_string(), "test.bsl".to_string(), repository)
            .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "Х");

    // Для непривычных типов должно быть Inferred, не Generic
    match var_type {
        Some(TypeHint::Inferred(type_name)) => {
            assert_eq!(type_name, "Число");
        }
        Some(hint) => panic!("Expected Inferred hint, got {:?}", hint),
        None => panic!("Variable Х not found"),
    }
}

#[test]
fn test_multiple_arrays_have_independent_types() {
    let source = r#"
        Х = Новый Массив();
        Y = Новый Массив();
    "#;

    let ast = Program {
        statements: vec![
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "Х".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::New {
                    type_name: "Массив".to_string(),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "Y".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::New {
                    type_name: "Массив".to_string(),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let repository = Arc::new(InMemoryTypeRepository::new());
    let program =
        AstToIrConverter::convert(ast, source.to_string(), "test.bsl".to_string(), repository)
            .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;

    // Обе переменные должны быть Generic
    let x_type = program.symbols.get_variable_type(root_scope, "Х");
    let y_type = program.symbols.get_variable_type(root_scope, "Y");

    assert!(
        matches!(x_type, Some(TypeHint::Generic { .. })),
        "Х должен быть Generic"
    );
    assert!(
        matches!(y_type, Some(TypeHint::Generic { .. })),
        "Y должен быть Generic"
    );
}
