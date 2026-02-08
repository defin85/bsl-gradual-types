//! Тесты для модуля ast_to_ir
//!
//! Тесты организованы по функциональности:
//! - Базовые тесты конвертации
//! - Тесты GlobalPropertyAccess
//! - Тесты директив компиляции

use std::sync::Arc;

use bsl_shared::domain::code_location::CompilerDirective;
use bsl_shared::domain::flow_analysis::FlowAnalysisContext;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{
    Certainty, ConcreteType, ResolutionMetadata, ResolutionResult, ResolutionSource, TypeResolution,
};
use bsl_shared::domain::NullSafetyAnalyzer;
use bsl_shared::ir::SemanticNodeKind;
use bsl_shared::ir::{CfgNodeKind, EdgeKind};

use bsl_syntax::ast::{Expression, Program, Span as AstSpan, Statement};

use super::AstToIrConverter;

/// Helper функция для тестов - создаёт пустой TypeRepository
fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Helper функция для тестов - создаёт пустой SignatureIndex
fn create_test_signature_index() -> SignatureIndex {
    SignatureIndex::new()
}

// =============================================================================
// Базовые тесты конвертации
// =============================================================================

#[test]
fn test_variable_declaration_conversion() {
    let ast = Program {
        statements: vec![Statement::VarDeclaration {
            name: "x".to_string(),
            type_hint: Some("Число".to_string()),
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Перем x: Число;".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    assert_eq!(ir.nodes.len(), 1);
    if let SemanticNodeKind::VariableDeclaration {
        name, type_hint, ..
    } = &ir.nodes[0].kind
    {
        assert_eq!(name, "x");
        assert!(type_hint.is_some());
        assert_eq!(type_hint.as_deref(), Some("Число"));
    } else {
        panic!("Expected VariableDeclaration");
    }
}

#[test]
fn test_cfg_present_for_declarations_only() {
    let ast = Program {
        statements: vec![Statement::FunctionDecl {
            name: "Empty".to_string(),
            params: vec![],
            body: vec![],
            compiler_directive: None,
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Функция Empty()\nКонецФункции".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let cfg = ir.cfg.expect("CFG must always be present");
    assert_eq!(cfg.nodes().len(), 2);
    assert_eq!(cfg.edges().len(), 1);
    assert!(cfg
        .nodes()
        .iter()
        .any(|n| matches!(n.kind, CfgNodeKind::Entry)));
    assert!(cfg
        .nodes()
        .iter()
        .any(|n| matches!(n.kind, CfgNodeKind::Exit)));
}

#[test]
fn test_cfg_present_for_root_level_assignment() {
    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "x".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::Number {
                value: 42.0,
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "x = 42;".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let cfg = ir
        .cfg
        .expect("CFG must be built for root-level executable code");
    assert!(cfg
        .nodes()
        .iter()
        .any(|n| matches!(n.kind, CfgNodeKind::Entry)));
    assert!(cfg
        .nodes()
        .iter()
        .any(|n| matches!(n.kind, CfgNodeKind::Exit)));
}

#[test]
fn test_cfg_present_for_function_body() {
    let ast = Program {
        statements: vec![Statement::FunctionDecl {
            name: "TestFunc".to_string(),
            params: vec![],
            body: vec![Statement::Assignment {
                target: Expression::Identifier {
                    name: "x".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Number {
                    value: 1.0,
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            }],
            compiler_directive: None,
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Функция TestFunc()\n  x = 1;\nКонецФункции".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let cfg = ir
        .cfg
        .expect("CFG must be built for non-empty function body");
    assert!(cfg.nodes().len() >= 3); // Entry + stmt + Exit (или больше)
    assert!(cfg.edges().len() >= 2);
    assert!(cfg
        .nodes()
        .iter()
        .any(|n| matches!(n.kind, CfgNodeKind::Assignment { .. })));
}

#[test]
fn test_cfg_contains_conditional_edges_for_if_statement() {
    let ast = Program {
        statements: vec![Statement::FunctionDecl {
            name: "TestFunc".to_string(),
            params: vec![],
            body: vec![Statement::If {
                condition: Expression::Boolean {
                    value: true,
                    span: AstSpan::stub(),
                },
                then_body: vec![Statement::Assignment {
                    target: Expression::Identifier {
                        name: "x".to_string(),
                        span: AstSpan::stub(),
                    },
                    value: Expression::Number {
                        value: 1.0,
                        span: AstSpan::stub(),
                    },
                    span: AstSpan::stub(),
                }],
                else_body: Some(vec![Statement::Assignment {
                    target: Expression::Identifier {
                        name: "x".to_string(),
                        span: AstSpan::stub(),
                    },
                    value: Expression::Number {
                        value: 2.0,
                        span: AstSpan::stub(),
                    },
                    span: AstSpan::stub(),
                }]),
                span: AstSpan::stub(),
            }],
            compiler_directive: None,
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Функция TestFunc()\nЕсли Истина Тогда\n  x = 1;\nИначе\n  x = 2;\nКонецЕсли\nКонецФункции"
            .to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let cfg = ir.cfg.expect("CFG must be built for if statement");
    assert!(cfg
        .edges()
        .iter()
        .any(|e| e.kind == EdgeKind::ConditionalTrue));
    assert!(cfg
        .edges()
        .iter()
        .any(|e| e.kind == EdgeKind::ConditionalFalse));
}

#[test]
fn test_null_safety_can_run_on_v2_cfg_method_call() {
    let ast = Program {
        statements: vec![Statement::FunctionDecl {
            name: "TestFunc".to_string(),
            params: vec![],
            body: vec![Statement::Call {
                expression: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "x".to_string(),
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
            compiler_directive: None,
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Функция TestFunc()\n  x.Метод();\nКонецФункции".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let cfg = ir.cfg.expect("CFG must be built for function body");
    assert!(cfg
        .nodes()
        .iter()
        .any(|n| matches!(n.kind, CfgNodeKind::MethodCall { .. })));

    let mut ctx = FlowAnalysisContext::new();
    ctx.set_variable(
        "x",
        TypeResolution {
            result: ResolutionResult::nullable(ConcreteType::string()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        },
    );

    let mut analyzer = NullSafetyAnalyzer::new(cfg);
    let result = analyzer.analyze(&ctx);
    assert!(!result.warnings.is_empty());
}

#[test]
fn test_if_statement_with_scope() {
    let ast = Program {
        statements: vec![Statement::If {
            condition: Expression::Boolean {
                value: true,
                span: AstSpan::stub(),
            },
            then_body: vec![Statement::VarDeclaration {
                name: "y".to_string(),
                type_hint: None,
                span: AstSpan::stub(),
            }],
            else_body: None,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Если Истина Тогда Перем y; КонецЕсли".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Должно быть 2 узла: IfStatement + VariableDeclaration
    assert_eq!(ir.nodes.len(), 2);

    // Должно быть 2 scope: root + then branch
    assert_eq!(ir.symbols.scopes.len(), 2);
}

#[test]
fn test_function_call_with_args() {
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

    let ir = AstToIrConverter::convert(
        ast,
        "Сообщить(\"Привет\");".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    assert_eq!(ir.nodes.len(), 1);
    if let SemanticNodeKind::FunctionCall {
        function_name,
        object_name,
        object_node,
        ..
    } = &ir.nodes[0].kind
    {
        assert_eq!(function_name, "Сообщить");
        assert!(object_name.is_none());
        assert_eq!(*object_node, None);
    } else {
        panic!("Expected FunctionCall");
    }
}

#[test]
fn test_nested_scopes() {
    let ast = Program {
        statements: vec![
            Statement::VarDeclaration {
                name: "global".to_string(),
                type_hint: Some("Строка".to_string()),
                span: AstSpan::stub(),
            },
            Statement::FunctionDecl {
                name: "TestFunc".to_string(),
                params: vec![],
                body: vec![Statement::VarDeclaration {
                    name: "local".to_string(),
                    type_hint: Some("Число".to_string()),
                    span: AstSpan::stub(),
                }],
                compiler_directive: None,
                is_export: false,
                span: AstSpan::stub(),
            },
        ],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Перем global: Строка;\nФункция TestFunc()\n  Перем local: Число;\nКонецФункции"
            .to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Должно быть 3 scope: root + function body
    assert!(ir.symbols.scopes.len() >= 2);

    // Глобальная переменная должна быть в root scope
    // Используем публичный API вместо прямого доступа
    assert!(ir.symbols.has_variable(ir.symbols.root_scope, "global"));
}

#[test]
fn test_function_body_indices() {
    let ast = Program {
        statements: vec![Statement::FunctionDecl {
            name: "TestFunc".to_string(),
            params: vec![],
            body: vec![
                Statement::VarDeclaration {
                    name: "local".to_string(),
                    type_hint: Some("Число".to_string()),
                    span: AstSpan::stub(),
                },
                Statement::Assignment {
                    target: Expression::Identifier {
                        name: "local".to_string(),
                        span: AstSpan::stub(),
                    },
                    value: Expression::Number {
                        value: 42.0,
                        span: AstSpan::stub(),
                    },
                    span: AstSpan::stub(),
                },
            ],
            compiler_directive: None,
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Функция TestFunc()\n  Перем local: Число;\n  local = 42;\nКонецФункции".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Проверяем, что есть 3 узла: 2 внутренних + FunctionDeclaration
    assert_eq!(ir.nodes.len(), 3);

    // Проверяем, что FunctionDeclaration содержит индексы тела
    if let SemanticNodeKind::FunctionDeclaration { body, .. } = &ir.nodes[2].kind {
        assert_eq!(body.len(), 2); // VariableDeclaration + Assignment
        assert_eq!(body[0], 0); // Индекс первого узла тела
        assert_eq!(body[1], 1); // Индекс второго узла тела
    } else {
        panic!("Expected FunctionDeclaration at nodes[2]");
    }
}

// =============================================================================
// Тесты директив компиляции
// =============================================================================

/// Test helper: Парсит директиву компиляции из текста
/// TODO: Будет переинтегрирована в основной код при полной реализации context tracking
fn parse_compiler_directive_test(text: &str) -> CompilerDirective {
    let text_lower = text.to_lowercase();

    // Порядок проверки важен: сначала более длинные директивы
    if text_lower.contains("&наклиентенасерверебезконтекста")
        || text_lower.contains("&atclientatservernocontext")
    {
        CompilerDirective::OnClientOnServerNoContext
    } else if text_lower.contains("&насерверебезконтекста")
        || text_lower.contains("&atservernocontext")
    {
        CompilerDirective::OnServerNoContext
    } else if text_lower.contains("&насервере") || text_lower.contains("&atserver") {
        CompilerDirective::OnServer
    } else if text_lower.contains("&наклиенте") || text_lower.contains("&atclient") {
        CompilerDirective::OnClient
    } else {
        CompilerDirective::Unknown
    }
}

#[test]
fn test_parse_server_directive() {
    let text = "&НаСервере\nПроцедура Тест()\nКонецПроцедуры";
    let directive = parse_compiler_directive_test(text);
    assert_eq!(directive, CompilerDirective::OnServer);
}

#[test]
fn test_parse_client_directive() {
    let text = "&НаКлиенте\nПроцедура Тест()\nКонецПроцедуры";
    let directive = parse_compiler_directive_test(text);
    assert_eq!(directive, CompilerDirective::OnClient);
}

#[test]
fn test_parse_server_no_context_directive() {
    let text = "&НаСервереБезКонтекста\nПроцедура Тест()\nКонецПроцедуры";
    let directive = parse_compiler_directive_test(text);
    assert_eq!(directive, CompilerDirective::OnServerNoContext);
}

#[test]
fn test_parse_universal_directive() {
    let text = "&НаКлиентеНаСервереБезКонтекста\nПроцедура Тест()\nКонецПроцедуры";
    let directive = parse_compiler_directive_test(text);
    assert_eq!(directive, CompilerDirective::OnClientOnServerNoContext);
}

#[test]
fn test_parse_no_directive() {
    let text = "Процедура Тест()\nКонецПроцедуры";
    let directive = parse_compiler_directive_test(text);
    assert_eq!(directive, CompilerDirective::Unknown);
}

#[test]
fn test_parse_english_directive() {
    let text = "&AtServer\nProcedure Test()\nEndProcedure";
    let directive = parse_compiler_directive_test(text);
    assert_eq!(directive, CompilerDirective::OnServer);
}

// =============================================================================
// Тесты GlobalPropertyAccess
// =============================================================================

#[test]
fn test_global_property_access_for_справочники() {
    // Тест: Справочники.Контрагенты создаёт GlobalPropertyAccess + MemberAccess
    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Менеджер".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::PropertyAccess {
                object: Box::new(Expression::Identifier {
                    name: "Справочники".to_string(),
                    span: AstSpan::stub(),
                }),
                property: "Контрагенты".to_string(),
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Менеджер = Справочники.Контрагенты;".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Должно быть 3 узла:
    // 0: GlobalPropertyAccess { name: "Справочники" }
    // 1: MemberAccess { object_node: Some(0), member_name: "Контрагенты" }
    // 2: Assignment { variable: "Менеджер", value_node: Some(1) }
    assert_eq!(ir.nodes.len(), 3);

    // Проверяем GlobalPropertyAccess
    if let SemanticNodeKind::GlobalPropertyAccess { name } = &ir.nodes[0].kind {
        assert_eq!(name, "Справочники");
    } else {
        panic!(
            "Expected GlobalPropertyAccess at nodes[0], got: {:?}",
            ir.nodes[0].kind
        );
    }

    // Проверяем MemberAccess
    if let SemanticNodeKind::MemberAccess {
        object_node,
        object_name,
        member_name,
        ..
    } = &ir.nodes[1].kind
    {
        assert_eq!(*object_node, Some(0)); // Ссылка на GlobalPropertyAccess
        assert!(object_name.is_none());
        assert_eq!(member_name, "Контрагенты");
    } else {
        panic!(
            "Expected MemberAccess at nodes[1], got: {:?}",
            ir.nodes[1].kind
        );
    }

    // Проверяем Assignment
    if let SemanticNodeKind::Assignment {
        variable,
        value_node,
        ..
    } = &ir.nodes[2].kind
    {
        assert_eq!(variable, "Менеджер");
        assert_eq!(*value_node, Some(1)); // Ссылка на MemberAccess
    } else {
        panic!(
            "Expected Assignment at nodes[2], got: {:?}",
            ir.nodes[2].kind
        );
    }
}

#[test]
fn test_global_property_access_for_documents() {
    // Тест с английским именем: Documents.Order
    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "DocManager".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::PropertyAccess {
                object: Box::new(Expression::Identifier {
                    name: "Documents".to_string(),
                    span: AstSpan::stub(),
                }),
                property: "Order".to_string(),
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "DocManager = Documents.Order;".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Проверяем GlobalPropertyAccess
    if let SemanticNodeKind::GlobalPropertyAccess { name } = &ir.nodes[0].kind {
        assert_eq!(name, "Documents");
    } else {
        panic!("Expected GlobalPropertyAccess at nodes[0]");
    }

    // Проверяем MemberAccess
    if let SemanticNodeKind::MemberAccess {
        object_node,
        object_name,
        member_name,
        ..
    } = &ir.nodes[1].kind
    {
        assert_eq!(*object_node, Some(0));
        assert!(object_name.is_none());
        assert_eq!(member_name, "Order");
    } else {
        panic!("Expected MemberAccess at nodes[1]");
    }
}

#[test]
fn test_regular_property_access_not_global() {
    // Тест: обычный PropertyAccess (не глобальная коллекция) НЕ создаёт GlobalPropertyAccess
    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Результат".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::PropertyAccess {
                object: Box::new(Expression::Identifier {
                    name: "МояПеременная".to_string(),
                    span: AstSpan::stub(),
                }),
                property: "Свойство".to_string(),
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Результат = МояПеременная.Свойство;".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    // Должно быть 2 узла:
    // 0: MemberAccess (без GlobalPropertyAccess)
    // 1: Assignment
    assert_eq!(ir.nodes.len(), 2);

    // Первый узел должен быть MemberAccess (не GlobalPropertyAccess)
    if let SemanticNodeKind::MemberAccess {
        object_node,
        object_name,
        member_name,
        ..
    } = &ir.nodes[0].kind
    {
        assert_eq!(*object_node, None); // Нет вложенного узла
        assert_eq!(object_name.as_ref().unwrap(), "МояПеременная");
        assert_eq!(member_name, "Свойство");
    } else {
        panic!(
            "Expected MemberAccess at nodes[0], got: {:?}",
            ir.nodes[0].kind
        );
    }
}

#[test]
fn test_global_property_access_for_accounting_registers() {
    // Тест: РегистрыБухгалтерии.Хозрасчетный создаёт GlobalPropertyAccess + MemberAccess
    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "РегМенеджер".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::PropertyAccess {
                object: Box::new(Expression::Identifier {
                    name: "РегистрыБухгалтерии".to_string(),
                    span: AstSpan::stub(),
                }),
                property: "Хозрасчетный".to_string(),
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "РегМенеджер = РегистрыБухгалтерии.Хозрасчетный;".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    assert_eq!(ir.nodes.len(), 3);

    // Проверяем GlobalPropertyAccess
    if let SemanticNodeKind::GlobalPropertyAccess { name } = &ir.nodes[0].kind {
        assert_eq!(name, "РегистрыБухгалтерии");
    } else {
        panic!(
            "Expected GlobalPropertyAccess at nodes[0], got: {:?}",
            ir.nodes[0].kind
        );
    }

    // Проверяем MemberAccess
    if let SemanticNodeKind::MemberAccess {
        object_node,
        object_name,
        member_name,
        ..
    } = &ir.nodes[1].kind
    {
        assert_eq!(*object_node, Some(0));
        assert!(object_name.is_none());
        assert_eq!(member_name, "Хозрасчетный");
    } else {
        panic!(
            "Expected MemberAccess at nodes[1], got: {:?}",
            ir.nodes[1].kind
        );
    }
}

#[test]
fn test_global_property_access_for_calculation_registers() {
    // Тест: РегистрыРасчета.ОсновныеНачисления создаёт GlobalPropertyAccess + MemberAccess
    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "РегМенеджер".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::PropertyAccess {
                object: Box::new(Expression::Identifier {
                    name: "РегистрыРасчета".to_string(),
                    span: AstSpan::stub(),
                }),
                property: "ОсновныеНачисления".to_string(),
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "РегМенеджер = РегистрыРасчета.ОсновныеНачисления;".to_string(),
        "test.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    assert_eq!(ir.nodes.len(), 3);

    // Проверяем GlobalPropertyAccess
    if let SemanticNodeKind::GlobalPropertyAccess { name } = &ir.nodes[0].kind {
        assert_eq!(name, "РегистрыРасчета");
    } else {
        panic!(
            "Expected GlobalPropertyAccess at nodes[0], got: {:?}",
            ir.nodes[0].kind
        );
    }

    // Проверяем MemberAccess
    if let SemanticNodeKind::MemberAccess {
        object_node,
        object_name,
        member_name,
        ..
    } = &ir.nodes[1].kind
    {
        assert_eq!(*object_node, Some(0));
        assert!(object_name.is_none());
        assert_eq!(member_name, "ОсновныеНачисления");
    } else {
        panic!(
            "Expected MemberAccess at nodes[1], got: {:?}",
            ir.nodes[1].kind
        );
    }
}

#[test]
fn test_form_module_implicit_symbols_are_injected_into_procedure_scope() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![],
            compiler_directive: Some(CompilerDirective::OnServer),
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\nКонецПроцедуры".to_string(),
        "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let body_scope = ir
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => Some(*body_scope),
            _ => None,
        })
        .expect("procedure scope");

    for name in [
        "ЭтотОбъект",
        "ЭтаФорма",
        "Форма",
        "Объект",
        "Элементы",
        "Параметры",
    ] {
        assert!(
            ir.symbols.has_variable(body_scope, name),
            "expected implicit symbol '{}' in form procedure scope",
            name
        );
    }
}

#[test]
fn test_form_module_no_context_does_not_inject_context_symbols() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![],
            compiler_directive: Some(CompilerDirective::OnServerNoContext),
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "&НаСервереБезКонтекста\nПроцедура Тест()\nКонецПроцедуры".to_string(),
        "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let body_scope = ir
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => Some(*body_scope),
            _ => None,
        })
        .expect("procedure scope");

    for name in [
        "ЭтотОбъект",
        "ЭтаФорма",
        "Форма",
        "Объект",
        "Элементы",
        "Параметры",
    ] {
        assert!(
            ir.symbols
                .lookup_variable_in_hierarchy(body_scope, name)
                .is_none(),
            "expected no implicit symbol '{}' for *БезКонтекста procedure",
            name
        );
    }
}

#[test]
fn test_nested_local_function_inherits_no_context_from_outer_procedure() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Внешняя".to_string(),
            params: vec![],
            body: vec![Statement::FunctionDecl {
                name: "Внутренняя".to_string(),
                params: vec![],
                body: vec![],
                compiler_directive: None,
                is_export: false,
                span: AstSpan::stub(),
            }],
            compiler_directive: Some(CompilerDirective::OnServerNoContext),
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "&НаСервереБезКонтекста\nПроцедура Внешняя()\n    Функция Внутренняя()\n    КонецФункции\nКонецПроцедуры".to_string(),
        "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let nested_body_scope = ir
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            SemanticNodeKind::FunctionDeclaration {
                name, body_scope, ..
            } if name == "Внутренняя" => Some(*body_scope),
            _ => None,
        })
        .expect("nested function scope");

    for name in [
        "ЭтотОбъект",
        "ЭтаФорма",
        "Форма",
        "Объект",
        "Элементы",
        "Параметры",
    ] {
        assert!(
            ir.symbols
                .lookup_variable_in_hierarchy(nested_body_scope, name)
                .is_none(),
            "expected no implicit symbol '{}' in nested function under *БезКонтекста",
            name
        );
    }
}

#[test]
fn test_manager_module_has_implicit_this_object_and_object() {
    let ast = Program {
        statements: vec![Statement::ProcedureDecl {
            name: "Тест".to_string(),
            params: vec![],
            body: vec![],
            compiler_directive: None,
            is_export: false,
            span: AstSpan::stub(),
        }],
    };

    let ir = AstToIrConverter::convert(
        ast,
        "Процедура Тест()\nКонецПроцедуры".to_string(),
        "Documents/Док1/Ext/ManagerModule.bsl".to_string(),
        create_test_repository(),
        create_test_signature_index(),
    )
    .unwrap();

    let body_scope = ir
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => Some(*body_scope),
            _ => None,
        })
        .expect("procedure scope");

    for name in ["ЭтотОбъект", "Объект"] {
        assert!(
            ir.symbols
                .lookup_variable_in_hierarchy(body_scope, name)
                .is_some(),
            "expected implicit symbol '{}' in manager module",
            name
        );
    }
}
