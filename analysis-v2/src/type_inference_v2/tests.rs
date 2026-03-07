use super::*;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{
    FacetKind, MetadataKind, ParameterInfo, PrimitiveType, RawAttributeData, RawDataSource,
    RawPropertyData, RawTabularSectionData, RawTypeData, FORM_DATA_SEMANTICS_NOTE,
};
use bsl_shared::TypeRepository;
use bsl_syntax::ParseOptions;

fn parse(code: &str) -> Program {
    let parsed = bsl_syntax::parse(code, &ParseOptions::default()).expect("parse ok");
    parsed.program
}

fn deps_with_array_method() -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Массив".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        }])
        .expect("load types");

    let mut sigs = SignatureIndex::new();
    sigs.add_platform_method(
        TypeId::new("Массив"),
        MethodSignature::new(
            "Количество".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Число".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            Default::default(),
        ),
    );
    repository_impl.set_signature_index(sigs.clone());

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(SemanticDeps {
        repository,
        signature_index: sigs,
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    })
}

fn deps_with_common_module_method() -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    let mut sigs = SignatureIndex::new();
    sigs.add_config_method(
        TypeId::new("ОбщиеМодули.ОбщийМодуль1"),
        MethodSignature::new(
            "Ф1".to_string(),
            Some("ОбщиеМодули.ОбщийМодуль1".to_string()),
            vec![],
            Some("Число".to_string()),
            None,
            None,
            SignatureSource::Configuration,
            None,
            Default::default(),
        ),
    );
    repository_impl.set_signature_index(sigs.clone());

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(SemanticDeps {
        repository,
        signature_index: sigs,
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    })
}

fn deps_with_universal_collection_types() -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Соответствие".to_string(),
                source: RawDataSource::Platform,
                methods: vec![],
                ..Default::default()
            },
            RawTypeData {
                name: "Структура".to_string(),
                source: RawDataSource::Platform,
                methods: vec![],
                ..Default::default()
            },
            RawTypeData {
                name: "ТаблицаЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Колонки".to_string(),
                    prop_type: "КоллекцияКолонокТаблицыЗначений".to_string(),
                    is_readonly: false,
                }],
                methods: vec![],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияКолонокТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                methods: vec![],
                ..Default::default()
            },
            RawTypeData {
                name: "СтрокаТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ОписаниеТипов".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load universal collection types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    })
}

fn deps_with_document_create_document_method() -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Документы.РеализацияТоваровУслуг".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            ..Default::default()
        }])
        .expect("load config document type");

    let mut sigs = SignatureIndex::new();
    sigs.add_platform_method(
        TypeId::new("ДокументМенеджер"),
        MethodSignature::new(
            "СоздатьДокумент".to_string(),
            Some("ДокументМенеджер".to_string()),
            vec![],
            Some("ДокументОбъект.<Имя документа>".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            Default::default(),
        ),
    );
    repository_impl.set_signature_index(sigs.clone());

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(SemanticDeps {
        repository,
        signature_index: sigs,
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    })
}

