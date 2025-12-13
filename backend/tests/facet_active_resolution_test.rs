//! Integration тесты для Milestone 3.17: Active Facet Resolution
//!
//! Проверяем, что после вызова методов менеджера TypeResolution содержит корректный active_facet:
//! - НайтиПоКоду() → active_facet = Reference
//! - СоздатьЭлемент() / СоздатьДокумент() → active_facet = Object
//! - Выбрать() → active_facet = Selection
//!
//! Эти тесты критичны для корректной валидации методов фасетных типов в LSP.

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::parsing::bsl::ast::{Expression, Program, Span as AstSpan, Statement};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{FacetKind, TypeResolution};
use bsl_shared::domain::TypeResolver;
use bsl_shared::ir::ScopeId;
use std::sync::Arc;

/// Helper: создать TypeRepository с типами
fn create_test_repository() -> Arc<dyn TypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Helper: создать SignatureIndex с методами Справочник/Документ
fn create_signature_index_with_facet_methods() -> SignatureIndex {
    let mut sig_idx = SignatureIndex::new();

    // СправочникМенеджер.СоздатьЭлемент() → СправочникОбъект (Object facet)
    sig_idx.add_platform_method(
        TypeId::new("СправочникМенеджер"),
        MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникОбъект".to_string()),
            SignatureSource::Platform,
            Some(FacetKind::Object),
            ContextRequirements::ServerOnly,
        ),
    );

    // СправочникМенеджер.НайтиПоКоду() → СправочникСсылка (Reference facet)
    sig_idx.add_platform_method(
        TypeId::new("СправочникМенеджер"),
        MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникСсылка".to_string()),
            SignatureSource::Platform,
            Some(FacetKind::Reference),
            ContextRequirements::ServerOnly,
        ),
    );

    // СправочникМенеджер.Выбрать() → СправочникВыборка (Selection facet)
    sig_idx.add_platform_method(
        TypeId::new("СправочникМенеджер"),
        MethodSignature::new(
            "Выбрать".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникВыборка".to_string()),
            SignatureSource::Platform,
            Some(FacetKind::Selection),
            ContextRequirements::ServerOnly,
        ),
    );

    // ДокументМенеджер.СоздатьДокумент() → ДокументОбъект (Object facet)
    sig_idx.add_platform_method(
        TypeId::new("ДокументМенеджер"),
        MethodSignature::new(
            "СоздатьДокумент".to_string(),
            Some("ДокументМенеджер".to_string()),
            vec![],
            Some("ДокументОбъект".to_string()),
            SignatureSource::Platform,
            Some(FacetKind::Object),
            ContextRequirements::ServerOnly,
        ),
    );

    // ДокументМенеджер.НайтиПоНомеру() → ДокументСсылка (Reference facet)
    sig_idx.add_platform_method(
        TypeId::new("ДокументМенеджер"),
        MethodSignature::new(
            "НайтиПоНомеру".to_string(),
            Some("ДокументМенеджер".to_string()),
            vec![],
            Some("ДокументСсылка".to_string()),
            SignatureSource::Platform,
            Some(FacetKind::Reference),
            ContextRequirements::ServerOnly,
        ),
    );

    sig_idx
}

/// Helper: конвертировать AST в IR и вернуть TypeResolution переменной
fn convert_and_get_resolution(
    ast: Program,
    source: &str,
    var_name: &str,
    signature_index: SignatureIndex,
) -> Option<TypeResolution> {
    let repository = create_test_repository();

    // Создаём TypeResolver для корректной резолюции фасетов
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    let ir = AstToIrConverter::convert_with_resolver(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
        Some(resolver),
    )
    .ok()?;

    // Получаем TypeResolution из SymbolTable
    ir.symbols.get_variable_type(ScopeId(0), var_name)
}

// =============================================================================
// TEST 1: НайтиПоКоду() → active_facet = Reference
// =============================================================================

#[test]
fn test_find_by_code_returns_reference_facet() {
    let source = r#"
Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
"#;

    // Строим AST вручную
    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Ссылка".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Справочники".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Контрагенты".to_string(),
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
        }],
    };

    let sig_idx = create_signature_index_with_facet_methods();
    let resolution = convert_and_get_resolution(ast, source, "Ссылка", sig_idx);

    assert!(
        resolution.is_some(),
        "Variable Ссылка should be in symbol table"
    );

    let resolution = resolution.unwrap();
    println!("Ссылка resolution: {:?}", resolution);

    // Проверяем тип
    assert_eq!(
        resolution.type_name(),
        "СправочникСсылка.Контрагенты",
        "Type should be СправочникСсылка.Контрагенты"
    );

    // Проверяем active_facet
    assert_eq!(
        resolution.active_facet,
        Some(FacetKind::Reference),
        "НайтиПоКоду() должен возвращать Reference facet"
    );
}

