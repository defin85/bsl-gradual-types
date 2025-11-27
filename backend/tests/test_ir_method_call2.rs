//! Unit тесты для исправления критического бага P0:
//! Переменные НЕ добавлялись в SymbolTable при присваивании
//!
//! Проблема: МассивСтрок = Новый Массив(); → переменная не попадала в scope
//! Решение: Assignment теперь вызывает register_variable() если переменная не существует

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::TypeResolution;
use std::sync::Arc;

/// Helper функция - создаёт TypeRepository с базовыми типами
fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Helper функция - создаёт пустой SignatureIndex
fn create_test_signature_index() -> SignatureIndex {
    SignatureIndex::new()
}

/// ✅ КРИТИЧЕСКИЙ ТЕСТ: Проверка IR узлов для вызова метода
#[test]
fn test_ir_nodes_for_method_call() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![
                Statement::Assignment {
                    target: Expression::Identifier {
                        name: "ТаблицаЗначенійТип".to_string(),
                        span: AstSpan::stub(),
                    },
                    value: Expression::New {
                        type_name: "ТаблицаЗначеній".to_string(),
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
                                name: "ТаблицаЗначенійТип".to_string(),
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
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .expect("IR conversion failed");

    println!("\n=== IR NODES ===");
    for (i, node) in ir.nodes.iter().enumerate() {
        println!(
            "[{}] {:?} span=[{},{}..{},{}]",
            i,
            std::mem::discriminant(&node.kind),
            node.span.start_line,
            node.span.start_column,
            node.span.end_line,
            node.span.end_column
        );

        // Детали для FunctionCall
        if let bsl_shared::ir::SemanticNodeKind::FunctionCall {
            function_name,
            object_name,
            object_type,
            ..
        } = &node.kind
        {
            println!(
                "    FunctionCall: {} | object_name={:?} | object_type={:?}",
                function_name, object_name, object_type
            );
        }

        // Детали для Assignment
        if let bsl_shared::ir::SemanticNodeKind::Assignment {
            variable,
            value_type,
            value_node,
        } = &node.kind
        {
            println!(
                "    Assignment: {} = {} | value_node={:?}",
                variable, value_type, value_node
            );
        }
    }
}
#[test]
fn test_assignment_declares_variable_in_symbol_table() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![Statement::Assignment {
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
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\n  МассивСтрок = Новый Массив();\nКонецПроцедуры".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Найти scope процедуры Тест
    let _procedure_scope = ir
        .symbols
        .scopes
        .values()
        .find(|s| s.parent == Some(ir.symbols.root_scope))
        .expect("Procedure scope not found");

    let scope_id = ir
        .symbols
        .scopes
        .iter()
        .find(|(_, s)| s.parent == Some(ir.symbols.root_scope))
        .map(|(id, _)| *id)
        .expect("Procedure scope ID not found");

    // ✅ КРИТИЧНО: Переменная ДОЛЖНА быть в SymbolTable
    let var_type = ir.symbols.get_variable_type(scope_id, "МассивСтрок");
    assert!(
        var_type.is_some(),
        "❌ FAILED: Переменная МассивСтрок НЕ добавлена в SymbolTable!"
    );

    // Проверить тип: должен быть Generic<Массив, [?]>
    let res = var_type.unwrap();
    assert_eq!(res.type_name(), "Массив<Неопределено>", "Type должен быть Массив<Неопределено>");

    println!("✅ PASSED: Переменная МассивСтрок добавлена в SymbolTable как Generic<Массив, [?]>");
}

/// ✅ Проверка для Соответствие (2 Generic параметра)
#[test]
fn test_assignment_map_generic_type() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![Statement::Assignment {
                target: Expression::Identifier {
                    name: "СоответствиеДанных".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::New {
                    type_name: "Соответствие".to_string(),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            }],
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\n  СоответствиеДанных = Новый Соответствие();\nКонецПроцедуры"
            .to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Найти scope процедуры
    let scope_id = ir
        .symbols
        .scopes
        .iter()
        .find(|(_, s)| s.parent == Some(ir.symbols.root_scope))
        .map(|(id, _)| *id)
        .expect("Procedure scope not found");

    let var_type = ir.symbols.get_variable_type(scope_id, "СоответствиеДанных");
    assert!(var_type.is_some(), "Переменная НЕ найдена в SymbolTable");

    let res = var_type.unwrap();
    assert_eq!(res.type_name(), "Соответствие<Неопределено, Неопределено>", "Type должен быть Соответствие<Неопределено, Неопределено>");

    println!("✅ PASSED: Соответствие инициализировано как Generic<Соответствие, [?, ?]>");
}

/// ✅ Проверка для обычного типа (не Generic)
#[test]
fn test_assignment_explicit_type() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![Statement::Assignment {
                target: Expression::Identifier {
                    name: "Строка1".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::String {
                    value: "текст".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            }],
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\n  Строка1 = \"текст\";\nКонецПроцедуры".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Найти scope процедуры
    let scope_id = ir
        .symbols
        .scopes
        .iter()
        .find(|(_, s)| s.parent == Some(ir.symbols.root_scope))
        .map(|(id, _)| *id)
        .expect("Procedure scope not found");

    let var_type = ir.symbols.get_variable_type(scope_id, "Строка1");
    assert!(var_type.is_some(), "Переменная НЕ найдена в SymbolTable");

    let res = var_type.unwrap();
    assert_eq!(res.type_name(), "Строка");

    println!("✅ PASSED: Строка1 инициализирована как Inferred(Строка)");
}

/// ✅ Проверка обновления типа (flow-sensitive) для уже объявленной переменной
#[test]
fn test_assignment_updates_existing_variable() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![
                Statement::VarDeclaration {
                    name: "x".to_string(),
                    type_hint: Some("Число".to_string()),
                    span: AstSpan::stub(),
                },
                Statement::Assignment {
                    target: Expression::Identifier {
                        name: "x".to_string(),
                        span: AstSpan::stub(),
                    },
                    value: Expression::String {
                        value: "новый тип".to_string(),
                        span: AstSpan::stub(),
                    },
                    span: AstSpan::stub(),
                },
            ],
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\n  Перем x: Число;\n  x = \"новый тип\";\nКонецПроцедуры".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Найти scope процедуры
    let scope_id = ir
        .symbols
        .scopes
        .iter()
        .find(|(_, s)| s.parent == Some(ir.symbols.root_scope))
        .map(|(id, _)| *id)
        .expect("Procedure scope not found");

    let var_type = ir.symbols.get_variable_type(scope_id, "x");
    assert!(var_type.is_some(), "Переменная НЕ найдена в SymbolTable");

    // После assignment тип ДОЛЖЕН измениться с "Число" на "Строка"
    let res = var_type.unwrap();
    assert_eq!(res.type_name(), "Строка", "Тип не обновился!");

    println!("✅ PASSED: Flow-sensitive обновление типа работает");
}

/// ✅ Проверка множественных переменных в одной процедуре
#[test]
fn test_multiple_assignments_in_scope() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![
                Statement::Assignment {
                    target: Expression::Identifier {
                        name: "МассивЧисел".to_string(),
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
                        name: "МассивСтрок".to_string(),
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
                        name: "Имя".to_string(),
                        span: AstSpan::stub(),
                    },
                    value: Expression::String {
                        value: "текст".to_string(),
                        span: AstSpan::stub(),
                    },
                    span: AstSpan::stub(),
                },
            ],
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\n  МассивЧисел = Новый Массив();\n  МассивСтрок = Новый Массив();\n  Имя = \"текст\";\nКонецПроцедуры"
            .to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Найти scope процедуры
    let scope_id = ir
        .symbols
        .scopes
        .iter()
        .find(|(_, s)| s.parent == Some(ir.symbols.root_scope))
        .map(|(id, _)| *id)
        .expect("Procedure scope not found");

    // Все 3 переменные ДОЛЖНЫ быть в SymbolTable
    assert!(ir
        .symbols
        .get_variable_type(scope_id, "МассивЧисел")
        .is_some());
    assert!(ir
        .symbols
        .get_variable_type(scope_id, "МассивСтрок")
        .is_some());
    assert!(ir.symbols.get_variable_type(scope_id, "Имя").is_some());

    println!("✅ PASSED: Множественные переменные добавлены в SymbolTable");
}