fn deps_with_form_attribute_to_value_signature() -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументОбъект".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Object],
                properties: vec![RawPropertyData {
                    name: "Ссылка".to_string(),
                    prop_type: "ДокументСсылка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументСсылка".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Reference],
                ..Default::default()
            },
        ])
        .expect("load types");

    let mut sigs = SignatureIndex::new();
    sigs.add_global_function(
        TypeId::new("FormAttributeToValue"),
        MethodSignature::new(
            "FormAttributeToValue".to_string(),
            None,
            vec![ParameterInfo {
                name: "ИмяРеквизита".to_string(),
                type_name: Some("Строка".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            Some("ДокументОбъект.Док1".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            Default::default(),
        ),
    );
    repository_impl.set_signature_index(sigs.clone());

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(SemanticDeps {
        repository,
        signature_index: sigs,
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    })
}

#[test]
fn builds_type_index_for_simple_assignment_and_method_call() {
    let source = r#"Перем М;
М = Новый Массив();
Р = М.Количество();
"#;
    let program = parse(source);
    let deps = deps_with_array_method();
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let array_ident_offset = source
        .find("\nМ =")
        .map(|idx| idx + 1)
        .expect("assignment line start") as u32;
    let array_ident = index
        .type_at_byte_offset(array_ident_offset)
        .expect("type at assignment");
    assert_eq!(array_ident.type_name(), "Массив<Неопределено>");

    let method_call_offset = source.find("Количество").expect("method name") as u32;
    let method_call = index
        .type_at_byte_offset(method_call_offset)
        .expect("type at method call");
    assert_eq!(method_call.type_name(), "Число");
}

#[test]
fn resolves_common_module_method_return_type_from_signature_index() {
    let source = r#"Процедура Тест()
    x = ОбщийМодуль1.Ф1();
КонецПроцедуры
"#;
    let program = parse(source);
    let deps = deps_with_common_module_method();
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source.find("Ф1").expect("method name") as u32;
    let result = index
        .type_at_byte_offset(offset)
        .expect("type at method call");
    assert_eq!(result.type_name(), "Число");
}

#[test]
fn resolves_local_function_return_type_defined_later_in_common_module_file() {
    let source = r#"Процедура Тест()
    x = ФункцияКотораяВозвращаетСтроку();
КонецПроцедуры

Функция ФункцияКотораяВозвращаетСтроку()
    Возврат "ТестоваяСтрока";
КонецФункции
"#;
    let program = parse(source);
    let deps = deps_with_array_method();
    let file_path = "CommonModules/АвансовыйОтчетФормы/Ext/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let offset = source
        .find("ФункцияКотораяВозвращаетСтроку")
        .expect("function name") as u32;
    let result = index
        .type_at_byte_offset(offset)
        .expect("type at function call");
    assert_eq!(result.type_name(), "Строка");
}

#[test]
fn substitutes_placeholder_return_type_for_document_method_call() {
    let source = r#"Процедура Тест()
    Док = Документы.РеализацияТоваровУслуг.СоздатьДокумент();
КонецПроцедуры
"#;
    let program = parse(source);
    let deps = deps_with_document_create_document_method();
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source
        .find("СоздатьДокумент()")
        .map(|idx| idx + "СоздатьДокумент".len())
        .expect("method call") as u32;
    let result = index
        .type_at_byte_offset(offset)
        .expect("type at method call");
    assert_eq!(
        result.type_name(),
        "ДокументОбъект.РеализацияТоваровУслуг",
        "Expected placeholder <Имя документа> to be substituted from receiver metadata name"
    );
    assert!(!result.is_unknown());
}

#[test]
fn infers_union_return_type_for_local_function() {
    let source = r#"Функция F(Флаг)
    Если Флаг Тогда
        Возврат 1;
    Иначе
        Возврат "x";
    КонецЕсли;
КонецФункции

Процедура Тест()
    x = F(Истина);
КонецПроцедуры
"#;
    let program = parse(source);
    let deps = deps_with_array_method();
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source.find("F(Истина)").expect("call") as u32;
    let result = index.type_at_byte_offset(offset).expect("type at call");
    assert_eq!(result.type_name(), "Строка | Число");
}

#[test]
fn propagates_union_return_type_through_local_function_call() {
    let source = r#"Функция B(Флаг)
    Если Флаг Тогда
        Возврат 1;
    Иначе
        Возврат "x";
    КонецЕсли;
КонецФункции

Функция A(Флаг)
    Возврат B(Флаг);
КонецФункции

Процедура Тест()
    x = A(Истина);
КонецПроцедуры
"#;
    let program = parse(source);
    let deps = deps_with_array_method();
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source.find("A(Истина)").expect("call") as u32;
    let result = index.type_at_byte_offset(offset).expect("type at call");

    match result.result {
        ResolutionResult::Union(variants) => {
            assert!(
                variants
                    .iter()
                    .any(|v| matches!(v.type_, ConcreteType::Primitive(PrimitiveType::String))),
                "expected String variant, got: {:?}",
                variants
            );
            assert!(
                variants
                    .iter()
                    .any(|v| matches!(v.type_, ConcreteType::Primitive(PrimitiveType::Number))),
                "expected Number variant, got: {:?}",
                variants
            );
        }
        other => panic!("expected Union, got: {:?}", other),
    }
}

#[test]
fn adds_undefined_when_function_can_fallthrough() {
    let source = r#"Функция F(Флаг)
    Если Флаг Тогда
        Возврат 1;
    КонецЕсли;
КонецФункции

Процедура Тест()
    x = F(Истина);
КонецПроцедуры
"#;
    let program = parse(source);
    let deps = deps_with_array_method();
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source.find("F(Истина)").expect("call") as u32;
    let result = index.type_at_byte_offset(offset).expect("type at call");
    assert_eq!(result.type_name(), "Неопределено | Число");
}

#[test]
fn mutual_recursion_is_deterministic_and_terminates() {
    let source = r#"Функция A()
    Возврат B();
КонецФункции

Функция B()
    Возврат A();
КонецФункции

Процедура Тест()
    x = A();
КонецПроцедуры
"#;
    let program = parse(source);
    let deps = deps_with_array_method();
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source.find("A();").expect("call") as u32;
    let result = index.type_at_byte_offset(offset).expect("type at call");
    assert!(
        result.is_unknown() && matches!(result.result, ResolutionResult::Dynamic),
        "expected Unknown/Dynamic, got: {:?}",
        result
    );
}

#[test]
fn seeds_form_module_context_for_elements_property_access() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
            RawTypeData {
                name: "ЭлементыФормы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "СчетФактураПросмотр".to_string(),
                    prop_type: "ГруппаФормы".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ГруппаФормы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = Элементы.СчетФактураПросмотр;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let loc = CodeLocation::determine_from_path(Path::new(file_path)).expect("code location");
    assert!(
        matches!(loc.module_type, ModuleType::FormModule { .. }),
        "expected FormModule for seed path, got {:?}",
        loc.module_type
    );
    assert!(
        repository_impl
            .find_type("Формы.Документы.Док1.Форма1")
            .is_some(),
        "expected synthetic form type to be present"
    );
    assert!(
        repository_impl
            .find_type("ЭлементыФормы.Документы.Док1.Форма1")
            .is_some(),
        "expected synthetic form elements type to be present"
    );

    let index = build_type_index_with_path(&program, file_path, deps);

    let receiver_offset = source.find("Элементы").expect("Элементы") as u32;
    let receiver = index
        .type_at_byte_offset(receiver_offset)
        .expect("type at Элементы");
    assert_eq!(
        receiver.type_name(),
        "ЭлементыФормы.Документы.Док1.Форма1",
        "receiver should be seeded from form module context"
    );

    let member_offset = source.find("СчетФактураПросмотр").expect("member") as u32;
    let member = index
        .type_at_byte_offset(member_offset)
        .expect("type at member access");
    assert_eq!(member.type_name(), "ГруппаФормы");
}

#[test]
fn seeds_form_module_context_for_this_object_and_parameters() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Параметры;
    z = Объект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
    let this_object = index
        .type_at_byte_offset(this_object_offset)
        .expect("type at ЭтотОбъект");
    assert_eq!(this_object.type_name(), "Формы.Документы.Док1.Форма1");

    let params_offset = source.find("Параметры").expect("Параметры") as u32;
    let params = index
        .type_at_byte_offset(params_offset)
        .expect("type at Параметры");
    assert_eq!(params.type_name(), "Структура");

    let object_offset = source
        .find("z = Объект")
        .map(|idx| idx + "z = ".len())
        .expect("Объект") as u32;
    let object = index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");
    assert_eq!(object.type_name(), "Документы.Док1");
}

#[test]
fn form_module_object_seed_contains_form_data_semantics_metadata_notes() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    z = Объект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let object_offset = source
        .find("z = Объект")
        .map(|idx| idx + "z = ".len())
        .expect("Объект") as u32;
    let object = index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");

    assert_eq!(object.type_name(), "Документы.Док1");
    assert_eq!(object.active_facet, None);
    assert!(
        object
            .metadata
            .notes
            .iter()
            .any(|note| note == FORM_DATA_SEMANTICS_NOTE),
        "missing form-data semantics note: {:?}",
        object.metadata.notes
    );
}

#[test]
fn form_module_object_descriptor_degrades_to_inferred_weak_without_metadata() {
    let deps = deps_with_array_method();
    let source = r#"Процедура Тест()
    z = Объект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let object_offset = source
        .find("z = Объект")
        .map(|idx| idx + "z = ".len())
        .expect("Объект") as u32;
    let object = index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");

    assert_eq!(object.type_name(), "Документы.Док1");
    assert_eq!(object.active_facet, None);
    assert_eq!(object.certainty, Certainty::InferredWeak);
}

#[test]
fn resolves_form_module_object_link_property_from_object_facet() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументОбъект".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Object],
                properties: vec![RawPropertyData {
                    name: "Ссылка".to_string(),
                    prop_type: "ДокументСсылка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументСсылка".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Reference],
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = Объект.Ссылка;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let link_offset = source.find("Ссылка").expect("Ссылка") as u32;
    let link_type = index
        .type_at_byte_offset(link_offset)
        .expect("type at Объект.Ссылка");
    assert_eq!(link_type.type_name(), "ДокументСсылка.Док1");
}

#[test]
fn resolves_form_module_object_link_property_without_platform_object_properties() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = Объект.Ссылка;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let link_offset = source.find("Ссылка").expect("Ссылка") as u32;
    let link_type = index
        .type_at_byte_offset(link_offset)
        .expect("type at Объект.Ссылка");
    assert_eq!(link_type.type_name(), "ДокументСсылка.Док1");
}

