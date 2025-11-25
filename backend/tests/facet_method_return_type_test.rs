//! Unit тесты для Milestone 3.11 Phase 2 - Facet Switching при вызове методов
//!
//! Проверяют:
//! 1. find_method находит методы по базовому фасетному типу
//! 2. Return type с подстановкой имени объекта: СправочникОбъект → СправочникОбъект.Контрагенты

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::types::FacetKind;
use bsl_shared::ir::{ScopeId, TypeHint};
use std::sync::Arc;

/// Helper функция - создаёт TypeRepository
fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Helper функция - создаёт SignatureIndex с методами СправочникМенеджер
fn create_catalog_signature_index() -> SignatureIndex {
    let mut sig_idx = SignatureIndex::new();

    // СправочникМенеджер.СоздатьЭлемент() → СправочникОбъект
    // Методы хранятся под БАЗОВЫМ типом "СправочникМенеджер"
    sig_idx.add_platform_method(
        "СправочникМенеджер".to_string(),
        MethodSignature {
            name: "СоздатьЭлемент".to_string(),
            owner_type: Some("СправочникМенеджер".to_string()),
            params: vec![],
            return_type: Some("СправочникОбъект".to_string()), // Базовый фасетный тип
            source: SignatureSource::Platform,
            return_facet: Some(FacetKind::Object),
            context_requirements: ContextRequirements::ServerOnly,
        },
    );

    // СправочникМенеджер.НайтиПоКоду() → СправочникСсылка
    sig_idx.add_platform_method(
        "СправочникМенеджер".to_string(),
        MethodSignature {
            name: "НайтиПоКоду".to_string(),
            owner_type: Some("СправочникМенеджер".to_string()),
            params: vec![],
            return_type: Some("СправочникСсылка".to_string()),
            source: SignatureSource::Platform,
            return_facet: Some(FacetKind::Reference),
            context_requirements: ContextRequirements::ServerOnly,
        },
    );

    // СправочникМенеджер.Выбрать() → СправочникВыборка
    sig_idx.add_platform_method(
        "СправочникМенеджер".to_string(),
        MethodSignature {
            name: "Выбрать".to_string(),
            owner_type: Some("СправочникМенеджер".to_string()),
            params: vec![],
            return_type: Some("СправочникВыборка".to_string()),
            source: SignatureSource::Platform,
            return_facet: Some(FacetKind::Selection),
            context_requirements: ContextRequirements::ServerOnly,
        },
    );

    sig_idx
}

#[test]
fn test_find_method_on_concrete_faceted_type() {
    // Тест: find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент")
    // должен найти метод под ключом "СправочникМенеджер" (базовый тип)
    let sig_idx = create_catalog_signature_index();

    // Поиск по конкретному типу
    let method = sig_idx.find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент");

    assert!(
        method.is_some(),
        "Method СоздатьЭлемент should be found for СправочникМенеджер.Контрагенты"
    );

    let method = method.unwrap();
    assert_eq!(method.name, "СоздатьЭлемент");
    assert_eq!(method.return_type, Some("СправочникОбъект".to_string()));
}

#[test]
fn test_find_method_on_base_type() {
    // Тест: find_method("СправочникМенеджер", "СоздатьЭлемент")
    let sig_idx = create_catalog_signature_index();

    let method = sig_idx.find_method("СправочникМенеджер", "СоздатьЭлемент");

    assert!(
        method.is_some(),
        "Method СоздатьЭлемент should be found for base СправочникМенеджер"
    );
}

#[test]
fn test_substitute_type_name() {
    // Тест: substitute_type_name("СправочникОбъект", "Контрагенты") → "СправочникОбъект.Контрагенты"
    let result = SignatureIndex::substitute_type_name("СправочникОбъект", "Контрагенты");
    assert_eq!(result, "СправочникОбъект.Контрагенты");

    // Тест для Selection
    let result = SignatureIndex::substitute_type_name("СправочникВыборка", "Номенклатура");
    assert_eq!(result, "СправочникВыборка.Номенклатура");

    // Не-фасетный тип не подставляется
    let result = SignatureIndex::substitute_type_name("Число", "Контрагенты");
    assert_eq!(result, "Число");
}

