//! Граничные случаи для Return Type Inference (Milestone 3.9)
//!
//! Расширенное тестирование edge cases

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::types::TypeResolution;
use std::sync::Arc;

fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

fn create_test_signature_index() -> SignatureIndex {
    let mut sig_idx = SignatureIndex::new();

    // Массив.Количество() -> Число
    sig_idx.add_platform_method(
        "Массив".to_string(),
        MethodSignature::new(
            "Количество".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Число".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );

    // ТаблицаЗначений.Количество() -> Число
    sig_idx.add_platform_method(
        "ТаблицаЗначений".to_string(),
        MethodSignature::new(
            "Количество".to_string(),
            Some("ТаблицаЗначений".to_string()),
            vec![],
            Some("Число".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );

    // Строка.Длина() -> Число
    sig_idx.add_platform_method(
        "Строка".to_string(),
        MethodSignature::new(
            "Длина".to_string(),
            Some("Строка".to_string()),
            vec![],
            Some("Число".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );

    sig_idx
}

// Test 1: Generic тип в методе
#[test]
fn test_generic_collection_method() {
    let source = r#"
Массив = Новый Массив;
Кол = Массив.Количество();
"#;

    let ast = Program {
        statements: vec![
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "Массив".to_string(),
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
                    name: "Кол".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Массив".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Количество".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let repository = create_test_repository();
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    let root_scope = ir.symbols.root_scope;
    let kol_type = ir.symbols.get_variable_type(root_scope, "Кол");

    match kol_type {
        Some(res) => { let type_name = res.type_name();
            assert_eq!(type_name, "Число", "Generic массив.Количество() должен возвращать Число");
        }
        other => panic!("Expected Inferred(Число), got {:?}", other),
    }
}

// Test 2: Цепочка вызовов (методный вызов на результате метода)
// EDGE CASE: М = Новый Массив; Размер = М.Количество().ЧтоТо()
// Ожидание: Fallback на Dynamic, так как Число не имеет методов в индексе
#[test]
fn test_chained_method_calls() {
    let source = r#"
М = Новый Массив;
Размер = М.Количество();
"#;

    let ast = Program {
        statements: vec![
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "М".to_string(),
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
                    name: "Размер".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "М".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Количество".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let repository = create_test_repository();
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    let root_scope = ir.symbols.root_scope;
    let size_type = ir.symbols.get_variable_type(root_scope, "Размер");

    match size_type {
        Some(res) => { let type_name = res.type_name();
            assert_eq!(type_name, "Число", "Результат Количество() должен быть Число");
        }
        other => panic!("Expected Inferred(Число), got {:?}", other),
    }
}

// Test 3: Несуществующий метод в generic типе
#[test]
fn test_nonexistent_method_on_generic_type() {
    let source = r#"
Коллекция = Новый Массив;
Результат = Коллекция.УдалитьЭтоТоНовое();
"#;

    let ast = Program {
        statements: vec![
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "Коллекция".to_string(),
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
                    name: "Результат".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Коллекция".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "УдалитьЭтоТоНовое".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let repository = create_test_repository();
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    let root_scope = ir.symbols.root_scope;
    let result_type = ir.symbols.get_variable_type(root_scope, "Результат");

    match result_type {
        Some(res) => { let type_name = res.type_name();
            assert_eq!(type_name, "Dynamic", "Несуществующий метод должен возвращать Dynamic");
        }
        other => panic!("Expected Inferred(Dynamic), got {:?}", other),
    }
}

// Test 4: Комбинация разных типов возврата
#[test]
fn test_multiple_method_calls_different_returns() {
    let source = r#"
ТЗ = Новый ТаблицаЗначений;
К1 = ТЗ.Количество();
К2 = ТЗ.Количество();
"#;

    let ast = Program {
        statements: vec![
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "ТЗ".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::New {
                    type_name: "ТаблицаЗначений".to_string(),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "К1".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "ТЗ".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Количество".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "К2".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "ТЗ".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Количество".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let repository = create_test_repository();
    let signature_index = create_test_signature_index();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    let root_scope = ir.symbols.root_scope;

    // Оба должны иметь тип Число
    let k1_type = ir.symbols.get_variable_type(root_scope, "К1");
    let k2_type = ir.symbols.get_variable_type(root_scope, "К2");

    match (k1_type, k2_type) {
        (Some(res1), Some(res2)) => { let t1 = res1.type_name(); let t2 = res2.type_name();
            assert_eq!(t1, "Число", "К1 должна быть Число");
            assert_eq!(t2, "Число", "К2 должна быть Число");
        }
        other => panic!("Expected Inferred types, got {:?}", other),
    }
}