#[test]
fn resolves_form_module_object_deletion_mark_property_without_platform_object_properties() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = Объект.ПометкаУдаления;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let mark_offset = source.find("ПометкаУдаления").expect("ПометкаУдаления") as u32;
    let mark_type = index
        .type_at_byte_offset(mark_offset)
        .expect("type at Объект.ПометкаУдаления");
    assert_eq!(mark_type.type_name(), "Булево");
}

#[test]
fn resolves_catalog_form_module_object_link_property_without_platform_object_properties() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Справочники.Спр1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Catalog),
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Справочники.Спр1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = Объект.Ссылка;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Catalogs/Спр1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let link_offset = source.find("Ссылка").expect("Ссылка") as u32;
    let link_type = index
        .type_at_byte_offset(link_offset)
        .expect("type at Объект.Ссылка");
    assert_eq!(link_type.type_name(), "СправочникСсылка.Спр1");
}

#[test]
fn form_module_object_member_resolution_does_not_leak_form_shape() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументОбъект".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Object],
                properties: vec![RawPropertyData {
                    name: "Ссылка".to_string(),
                    prop_type: "ДокументСсылка".to_string(),
                    is_readonly: true,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ДокументСсылка".to_string(),
                source: RawDataSource::Platform,
                facets: vec![FacetKind::Reference],
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "СчетФактура".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    a = Объект.СчетФактура;
    b = Объект.Ссылка;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let attr_offset = source.find("СчетФактура").expect("СчетФактура") as u32;
    let attr_type = index
        .type_at_byte_offset(attr_offset)
        .expect("type at Объект.СчетФактура");
    assert!(
        attr_type.is_unknown(),
        "form-only реквизиты не должны резолвиться через Объект, got: {:?}",
        attr_type
    );

    let link_offset = source.rfind("Ссылка").expect("Ссылка") as u32;
    let link_type = index
        .type_at_byte_offset(link_offset)
        .expect("type at Объект.Ссылка");
    assert_eq!(link_type.type_name(), "ДокументСсылка.Док1");
}

#[test]
fn form_module_object_resolves_tabular_projection_without_form_shape_leakage() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                tabular_sections: vec![RawTabularSectionData {
                    name: "Товары".to_string(),
                    attributes: vec![RawAttributeData {
                        name: "Номенклатура".to_string(),
                        attr_type: "СправочникСсылка.Номенклатура".to_string(),
                    }],
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                properties: vec![RawPropertyData {
                    name: "СчетФактура".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
        ])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    a = Объект.Товары;
    b = Объект.СчетФактура;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let table_offset = source.find("Товары").expect("Товары") as u32;
    let table_type = index
        .type_at_byte_offset(table_offset)
        .expect("type at Объект.Товары");
    assert_eq!(table_type.type_name(), "ДанныеФормыКоллекция<СтрокаТовары>");

    let form_only_offset = source.find("СчетФактура").expect("СчетФактура") as u32;
    let form_only_type = index
        .type_at_byte_offset(form_only_offset)
        .expect("type at Объект.СчетФактура");
    assert!(
        form_only_type.is_unknown(),
        "form-only реквизиты не должны резолвиться через Объект, got: {:?}",
        form_only_type
    );
}

#[test]
fn form_attribute_to_value_object_keeps_object_members_available() {
    let deps = deps_with_form_attribute_to_value_signature();
    let source = r#"Процедура Тест()
    x = FormAttributeToValue("Объект").Ссылка;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let link_offset = source.rfind("Ссылка").expect("Ссылка") as u32;
    let link_type = index
        .type_at_byte_offset(link_offset)
        .expect("type at FormAttributeToValue(\"Объект\").Ссылка");
    assert_eq!(link_type.type_name(), "ДокументСсылка");
}

#[test]
fn seeds_manager_module_context_for_this_object_and_object() {
    let deps = deps_with_array_method();
    let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Ext/ManagerModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
    let this_object = index
        .type_at_byte_offset(this_object_offset)
        .expect("type at ЭтотОбъект");
    assert_eq!(this_object.type_name(), "ДокументМенеджер.Док1");

    let object_offset = source.find("Объект").expect("Объект") as u32;
    let object = index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");
    assert_eq!(object.type_name(), "ДокументМенеджер.Док1");
}

#[test]
fn seeds_object_module_context_for_this_object_and_object() {
    let deps = deps_with_array_method();
    let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Ext/ObjectModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
    let this_object = index
        .type_at_byte_offset(this_object_offset)
        .expect("type at ЭтотОбъект");
    assert_eq!(this_object.type_name(), "ДокументОбъект.Док1");

    let object_offset = source.find("Объект").expect("Объект") as u32;
    let object = index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");
    assert_eq!(object.type_name(), "ДокументОбъект.Док1");
}

#[test]
fn seeds_catalog_object_module_context_for_hierarchical_catalogs() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Справочники.Иерархический".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Catalog),
            ..Default::default()
        }])
        .expect("load types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Catalogs/Иерархический/Ext/ObjectModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
    let this_object = index
        .type_at_byte_offset(this_object_offset)
        .expect("type at ЭтотОбъект");
    assert_eq!(this_object.type_name(), "СправочникОбъект.Иерархический");

    let object_offset = source.find("Объект").expect("Объект") as u32;
    let object = index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");
    assert_eq!(object.type_name(), "СправочникОбъект.Иерархический");
}

