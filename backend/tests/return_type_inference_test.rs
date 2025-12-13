//! Unit тесты для Milestone 3.9 - Return Type Inference
//!
//! Проверяют вывод типа результата вызова платформенных методов и функций

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_id::TypeId;
use std::sync::Arc;

/// Helper функция - создаёт TypeRepository
fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Helper функция - создаёт SignatureIndex с платформенными методами
fn create_test_signature_index_with_methods() -> SignatureIndex {
    let mut sig_idx = SignatureIndex::new();

    // Метод Массив.Количество() -> "Число"
    sig_idx.add_platform_method(
        TypeId::new("Массив"),
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

    // Метод ТаблицаЗначений.Количество() -> "Число"
    sig_idx.add_platform_method(
        TypeId::new("ТаблицаЗначений"),
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

    // Глобальная функция ТипЗнч() -> "Тип"
    sig_idx.add_global_function(
        TypeId::new("ТипЗнч"),
        MethodSignature::new(
            "ТипЗнч".to_string(),
            None,
            vec![],
            Some("Тип".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );

    // Void метод (процедура) Сообщить() -> None
    sig_idx.add_global_function(
        TypeId::new("Сообщить"),
        MethodSignature::new(
            "Сообщить".to_string(),
            None,
            vec![],
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );

    sig_idx
}

#[test]
fn test_method_return_type_basic() {
    // Тест: Кол = ТЗ.Количество() → Кол: Число
    let source = r#"
ТЗ = Новый ТаблицаЗначений;
Кол = ТЗ.Количество();
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
                    name: "Кол".to_string(),
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
    let signature_index = create_test_signature_index_with_methods();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    // Проверяем тип переменной Кол
    let root_scope = ir.symbols.root_scope;
    let kol_type = ir.symbols.get_variable_type(root_scope, "Кол");

    match kol_type {
        Some(res) => { let type_name = res.type_name();
            assert_eq!(type_name, "Число", "Тип должен быть Число");
        }
        other => panic!("Expected Inferred(Число), got {:?}", other),
    }
}

#[test]
fn test_global_function_return_type() {
    // Тест: Тип = ТипЗнч(ТЗ) → Тип: Тип
    let source = r#"
ТЗ = Новый ТаблицаЗначений;
Тип = ТипЗнч(ТЗ);
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
                    name: "Тип".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::Identifier {
                        name: "ТипЗнч".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![Expression::Identifier {
                        name: "ТЗ".to_string(),
                        span: AstSpan::stub(),
                    }],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let repository = create_test_repository();
    let signature_index = create_test_signature_index_with_methods();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    // Проверяем тип переменной Тип
    let root_scope = ir.symbols.root_scope;
    let tip_type = ir.symbols.get_variable_type(root_scope, "Тип");

    match tip_type {
        Some(res) => { let type_name = res.type_name();
            assert_eq!(type_name, "Тип", "Тип должен быть Тип");
        }
        other => panic!("Expected Inferred(Тип), got {:?}", other),
    }
}

#[test]
fn test_void_method_return_type() {
    // Тест: Void метод → "Неопределено"
    let source = r#"Сообщить("Привет");"#;

    let ast = Program {
        statements: vec![Statement::Call {
            expression: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "Сообщить".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![Expression::String {
                    value: "Привет".to_string(),
                    span: AstSpan::stub(),
                }],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let repository = create_test_repository();
    let signature_index = create_test_signature_index_with_methods();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    // Проверяем, что IR успешно создан
    assert_eq!(ir.nodes.len(), 1);
}

#[test]
fn test_nonexistent_method_fallback() {
    // Тест: Несуществующий метод → fallback на "Dynamic"
    let source = r#"
ТЗ = Новый ТаблицаЗначений;
Результат = ТЗ.НесуществующийМетод();
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
                    name: "Результат".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "ТЗ".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "НесуществующийМетод".to_string(),
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
    let signature_index = create_test_signature_index_with_methods();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    // Проверяем тип переменной Результат
    let root_scope = ir.symbols.root_scope;
    let result_type = ir.symbols.get_variable_type(root_scope, "Результат");

    match result_type {
        Some(res) => { let type_name = res.type_name();
            assert_eq!(type_name, "Dynamic", "Несуществующий метод должен возвращать Dynamic");
        }
        other => panic!("Expected Inferred(Dynamic), got {:?}", other),
    }
}

#[test]
fn test_case_insensitive_method_lookup() {
    // Тест: Case-insensitive поиск метода
    let source = r#"
ТЗ = Новый ТаблицаЗначений;
Кол = ТЗ.количество();
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
                    name: "Кол".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "ТЗ".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "количество".to_string(),
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
    let signature_index = create_test_signature_index_with_methods();
    let ir = AstToIrConverter::convert(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    // Проверяем тип переменной Кол (case-insensitive)
    let root_scope = ir.symbols.root_scope;
    let kol_type = ir.symbols.get_variable_type(root_scope, "Кол");

    match kol_type {
        Some(res) => { let type_name = res.type_name();
            assert_eq!(type_name, "Число", "Case-insensitive поиск должен работать");
        }
        other => panic!("Expected Inferred(Число), got {:?}", other),
    }
}
