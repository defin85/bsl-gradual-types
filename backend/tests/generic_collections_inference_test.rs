//! Integration тесты для Generic Collections Inference
//!
//! Проверяют flow-sensitive вывод Generic параметров коллекций.

use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::SignatureIndex;
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
    let signature_index = SignatureIndex::new();
    let program = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Failed to convert AST to IR");

    // Ищем root scope и проверяем переменную МассивСтрок
    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "МассивСтрок");

    // Проверяем, что это Generic тип
    let res = var_type.expect("Variable МассивСтрок not found in scope");
    assert_eq!(
        res.type_name(),
        "Массив<Неопределено>",
        "Type должен быть Массив<Неопределено>"
    );
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
    let signature_index = SignatureIndex::new();
    let program = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "Словарь");

    // Проверяем, что это Generic тип с 2 параметрами
    let res = var_type.expect("Variable Словарь not found in scope");
    assert_eq!(
        res.type_name(),
        "Соответствие<Неопределено, Неопределено>",
        "Type должен быть Соответствие<Неопределено, Неопределено>"
    );
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
    let signature_index = SignatureIndex::new();
    let program = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "Список");

    let res = var_type.expect("Variable Список not found");
    assert_eq!(
        res.type_name(),
        "Список<Неопределено>",
        "Type должен быть Список<Неопределено>"
    );
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
    let signature_index = SignatureIndex::new();
    let program = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;
    let var_type = program.symbols.get_variable_type(root_scope, "Х");

    // Для непривычных типов должно быть Inferred, не Generic
    let res = var_type.expect("Variable Х not found");
    assert_eq!(res.type_name(), "Число");
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
    let signature_index = SignatureIndex::new();
    let program = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Failed to convert AST to IR");

    let root_scope = program.symbols.root_scope;

    // Обе переменные должны быть Generic
    let x_type = program.symbols.get_variable_type(root_scope, "Х");
    let y_type = program.symbols.get_variable_type(root_scope, "Y");

    // Обе переменные должны быть Generic (содержать "<" в type_name)
    assert!(x_type.is_some(), "Х должен быть определён");
    assert!(
        x_type.unwrap().type_name().contains("<"),
        "Х должен быть Generic"
    );
    assert!(y_type.is_some(), "Y должен быть определён");
    assert!(
        y_type.unwrap().type_name().contains("<"),
        "Y должен быть Generic"
    );
}