#[test]
fn seeds_recordset_module_context_for_this_object_and_object() {
    let deps = deps_with_array_method();
    let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
    let this_object = index
        .type_at_byte_offset(this_object_offset)
        .expect("type at ЭтотОбъект");
    assert_eq!(
        this_object.type_name(),
        "РегистрСведенийНаборЗаписей.Регистр1"
    );

    let object_offset = source.find("Объект").expect("Объект") as u32;
    let object = index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");
    assert_eq!(object.type_name(), "РегистрСведенийНаборЗаписей.Регистр1");
}

#[test]
fn no_context_directive_hides_form_context_symbols() {
    let deps = deps_with_array_method();
    let source = r#"&НаСервереБезКонтекста
Процедура Тест()
    x = ЭтотОбъект;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
    let this_object = index
        .type_at_byte_offset(this_object_offset)
        .expect("type at ЭтотОбъект");
    assert_eq!(
        this_object.is_undeclared_variable(),
        Some("ЭтотОбъект"),
        "expected ЭтотОбъект to be undeclared in *БезКонтекста"
    );
}

#[test]
fn object_module_bare_identifier_resolves_owner_member_before_undeclared() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Документы.Док1".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            properties: vec![RawPropertyData {
                name: "ДоговорКонтрагента".to_string(),
                prop_type: "Строка".to_string(),
                is_readonly: false,
            }],
            ..Default::default()
        }])
        .expect("load types");
    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = ДоговорКонтрагента;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Ext/ObjectModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let offset = source.find("ДоговорКонтрагента").expect("identifier") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at owner member identifier");
    assert!(
        resolved.is_undeclared_variable().is_none(),
        "owner member must resolve before undeclared: {:?}",
        resolved
    );
    assert_eq!(resolved.type_name(), "Строка");
}