// =============================================================================
// TEST 2: СоздатьЭлемент() → active_facet = Object
// =============================================================================

#[test]
fn test_create_element_returns_object_facet() {
    let source = r#"
Объект = Справочники.Номенклатура.СоздатьЭлемент();
"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Объект".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Справочники".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Номенклатура".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "СоздатьЭлемент".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let sig_idx = create_signature_index_with_facet_methods();
    let resolution = convert_and_get_resolution(ast, source, "Объект", sig_idx);

    assert!(
        resolution.is_some(),
        "Variable Объект should be in symbol table"
    );

    let resolution = resolution.unwrap();
    println!("Объект resolution: {:?}", resolution);

    // Проверяем тип
    assert_eq!(
        resolution.type_name(),
        "СправочникОбъект.Номенклатура",
        "Type should be СправочникОбъект.Номенклатура"
    );

    // Проверяем active_facet
    assert_eq!(
        resolution.active_facet,
        Some(FacetKind::Object),
        "СоздатьЭлемент() должен возвращать Object facet"
    );
}

// =============================================================================
// TEST 3: Выбрать() → active_facet = Selection
// =============================================================================

#[test]
fn test_select_returns_selection_facet() {
    let source = r#"
Выборка = Справочники.Контрагенты.Выбрать();
"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Выборка".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Справочники".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Контрагенты".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "Выбрать".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let sig_idx = create_signature_index_with_facet_methods();
    let resolution = convert_and_get_resolution(ast, source, "Выборка", sig_idx);

    assert!(
        resolution.is_some(),
        "Variable Выборка should be in symbol table"
    );

    let resolution = resolution.unwrap();
    println!("Выборка resolution: {:?}", resolution);

    // Проверяем тип
    assert_eq!(
        resolution.type_name(),
        "СправочникВыборка.Контрагенты",
        "Type should be СправочникВыборка.Контрагенты"
    );

    // Проверяем active_facet
    assert_eq!(
        resolution.active_facet,
        Some(FacetKind::Selection),
        "Выбрать() должен возвращать Selection facet"
    );
}

// =============================================================================
// TEST 4: Документ.СоздатьДокумент() → active_facet = Object
// =============================================================================

#[test]
fn test_document_create_returns_object_facet() {
    let source = r#"
ДокОбъект = Документы.ЗаказКлиента.СоздатьДокумент();
"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "ДокОбъект".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Документы".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "ЗаказКлиента".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "СоздатьДокумент".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let sig_idx = create_signature_index_with_facet_methods();
    let resolution = convert_and_get_resolution(ast, source, "ДокОбъект", sig_idx);

    assert!(
        resolution.is_some(),
        "Variable ДокОбъект should be in symbol table"
    );

    let resolution = resolution.unwrap();
    println!("ДокОбъект resolution: {:?}", resolution);

    // Проверяем тип
    assert_eq!(
        resolution.type_name(),
        "ДокументОбъект.ЗаказКлиента",
        "Type should be ДокументОбъект.ЗаказКлиента"
    );

    // Проверяем active_facet
    assert_eq!(
        resolution.active_facet,
        Some(FacetKind::Object),
        "СоздатьДокумент() должен возвращать Object facet"
    );
}

// =============================================================================
// TEST 5: Документ.НайтиПоНомеру() → active_facet = Reference
// =============================================================================

#[test]
fn test_document_find_by_number_returns_reference_facet() {
    let source = r#"
ДокСсылка = Документы.ЗаказКлиента.НайтиПоНомеру("00001");
"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "ДокСсылка".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Документы".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "ЗаказКлиента".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "НайтиПоНомеру".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![Expression::String {
                    value: "00001".to_string(),
                    span: AstSpan::stub(),
                }],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let sig_idx = create_signature_index_with_facet_methods();
    let resolution = convert_and_get_resolution(ast, source, "ДокСсылка", sig_idx);

    assert!(
        resolution.is_some(),
        "Variable ДокСсылка should be in symbol table"
    );

    let resolution = resolution.unwrap();
    println!("ДокСсылка resolution: {:?}", resolution);

    // Проверяем тип
    assert_eq!(
        resolution.type_name(),
        "ДокументСсылка.ЗаказКлиента",
        "Type should be ДокументСсылка.ЗаказКлиента"
    );

    // Проверяем active_facet
    assert_eq!(
        resolution.active_facet,
        Some(FacetKind::Reference),
        "НайтиПоНомеру() должен возвращать Reference facet"
    );
}

// =============================================================================
// TEST 6: Цепочка вызовов с разными фасетами
// =============================================================================