#[test]
fn test_extract_metadata_name() {
    // Тест: extract_metadata_name("СправочникМенеджер.Контрагенты") → Some("Контрагенты")
    let name = SignatureIndex::extract_metadata_name("СправочникМенеджер.Контрагенты");
    assert_eq!(name, Some("Контрагенты"));

    // Не-фасетный тип
    let name = SignatureIndex::extract_metadata_name("Массив");
    assert_eq!(name, None);
}

#[test]
fn test_facet_method_call_return_type_inference() {
    // Полный интеграционный тест:
    // М = Справочники.Контрагенты;  // → СправочникМенеджер.Контрагенты
    // Объект = М.СоздатьЭлемент();  // → СправочникОбъект.Контрагенты

    let source = r#"
М = Справочники.Контрагенты;
Объект = М.СоздатьЭлемент();
"#;

    let ast = Program {
        statements: vec![
            // М = Справочники.Контрагенты
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "М".to_string(),
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
            },
            // Объект = М.СоздатьЭлемент()
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "Объект".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "М".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "СоздатьЭлемент".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![],
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
        create_catalog_signature_index(),
    )
    .unwrap();

    // Проверяем типы переменных
    println!("=== Symbol Table Variables ===");
    for (name, info) in &ir.symbols.scopes[&ScopeId(0)].variables {
        println!("  {} : {:?}", name, info);
    }

    // Проверяем тип М
    let m_type = ir.symbols.lookup_variable_in_hierarchy(ScopeId(0), "М");
    println!("\nМ type: {:?}", m_type);
    assert!(m_type.is_some(), "Variable М should be in symbol table");
    let (_, m_hint) = m_type.unwrap();
    match m_hint {
        TypeHint::Explicit(t) | TypeHint::Inferred(t) => {
            assert_eq!(
                t, "СправочникМенеджер.Контрагенты",
                "М should be СправочникМенеджер.Контрагенты"
            );
        }
        _ => panic!("Unexpected type hint for М"),
    }

    // Проверяем тип Объект
    let obj_type = ir.symbols.lookup_variable_in_hierarchy(ScopeId(0), "Объект");
    println!("Объект type: {:?}", obj_type);
    assert!(obj_type.is_some(), "Variable Объект should be in symbol table");
    let (_, obj_hint) = obj_type.unwrap();
    match obj_hint {
        TypeHint::Explicit(t) | TypeHint::Inferred(t) => {
            assert_eq!(
                t, "СправочникОбъект.Контрагенты",
                "Объект should be СправочникОбъект.Контрагенты (facet switched with name substitution)"
            );
        }
        _ => panic!("Unexpected type hint for Объект"),
    }
}

#[test]
fn test_facet_method_call_reference_return() {
    // М = Справочники.Номенклатура;
    // Ссылка = М.НайтиПоКоду("001");  // → СправочникСсылка.Номенклатура

    let source = r#"
М = Справочники.Номенклатура;
Ссылка = М.НайтиПоКоду("001");
"#;

    let ast = Program {
        statements: vec![
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "М".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::PropertyAccess {
                    object: Box::new(Expression::Identifier {
                        name: "Справочники".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Номенклатура".to_string(),
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "Ссылка".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "М".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "НайтиПоКоду".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![Expression::String {
                        value: "001".to_string(),
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
        create_catalog_signature_index(),
    )
    .unwrap();

    let ref_type = ir.symbols.lookup_variable_in_hierarchy(ScopeId(0), "Ссылка");
    assert!(ref_type.is_some(), "Variable Ссылка should be in symbol table");
    let (_, ref_hint) = ref_type.unwrap();
    match ref_hint {
        TypeHint::Explicit(t) | TypeHint::Inferred(t) => {
            assert_eq!(
                t, "СправочникСсылка.Номенклатура",
                "Ссылка should be СправочникСсылка.Номенклатура"
            );
        }
        _ => panic!("Unexpected type hint for Ссылка"),
    }
}