#[test]
fn recordset_module_bare_identifier_resolves_owner_member_before_undeclared() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "РегистрыСведений.Регистр1".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object],
            kind: Some(MetadataKind::InformationRegister),
            properties: vec![RawPropertyData {
                name: "ОбменДанными".to_string(),
                prop_type: "Булево".to_string(),
                is_readonly: false,
            }],
            ..Default::default()
        }])
        .expect("load types");
    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = ОбменДанными;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let offset = source.find("ОбменДанными").expect("identifier") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at owner member identifier");
    assert!(
        resolved.is_undeclared_variable().is_none(),
        "recordset owner member must resolve before undeclared: {:?}",
        resolved
    );
    assert_eq!(resolved.type_name(), "Булево");
}

#[test]
fn form_module_bare_identifier_does_not_use_applied_owner_fallback() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Документы.Док1".to_string(),
                source: RawDataSource::Configuration,
                facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
                kind: Some(MetadataKind::Document),
                properties: vec![RawPropertyData {
                    name: "ДоговорКонтрагента".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "Формы.Документы.Док1.Форма1".to_string(),
                source: RawDataSource::Configuration,
                ..Default::default()
            },
        ])
        .expect("load types");
    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let source = r#"Процедура Тест()
    x = ДоговорКонтрагента;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let offset = source.find("ДоговорКонтрагента").expect("identifier") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at bare identifier");
    assert_eq!(
        resolved.is_undeclared_variable(),
        Some("ДоговорКонтрагента"),
        "FormModule must stay strict and not use applied owner fallback"
    );
}