#[test]
fn test_chained_method_calls_different_facets() {
    let source = r#"
М = Справочники.Контрагенты;
Объект = М.СоздатьЭлемент();
Ссылка = М.НайтиПоКоду("001");
Выборка = М.Выбрать();
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
            // Ссылка = М.НайтиПоКоду("001")
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
            // Выборка = М.Выбрать()
            Statement::Assignment {
                target: Expression::Identifier {
                    name: "Выборка".to_string(),
                    span: AstSpan::stub(),
                },
                value: Expression::Call {
                    function: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "М".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Выбрать".to_string(),
                        span: AstSpan::stub(),
                    }),
                    args: vec![],
                    span: AstSpan::stub(),
                },
                span: AstSpan::stub(),
            },
        ],
    };

    let sig_idx = create_signature_index_with_facet_methods();
    let repository = create_test_repository();
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    let ir = AstToIrConverter::convert_with_resolver(
        ast,
        source.to_string(),
        "test.bsl".to_string(),
        repository,
        sig_idx,
        Some(resolver),
    )
    .unwrap();

    // Проверяем М — Manager facet
    let m_resolution = ir
        .symbols
        .lookup_variable_in_hierarchy(ScopeId(0), "М")
        .map(|(_, r)| r.clone());
    assert!(m_resolution.is_some(), "M should be in symbol table");
    let m_resolution = m_resolution.unwrap();
    assert_eq!(m_resolution.type_name(), "СправочникМенеджер.Контрагенты");
    assert_eq!(
        m_resolution.active_facet,
        Some(FacetKind::Manager),
        "М должен иметь Manager facet"
    );

    // Проверяем Объект — Object facet
    let obj_resolution = ir
        .symbols
        .lookup_variable_in_hierarchy(ScopeId(0), "Объект")
        .map(|(_, r)| r.clone());
    assert!(obj_resolution.is_some(), "Объект should be in symbol table");
    let obj_resolution = obj_resolution.unwrap();
    assert_eq!(
        obj_resolution.type_name(),
        "СправочникОбъект.Контрагенты"
    );
    assert_eq!(
        obj_resolution.active_facet,
        Some(FacetKind::Object),
        "Объект должен иметь Object facet"
    );

    // Проверяем Ссылка — Reference facet
    let ref_resolution = ir
        .symbols
        .lookup_variable_in_hierarchy(ScopeId(0), "Ссылка")
        .map(|(_, r)| r.clone());
    assert!(ref_resolution.is_some(), "Ссылка should be in symbol table");
    let ref_resolution = ref_resolution.unwrap();
    assert_eq!(
        ref_resolution.type_name(),
        "СправочникСсылка.Контрагенты"
    );
    assert_eq!(
        ref_resolution.active_facet,
        Some(FacetKind::Reference),
        "Ссылка должен иметь Reference facet"
    );

    // Проверяем Выборка — Selection facet
    let sel_resolution = ir
        .symbols
        .lookup_variable_in_hierarchy(ScopeId(0), "Выборка")
        .map(|(_, r)| r.clone());
    assert!(sel_resolution.is_some(), "Выборка should be in symbol table");
    let sel_resolution = sel_resolution.unwrap();
    assert_eq!(
        sel_resolution.type_name(),
        "СправочникВыборка.Контрагенты"
    );
    assert_eq!(
        sel_resolution.active_facet,
        Some(FacetKind::Selection),
        "Выборка должен иметь Selection facet"
    );
}

// =============================================================================
// TEST 7: Edge case — метод НЕ найден (активный фасет не устанавливается)
// =============================================================================

#[test]
fn test_unknown_method_no_active_facet() {
    let source = r#"
Результат = Справочники.Контрагенты.НеСуществующийМетод();
"#;

    let ast = Program {
        statements: vec![Statement::Assignment {
            target: Expression::Identifier {
                name: "Результат".to_string(),
                span: AstSpan::stub(),
            },
            value: Expression::Call {
                function: Box::new(Expression::PropertyAccess {
                    object: Box::new(Expression::PropertyAccess {
                        object: Box::new(Expression::Identifier {
                            name: "Справочники".to_string(),
                            span: AstSpan::stub(),
                        }),
                        property: "Контрагенты".to_string(),
                        span: AstSpan::stub(),
                    }),
                    property: "НеСуществующийМетод".to_string(),
                    span: AstSpan::stub(),
                }),
                args: vec![],
                span: AstSpan::stub(),
            },
            span: AstSpan::stub(),
        }],
    };

    let sig_idx = create_signature_index_with_facet_methods();
    let resolution = convert_and_get_resolution(ast, source, "Результат", sig_idx);

    assert!(
        resolution.is_some(),
        "Variable Результат should be in symbol table"
    );

    let resolution = resolution.unwrap();
    println!("Результат resolution: {:?}", resolution);

    // Метод не найден → тип Dynamic, без active_facet
    assert_eq!(
        resolution.type_name(),
        "Dynamic",
        "Unknown method should return Dynamic"
    );

    assert_eq!(
        resolution.active_facet, None,
        "Unknown method should not set active_facet"
    );
}