#[test]
fn local_parameter_shadows_global_collection_in_identifier_resolution() {
    let deps = deps_with_array_method();
    let source = r#"Процедура Тест(Справочники)
    x = Справочники;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Ext/ObjectModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let offset = source
        .find("x = Справочники")
        .map(|idx| idx + "x = ".len())
        .expect("identifier offset") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at identifier");

    assert!(
        resolved.is_unknown(),
        "procedure parameter must shadow global collection and stay unknown, got: {:?}",
        resolved
    );
}

#[test]
fn map_index_access_materializes_inserted_value_type() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    map = Новый Соответствие;
    map.Вставить("k", Новый ТаблицаЗначений);
    probe = map["k"];
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let offset = source
        .find("map[\"k\"]")
        .map(|idx| idx + "map[\"k\"]".len() - 1)
        .expect("map index") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at map index");

    assert_eq!(resolved.type_name(), "ТаблицаЗначений");
}

#[test]
fn universal_collection_effects_do_not_mutate_type_repository() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "Соответствие".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Структура".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ТаблицаЗначений".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Колонки".to_string(),
                    prop_type: "КоллекцияКолонокТаблицыЗначений".to_string(),
                    is_readonly: false,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияКолонокТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "СтрокаТаблицыЗначений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ОписаниеТипов".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load universal collection types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(Arc::new(TypeResolver::new(
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>
        ))),
        platform_signatures_loaded: true,
    });
    let before = repository_impl.get_all_types();

    let source = r#"Процедура Тест()
    map = Новый Соответствие;
    map.Вставить("k", Новый ТаблицаЗначений);
    S = Новый Структура;
    S.Вставить("Идентификатор", "A-01");
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    Стр = ТЗ.Добавить();
КонецПроцедуры
"#;
    let program = parse(source);
    let _index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let after = repository_impl.get_all_types();
    assert_eq!(after.len(), before.len());
    assert_eq!(
        after.iter().map(|ty| ty.name.as_str()).collect::<Vec<_>>(),
        before.iter().map(|ty| ty.name.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn typed_structure_alias_keeps_structural_members() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    S.Вставить("Идентификатор", "A-01");
    S2 = S;
    probe = S2.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner_offset = source.rfind("S2.Идентификатор").expect("owner") as u32;
    let owner = index
        .type_at_byte_offset(owner_offset)
        .expect("type at structure owner");
    assert!(owner.find_structural_member("идентификатор").is_some());

    let property_offset = source.rfind("Идентификатор").expect("property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at structure property");
    assert_eq!(property.type_name(), "Строка");
}

#[test]
fn value_table_add_row_materializes_typed_row_members() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    Стр = ТЗ.Добавить();
    probe = Стр.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("Идентификатор").expect("row property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at row property");
    assert_eq!(property.type_name(), "Строка");
}

#[test]
fn foreach_over_value_table_materializes_typed_row_members() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    Для каждого Стр Из ТЗ Цикл
        probe = Стр.Идентификатор;
    КонецЦикла;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("Идентификатор").expect("foreach row property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at foreach row property");
    assert_eq!(property.type_name(), "Строка");
}

#[test]
fn structure_fields_survive_if_branch_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    Если Истина Тогда
        S.Вставить("Идентификатор", "A-01");
    КонецЕсли;
    probe = S.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("Идентификатор").expect("merged property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at merged property");
    assert_eq!(property.type_name(), "Строка");
}
