use super::*;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{
    FacetKind, MetadataKind, ParameterInfo, PrimitiveType, RawAttributeData, RawDataSource,
    RawPropertyData, RawTabularSectionData, RawTypeData, StructuralMemberId, StructuralMemberSpan,
    FORM_DATA_SEMANTICS_NOTE,
};
use bsl_shared::domain::{
    normalize_global_context_property_key, GlobalContextIndex, GlobalContextPropertyData,
    GLOBAL_CONTEXT_SOURCE_KEY_NOTE_PREFIX, GLOBAL_CONTEXT_SOURCE_NOTE,
};
use bsl_shared::TypeRepository;
use bsl_syntax::ParseOptions;
use std::path::Path;

fn parse(code: &str) -> Program {
    let parsed = bsl_syntax::parse(code, &ParseOptions::default()).expect("parse ok");
    parsed.program
}

fn ir_program(
    source: &str,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> bsl_shared::ir::SemanticProgram {
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse ok");
    let mut program = crate::AstToIrConverter::convert_with_resolver(
        parsed.program.clone(),
        source.to_string(),
        file_path.to_string(),
        deps.repository.clone(),
        deps.signature_index.clone(),
        deps.resolver.clone(),
    )
    .expect("convert to ir");
    super::materialize_semantic_facts_with_path_profiled(
        &mut program,
        &parsed.program,
        source,
        file_path,
        deps,
    );
    program
}

fn semantic_facts_profile(
    source: &str,
    file_path: &str,
    deps: Arc<SemanticDeps>,
) -> TypeIndexBuildProfile {
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse ok");
    let mut program = crate::AstToIrConverter::convert_with_resolver(
        parsed.program.clone(),
        source.to_string(),
        file_path.to_string(),
        deps.repository.clone(),
        deps.signature_index.clone(),
        deps.resolver.clone(),
    )
    .expect("convert to ir");
    super::materialize_semantic_facts_with_path_profiled(
        &mut program,
        &parsed.program,
        source,
        file_path,
        deps,
    )
}

fn structural_member_span_for_literal(source: &str, literal: &str) -> StructuralMemberSpan {
    structural_member_span_for_literal_occurrence(source, literal, 0)
}

fn structural_member_span_for_literal_occurrence(
    source: &str,
    literal: &str,
    occurrence: usize,
) -> StructuralMemberSpan {
    let start = source
        .match_indices(literal)
        .nth(occurrence)
        .map(|(offset, _)| offset)
        .unwrap_or_else(|| panic!("missing literal {literal} occurrence {occurrence}"))
        as u32;
    StructuralMemberSpan::new(start, start + literal.len() as u32)
}

fn global_context_index_with_property(
    name: &str,
    english_name: Option<&str>,
    prop_type: &str,
) -> Arc<GlobalContextIndex> {
    Arc::new(GlobalContextIndex::loaded(vec![
        GlobalContextPropertyData {
            name: name.to_string(),
            english_name: english_name.map(str::to_string),
            prop_type: Some(prop_type.to_string()),
            is_readonly: true,
            description: None,
            contexts: vec!["Global context".to_string()],
            source_key: format!("objects/Global context/properties/{name}"),
            source_path: None,
            normalized_key: normalize_global_context_property_key(name),
            english_normalized_key: english_name.map(normalize_global_context_property_key),
            collection_item_type: None,
        },
    ]))
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
    })
}

fn deps_with_metadata_global_context_types() -> Arc<SemanticDeps> {
    deps_with_metadata_global_context_types_for_dimension_item_type(Some("ОбъектМетаданных: Поле"))
}

fn deps_with_metadata_global_context_types_for_dimension_item_type(
    dimension_item_type: Option<&str>,
) -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "ОбъектМетаданныхКонфигурация".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "РегистрыНакопления".to_string(),
                    prop_type: "КоллекцияОбъектовМетаданных".to_string(),
                    is_readonly: true,
                    collection_item_type: Some("ОбъектМетаданных: РегистрНакопления".to_string()),
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияОбъектовМетаданных".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ОбъектМетаданных: РегистрНакопления".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Измерения".to_string(),
                    prop_type: "КоллекцияОбъектовМетаданных".to_string(),
                    is_readonly: true,
                    collection_item_type: dimension_item_type.map(str::to_string),
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ОбъектМетаданных: Поле".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "Имя".to_string(),
                    prop_type: "Строка".to_string(),
                    is_readonly: true,
                    collection_item_type: None,
                }],
                ..Default::default()
            },
        ])
        .expect("load metadata global context platform types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
        global_context_index: global_context_index_with_property(
            "Метаданные",
            Some("Metadata"),
            "ОбъектМетаданныхКонфигурация",
        ),
    })
}

fn deps_with_loaded_global_context_property(
    name: &str,
    english_name: Option<&str>,
    prop_type: &str,
) -> Arc<SemanticDeps> {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
        global_context_index: global_context_index_with_property(name, english_name, prop_type),
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
fn resolves_bare_global_context_property_from_loaded_index() {
    let source = r#"Функция Значение() Экспорт
    Возврат Синтетика;
КонецФункции
"#;
    let program = parse(source);
    let deps = deps_with_loaded_global_context_property("Синтетика", Some("Synthetic"), "Строка");
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source.find("Синтетика").expect("synthetic identifier") as u32;
    let resolution = index
        .type_at_byte_offset(offset)
        .expect("type at synthetic global context property");
    assert_eq!(resolution.type_name(), "Строка");
    assert!(
        resolution.is_undeclared_variable().is_none(),
        "loaded global-context property must not be undeclared: {resolution:?}"
    );
    assert!(
        resolution
            .metadata
            .notes
            .iter()
            .any(|note| note == GLOBAL_CONTEXT_SOURCE_NOTE),
        "resolution should carry global-context provenance note: {resolution:?}"
    );
    assert!(
        resolution
            .metadata
            .notes
            .iter()
            .any(|note| note.starts_with(GLOBAL_CONTEXT_SOURCE_KEY_NOTE_PREFIX)),
        "resolution should carry global-context source key note: {resolution:?}"
    );
}

#[test]
fn unavailable_global_context_index_does_not_invent_metadata_binding() {
    let source = r#"Функция Значение() Экспорт
    Возврат Метаданные;
КонецФункции
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", Arc::new(SemanticDeps::empty()));

    let offset = source.find("Метаданные").expect("metadata identifier") as u32;
    let resolution = index
        .type_at_byte_offset(offset)
        .expect("type at unavailable global context property");
    assert_eq!(resolution.is_undeclared_variable(), Some("Метаданные"));
}

#[test]
fn local_variable_shadows_loaded_global_context_property() {
    let source = r#"Функция Значение() Экспорт
    Метаданные = "local";
    Возврат Метаданные;
КонецФункции
"#;
    let program = parse(source);
    let deps = deps_with_loaded_global_context_property(
        "Метаданные",
        Some("Metadata"),
        "ОбъектМетаданныхКонфигурация",
    );
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let return_offset = source
        .match_indices("Метаданные")
        .nth(1)
        .map(|(offset, _)| offset as u32)
        .expect("return metadata identifier");
    let resolution = index
        .type_at_byte_offset(return_offset)
        .expect("type at shadowed Метаданные");
    assert_eq!(resolution.type_name(), "Строка");
    assert!(
        !resolution
            .metadata
            .notes
            .iter()
            .any(|note| note == GLOBAL_CONTEXT_SOURCE_NOTE),
        "local shadow must not keep global-context provenance: {resolution:?}"
    );
}

#[test]
fn loaded_global_context_property_wins_over_legacy_global_collection_table() {
    let source = r#"Функция Значение() Экспорт
    Возврат РегистрыНакопления;
КонецФункции
"#;
    let program = parse(source);
    let deps = deps_with_loaded_global_context_property(
        "РегистрыНакопления",
        Some("AccumulationRegisters"),
        "СинтетическийМенеджерРегистров",
    );
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source
        .find("РегистрыНакопления")
        .expect("global manager property") as u32;
    let resolution = index
        .type_at_byte_offset(offset)
        .expect("type at loaded global manager property");
    assert_eq!(resolution.type_name(), "СинтетическийМенеджерРегистров");
    assert!(
        resolution
            .metadata
            .notes
            .iter()
            .any(|note| note == GLOBAL_CONTEXT_SOURCE_NOTE),
        "loaded global-context property must carry source provenance: {resolution:?}"
    );
}

#[test]
fn loaded_global_manager_collection_resolves_from_global_context_index() {
    let source = r#"Функция Менеджеры() Экспорт
    Возврат РегистрыНакопления;
КонецФункции
"#;
    let program = parse(source);
    let deps = deps_with_loaded_global_context_property(
        "РегистрыНакопления",
        Some("AccumulationRegisters"),
        "РегистрыНакопленияМенеджер",
    );
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let offset = source
        .find("РегистрыНакопления")
        .expect("global manager collection") as u32;
    let resolution = index
        .type_at_byte_offset(offset)
        .expect("type at loaded global manager collection");
    assert_eq!(resolution.type_name(), "РегистрыНакопленияМенеджер");
    assert!(
        resolution
            .metadata
            .notes
            .iter()
            .any(|note| note == GLOBAL_CONTEXT_SOURCE_NOTE),
        "loaded manager collection should come from GlobalContextIndex: {resolution:?}"
    );
}

#[test]
fn resolves_metadata_global_context_accumulation_register_dimension_name_chain() {
    let source = r#"Функция РеквизитГоловнаяОрганизация() Экспорт
    Возврат Метаданные.РегистрыНакопления.АвансовыеПлатежиИностранцевПоНДФЛ.Измерения.ГоловнаяОрганизация.Имя;
КонецФункции
"#;
    let program = parse(source);
    let deps = deps_with_metadata_global_context_types();
    let index = build_type_index_with_path(
        &program,
        "AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl",
        deps,
    );

    let metadata_offset = source.find("Метаданные").expect("metadata identifier") as u32;
    let metadata_type = index
        .type_at_byte_offset(metadata_offset)
        .expect("type at Метаданные");
    assert_eq!(metadata_type.type_name(), "ОбъектМетаданныхКонфигурация");
    assert!(
        metadata_type.is_undeclared_variable().is_none(),
        "Метаданные is a predefined global context property, got: {metadata_type:?}"
    );

    let name_offset = source.rfind("Имя").expect("final Name property") as u32;
    let name_type = index
        .type_at_byte_offset(name_offset)
        .expect("type at metadata field name");
    assert_eq!(name_type.type_name(), "Строка");
}

#[test]
fn nested_metadata_collection_item_type_falls_back_when_source_property_has_no_item_type() {
    let source = r#"Функция РеквизитГоловнаяОрганизация() Экспорт
    Возврат Метаданные.РегистрыНакопления.АвансовыеПлатежиИностранцевПоНДФЛ.Измерения.ГоловнаяОрганизация.Имя;
КонецФункции
"#;
    let program = parse(source);
    let deps = deps_with_metadata_global_context_types_for_dimension_item_type(None);
    let index = build_type_index_with_path(
        &program,
        "AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl",
        deps,
    );

    let name_offset = source.rfind("Имя").expect("final Name property") as u32;
    let name_type = index
        .type_at_byte_offset(name_offset)
        .expect("type at metadata field name");
    assert_eq!(name_type.type_name(), "Строка");
}

#[test]
fn resolves_conf_big_metadata_manager_module_lines_32_and_36() {
    let source = include_str!(
        "../../../examples/conf_big/AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl"
    );
    let program = parse(source);
    let deps = deps_with_metadata_global_context_types();
    let index = build_type_index_with_path(
        &program,
        "AccumulationRegisters/АвансовыеПлатежиИностранцевПоНДФЛ/Ext/ManagerModule.bsl",
        deps,
    );

    for marker in ["ГоловнаяОрганизация.Имя", "ФизическоеЛицо.Имя"]
    {
        let marker_start = source
            .find(marker)
            .unwrap_or_else(|| panic!("missing conf_big marker {marker}"));
        let name_offset = marker_start
            + marker
                .rfind("Имя")
                .unwrap_or_else(|| panic!("missing final Имя in marker {marker}"));
        let name_type = index
            .type_at_byte_offset(name_offset as u32)
            .unwrap_or_else(|| panic!("type at final Имя for marker {marker}"));
        assert_eq!(name_type.type_name(), "Строка", "marker {marker}");
    }
}

#[test]
fn metadata_collection_property_uses_repository_before_legacy_table() {
    let source = r#"Функция Коллекция() Экспорт
    Возврат Метаданные.РегистрыНакопления;
КонецФункции
"#;
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "ОбъектМетаданныхКонфигурация".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "РегистрыНакопления".to_string(),
                    prop_type: "СинтетическаяКоллекцияМетаданных".to_string(),
                    is_readonly: true,
                    collection_item_type: None,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "СинтетическаяКоллекцияМетаданных".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load configuration metadata repository property");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let deps = Arc::new(SemanticDeps {
        repository: repository.clone(),
        signature_index: SignatureIndex::new(),
        resolver: Some(Arc::new(TypeResolver::new(repository))),
        platform_signatures_loaded: true,
        global_context_index: global_context_index_with_property(
            "Метаданные",
            Some("Metadata"),
            "ОбъектМетаданныхКонфигурация",
        ),
    });
    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let collection_offset = source
        .find("РегистрыНакопления")
        .expect("metadata collection property") as u32;
    let collection_type = index
        .type_at_byte_offset(collection_offset)
        .expect("type at metadata collection property");
    assert_eq!(
        collection_type.type_name(),
        "СинтетическаяКоллекцияМетаданных"
    );
}

#[test]
fn metadata_collection_item_type_note_comes_from_source_property_instance() {
    let source = r#"Процедура Тест()
    Первый = Метаданные.СинтетическиеОбъекты.Первый;
    Второй = Метаданные.ДругиеОбъекты.Второй;
КонецПроцедуры
"#;
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "ОбъектМетаданныхКонфигурация".to_string(),
                source: RawDataSource::Platform,
                properties: vec![
                    RawPropertyData {
                        name: "СинтетическиеОбъекты".to_string(),
                        prop_type: "КоллекцияОбъектовМетаданных".to_string(),
                        is_readonly: true,
                        collection_item_type: Some("ОбъектМетаданных: Синтетика".to_string()),
                    },
                    RawPropertyData {
                        name: "ДругиеОбъекты".to_string(),
                        prop_type: "КоллекцияОбъектовМетаданных".to_string(),
                        is_readonly: true,
                        collection_item_type: Some("ОбъектМетаданных: ДругаяСинтетика".to_string()),
                    },
                ],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияОбъектовМетаданных".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ОбъектМетаданных: Синтетика".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ОбъектМетаданных: ДругаяСинтетика".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load per-property metadata collection item types");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let deps = Arc::new(SemanticDeps {
        repository: repository.clone(),
        signature_index: SignatureIndex::new(),
        resolver: Some(Arc::new(TypeResolver::new(repository))),
        platform_signatures_loaded: true,
        global_context_index: global_context_index_with_property(
            "Метаданные",
            Some("Metadata"),
            "ОбъектМетаданныхКонфигурация",
        ),
    });
    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let first_object_offset = source
        .match_indices("Первый")
        .nth(1)
        .map(|(offset, _)| offset as u32)
        .expect("first metadata object name");
    let second_object_offset = source
        .match_indices("Второй")
        .nth(1)
        .map(|(offset, _)| offset as u32)
        .expect("second metadata object name");

    let first_object_type = index
        .type_at_byte_offset(first_object_offset)
        .expect("type at first metadata object name");
    let second_object_type = index
        .type_at_byte_offset(second_object_offset)
        .expect("type at second metadata object name");

    assert_eq!(first_object_type.type_name(), "ОбъектМетаданных: Синтетика");
    assert_eq!(
        second_object_type.type_name(),
        "ОбъектМетаданных: ДругаяСинтетика"
    );
}

#[test]
fn metadata_collection_element_name_uses_item_type_before_collection_properties() {
    let source = r#"Функция ОбъектМетаданных() Экспорт
    Возврат Метаданные.СинтетическиеОбъекты.СинтетическийРегистр;
КонецФункции
"#;
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![
            RawTypeData {
                name: "ОбъектМетаданныхКонфигурация".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "СинтетическиеОбъекты".to_string(),
                    prop_type: "КоллекцияОбъектовМетаданных".to_string(),
                    is_readonly: true,
                    collection_item_type: Some("ОбъектМетаданных: Синтетика".to_string()),
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "КоллекцияОбъектовМетаданных".to_string(),
                source: RawDataSource::Platform,
                properties: vec![RawPropertyData {
                    name: "СинтетическийРегистр".to_string(),
                    prop_type: "Булево".to_string(),
                    is_readonly: true,
                    collection_item_type: None,
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "ОбъектМетаданных: Синтетика".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load metadata collection element fixture");

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let deps = Arc::new(SemanticDeps {
        repository: repository.clone(),
        signature_index: SignatureIndex::new(),
        resolver: Some(Arc::new(TypeResolver::new(repository))),
        platform_signatures_loaded: true,
        global_context_index: global_context_index_with_property(
            "Метаданные",
            Some("Metadata"),
            "ОбъектМетаданныхКонфигурация",
        ),
    });
    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", deps);

    let object_name_offset = source.find("СинтетическийРегистр").expect("object name") as u32;
    let object_name_type = index
        .type_at_byte_offset(object_name_offset)
        .expect("type at metadata object name");

    assert_eq!(object_name_type.type_name(), "ОбъектМетаданных: Синтетика");
}

#[test]
fn recovers_receiver_type_for_incomplete_bare_member_access_after_assignment() {
    let source =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let parsed = bsl_syntax::parse(source, &bsl_syntax::ParseOptions::default())
        .expect("parse with recovery");
    assert!(
        parsed.has_errors(),
        "incomplete bare member access fixture must exercise parser recovery"
    );

    let deps = deps_with_array_method();
    let index = build_type_index_from_parse_result_with_path(&parsed, source, "test.bsl", deps);
    let receiver_offset = source
        .find("    ЛокМассив.\n")
        .map(|idx| idx + "    ЛокМассив".len() - 1)
        .expect("receiver offset") as u32;
    let receiver_type = index
        .type_at_byte_offset(receiver_offset)
        .expect("recovered receiver type");

    assert_eq!(
        receiver_type.type_name(),
        "Массив<Неопределено>",
        "recovered bare member-access receiver must retain prior assignment type"
    );
}

#[test]
fn builds_type_index_from_semantic_program_for_simple_assignment_and_method_call() {
    let source = r#"Перем М;
М = Новый Массив();
Р = М.Количество();
"#;
    let file_path = "test.bsl";
    let program = parse(source);
    let deps = deps_with_array_method();
    let ir_program = ir_program(source, file_path, deps.clone());
    let legacy_index = build_type_index_with_path(&program, file_path, deps.clone());
    let ir_index = build_type_index_from_semantic_program_with_path(&ir_program, file_path, deps);

    let array_ident_offset = source
        .find("\nМ =")
        .map(|idx| idx + 1)
        .expect("assignment line start") as u32;
    assert_eq!(
        ir_index.type_at_byte_offset(array_ident_offset),
        legacy_index.type_at_byte_offset(array_ident_offset),
        "IR-backed builder must preserve assignment type inference contract"
    );

    let method_call_offset = source.find("Количество").expect("method name") as u32;
    assert_eq!(
        ir_index.type_at_byte_offset(method_call_offset),
        legacy_index.type_at_byte_offset(method_call_offset),
        "IR-backed builder must preserve method-call return type inference contract"
    );
    assert_eq!(
        ir_index.assignment_value_type_at_byte_offset(array_ident_offset),
        legacy_index.assignment_value_type_at_byte_offset(array_ident_offset),
        "IR-backed builder must preserve assignment value hint projection"
    );
    assert_eq!(
        ir_index.call_arg_types_at_byte_offset(method_call_offset),
        legacy_index.call_arg_types_at_byte_offset(method_call_offset),
        "IR-backed builder must preserve call arg hint projection"
    );
}

#[test]
fn semantic_program_index_materializes_configuration_symbol_exact_span_and_definition_anchor() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Документы.Док1".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            metadata_path: Some("Documents/Док1.xml".into()),
            ..Default::default()
        }])
        .expect("load config document type");
    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
        global_context_index: Default::default(),
    });

    let source = concat!(
        "Процедура Тест()\n",
        "    Результат = Документы.Док1;\n",
        "КонецПроцедуры\n",
    );
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse ok");
    assert!(
        parsed.syntax_errors.is_empty(),
        "complete configuration symbol fixture must parse without syntax recovery, errors={:?}",
        parsed.syntax_errors
    );
    let Statement::ProcedureDecl { body, .. } = &parsed.program.statements[0] else {
        panic!("expected procedure declaration");
    };
    assert_eq!(body.len(), 1, "expected assignment inside procedure body");
    let file_path = "inline.bsl";
    let ir_program = ir_program(source, file_path, deps.clone());
    let ir_index = build_type_index_from_semantic_program_with_path(&ir_program, file_path, deps);

    let member_offset = source.rfind("Док1").expect("document member") as u32;
    let resolved = ir_index
        .type_at_byte_offset(member_offset)
        .expect("type at configuration symbol member");
    assert_eq!(resolved.type_name(), "ДокументМенеджер.Док1");

    let definition = ir_index
        .definition_location_at_byte_offset(member_offset)
        .expect("definition anchor at configuration symbol member");
    assert_eq!(
        definition.primary_path().expect("metadata xml path"),
        Path::new("Documents/Док1.xml")
    );
}

#[test]
fn semantic_program_index_uses_materialized_facts_instead_of_reseeding_path() {
    let deps = deps_with_array_method();
    let source = r#"Процедура Тест()
    x = ЭтотОбъект;
    y = Объект;
КонецПроцедуры
"#;
    let canonical_file_path = "Documents/Док1/Ext/ObjectModule.bsl";
    let ir_program = ir_program(source, canonical_file_path, deps.clone());
    let ir_index =
        build_type_index_from_semantic_program_with_path(&ir_program, "inline.bsl", deps);

    let this_object_offset = source.find("ЭтотОбъект").expect("ЭтотОбъект") as u32;
    let this_object = ir_index
        .type_at_byte_offset(this_object_offset)
        .expect("type at ЭтотОбъект");
    assert_eq!(this_object.type_name(), "ДокументОбъект.Док1");

    let object_offset = source.find("Объект").expect("Объект") as u32;
    let object = ir_index
        .type_at_byte_offset(object_offset)
        .expect("type at Объект");
    assert_eq!(object.type_name(), "ДокументОбъект.Док1");
}

#[test]
fn recovers_receiver_type_from_semantic_program_for_incomplete_bare_member_access() {
    let source =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let file_path = "test.bsl";
    let parsed = bsl_syntax::parse(source, &bsl_syntax::ParseOptions::default())
        .expect("parse with recovery");
    assert!(
        parsed.has_errors(),
        "incomplete bare member access fixture must exercise parser recovery"
    );

    let deps = deps_with_array_method();
    let mut ir_program = crate::AstToIrConverter::convert_with_resolver(
        parsed.program.clone(),
        source.to_string(),
        file_path.to_string(),
        deps.repository.clone(),
        deps.signature_index.clone(),
        deps.resolver.clone(),
    )
    .expect("convert to ir");
    super::materialize_semantic_facts_with_recovery_with_path_profiled(
        &mut ir_program,
        &parsed,
        source,
        file_path,
        deps.clone(),
    );
    let legacy_index =
        build_type_index_from_parse_result_with_path(&parsed, source, file_path, deps.clone());
    let ir_index = build_type_index_from_semantic_program_with_recovery_with_path(
        &ir_program,
        source,
        &parsed.syntax_errors,
        file_path,
        deps,
    );

    let receiver_offset = source
        .find("    ЛокМассив.\n")
        .map(|idx| idx + "    ЛокМассив".len() - 1)
        .expect("receiver offset") as u32;
    assert_eq!(
        ir_index.type_at_byte_offset(receiver_offset),
        legacy_index.type_at_byte_offset(receiver_offset),
        "IR-backed builder must preserve syntax-only recovery for incomplete member access"
    );
}

#[test]
fn canonical_semantic_program_materializes_parenthesized_incomplete_receiver_from_source_text() {
    let source = concat!(
        "Процедура Тест()\n",
        "    ДляCompletion = (Новый Массив()).\n",
        "КонецПроцедуры\n",
    );
    let file_path = "test.bsl";
    let deps = deps_with_array_method();
    let ir_program = ir_program(source, file_path, deps.clone());
    let ir_index = build_type_index_from_semantic_program_with_path(&ir_program, file_path, deps);

    let probe = source
        .find("Новый Массив()")
        .map(|idx| idx + "Новый Массив()".len() - 1)
        .expect("parenthesized receiver probe") as u32;
    let resolved = ir_index
        .type_at_byte_offset(probe)
        .expect("type at parenthesized incomplete receiver");

    assert_eq!(
        resolved.type_name(),
        "Массив<Неопределено>",
        "canonical semantic facts must preserve exact owner type for parenthesized incomplete member access"
    );
}

#[test]
fn canonical_semantic_program_does_not_materialize_bare_incomplete_receiver_from_source_text() {
    let source = concat!(
        "Процедура Тест()\n",
        "    ЛокМассив = Новый Массив;\n",
        "    ЛокМассив.\n",
        "КонецПроцедуры\n",
    );
    let file_path = "test.bsl";
    let deps = deps_with_array_method();
    let ir_program = ir_program(source, file_path, deps.clone());
    let ir_index = build_type_index_from_semantic_program_with_path(&ir_program, file_path, deps);

    let probe = source
        .match_indices("ЛокМассив")
        .nth(1)
        .map(|(idx, marker)| idx + marker.len() - 1)
        .expect("second local receiver occurrence") as u32;

    assert_eq!(
        ir_index.type_at_byte_offset(probe),
        None,
        "source-text completion extraction must not reintroduce bare identifier fallback into canonical semantic facts"
    );
}

#[test]
fn canonical_semantic_program_materializes_object_module_incomplete_binding_from_source_text() {
    let source = concat!(
        "Процедура Тест()\n",
        "    ЭтотОбъект.\n",
        "КонецПроцедуры\n",
    );
    let file_path = "Documents/Док1/Ext/ObjectModule.bsl";
    let deps = deps_with_form_attribute_to_value_signature();
    let ir_program = ir_program(source, file_path, deps.clone());
    let ir_index = build_type_index_from_semantic_program_with_path(&ir_program, file_path, deps);

    let probe = source
        .find("ЭтотОбъект")
        .map(|idx| idx + "ЭтотОбъект".len() - 1)
        .expect("object module receiver probe") as u32;
    let resolved = ir_index
        .type_at_byte_offset(probe)
        .expect("type at object module incomplete receiver");

    assert_eq!(resolved.type_name(), "ДокументОбъект.Док1");
    assert_eq!(resolved.active_facet, Some(FacetKind::Object));
}

#[test]
fn extracts_parenthesized_choice_receiver_slices_with_partial_member_tail() {
    let source = concat!(
        "Процедура Тест()\n",
        "    __tmp = (Выбор Когда Истина Тогда Новый TypeA Иначе Новый TypeB КонецВыбора). X;\n",
        "КонецПроцедуры\n",
    );
    let dot_offset = source.find("). X").expect("choice dot") + 1;
    let slices = extract_incomplete_member_access_receiver_slices_at_dot_offset(source, dot_offset);
    let texts: Vec<&str> = slices
        .iter()
        .map(|(span, _)| &source[span.start as usize..span.end as usize])
        .collect();

    assert_eq!(texts, vec!["Новый TypeA", "Новый TypeB"]);
}

#[test]
fn filters_precomputed_incomplete_member_access_offsets_by_span() {
    let source = concat!(
        "Процедура Первая()\n",
        "    Первый.\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура Вторая()\n",
        "    Второй.\n",
        "КонецПроцедуры\n",
    );
    let all_offsets = incomplete_member_access_dot_offsets(source);
    let second_start = source
        .find("Процедура Вторая()")
        .expect("second procedure start");
    let second_end = source
        .match_indices("КонецПроцедуры")
        .nth(1)
        .map(|(offset, marker)| offset + marker.len())
        .expect("second procedure end");
    let span = bsl_shared::ir::Span::new(second_start as u32, second_end as u32);

    let direct = scan_incomplete_member_access_dot_offsets_within_span(source, span);
    let filtered =
        incomplete_member_access_dot_offsets_within_span_from_candidates(&all_offsets, span);

    assert_eq!(filtered, direct);
    assert_eq!(filtered.len(), 1);
}

#[test]
fn incomplete_member_access_offsets_ignore_comment_only_lines() {
    let source = concat!(
        "Процедура Тест()\n",
        "    // Комментарий с точкой.\n",
        "    Значение.\n",
        "КонецПроцедуры\n",
    );

    let offsets = incomplete_member_access_dot_offsets(source);
    let expected_offset = source
        .find("    Значение.")
        .map(|idx| idx + "    Значение".len())
        .expect("incomplete member access offset");

    assert_eq!(
        offsets,
        vec![expected_offset],
        "comment-only lines ending with '.' must not be treated as incomplete member access candidates"
    );
}

#[test]
fn incomplete_member_access_offsets_ignore_complete_member_tails() {
    let source = concat!(
        "Процедура Тест()\n",
        "    Значение.Свойство;\n",
        "    ДругоеЗначение.\n",
        "КонецПроцедуры\n",
    );

    let offsets = incomplete_member_access_dot_offsets(source);
    let expected_offset = source
        .find("    ДругоеЗначение.")
        .map(|idx| idx + "    ДругоеЗначение".len())
        .expect("incomplete member access offset");

    assert_eq!(
        offsets,
        vec![expected_offset],
        "source-only incomplete member access scan must ignore complete member tails like 'obj.Member;'"
    );
}

#[test]
fn parenthesized_choice_with_partial_member_tail_triggers_parser_recovery() {
    let source = concat!(
        "Процедура Тест()\n",
        "    __tmp = (Выбор Когда Истина Тогда Новый TypeA Иначе Новый TypeB КонецВыбора). X;\n",
        "КонецПроцедуры\n",
    );
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse with recovery");

    assert!(
        parsed.has_errors(),
        "one-line parenthesized choice completion fixture must go through parser recovery"
    );
}

#[test]
fn recovers_choice_branch_types_for_partial_member_tail() {
    let source = concat!(
        "Процедура Тест()\n",
        "    __tmp = (Выбор Когда Истина Тогда Новый Массив() Иначе Новый Массив() КонецВыбора). X;\n",
        "КонецПроцедуры\n",
    );
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse with recovery");
    let deps = deps_with_array_method();
    let index = build_type_index_from_parse_result_with_path(&parsed, source, "test.bsl", deps);

    let probes: Vec<u32> = source
        .match_indices("Новый Массив()")
        .map(|(idx, _)| (idx + "Новый Массив()".len() - 1) as u32)
        .collect();
    assert_eq!(
        probes.len(),
        2,
        "fixture must contain two array constructor probes"
    );

    for probe in probes {
        let resolved = index
            .type_at_byte_offset(probe)
            .expect("recovered branch type for choice receiver");
        assert_eq!(resolved.type_name(), "Массив<Неопределено>");
    }
}

#[test]
fn manual_recovery_records_choice_branch_types_for_partial_member_tail() {
    let source = concat!(
        "Процедура Тест()\n",
        "    __tmp = (Выбор Когда Истина Тогда Новый Массив() Иначе Новый Массив() КонецВыбора). X;\n",
        "КонецПроцедуры\n",
    );
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse with recovery");
    let deps = deps_with_array_method();
    let inferencer = TypeInferencer::new(deps);
    let mut facts = SemanticFacts::default();
    let env = TypeEnv::default();
    inferencer.record_incomplete_member_access_recovery_entries(
        RecoveryContext {
            source_text: source,
            syntax_errors: &parsed.syntax_errors,
        },
        bsl_shared::ir::Span::new(0, source.len() as u32),
        &env,
        &mut facts,
    );

    let probes: Vec<u32> = source
        .match_indices("Новый Массив()")
        .map(|(idx, _)| (idx + "Новый Массив()".len() - 1) as u32)
        .collect();
    assert_eq!(
        probes.len(),
        2,
        "fixture must contain two array constructor probes"
    );

    for probe in probes {
        let resolved = facts
            .type_at_byte_offset(probe)
            .expect("manual recovery branch type for choice receiver");
        assert_eq!(resolved.type_name(), "Массив<Неопределено>");
    }
}

#[test]
fn incomplete_constructor_call_does_not_materialize_recovery_target_in_canonical_ir() {
    let source = concat!(
        "Процедура Тест()\n",
        "    Новый Массив(1, )\n",
        "КонецПроцедуры\n",
    );
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse with recovery");
    assert!(
        parsed.has_errors(),
        "incomplete constructor fixture must go through parser recovery"
    );

    let deps = deps_with_array_method();
    let program = ir_program(source, "test.bsl", deps);
    let call_offset = source
        .find("Новый Массив(")
        .map(|idx| idx + "Новый Массив".len())
        .expect("constructor call offset") as u32;

    let target = program
        .semantic_facts
        .constructor_targets_by_span
        .iter()
        .find(|(span, target)| {
            span.contains(call_offset) && target.type_name.eq_ignore_ascii_case("Массив")
        });
    assert!(
        target.is_none(),
        "canonical IR must not synthesize constructor target from parse recovery for incomplete constructor call"
    );
}

#[test]
fn diagnostics_only_build_omits_exact_only_and_projection_only_fact_surfaces() {
    let source = r#"Процедура Тест()
    М = Новый Массив();
    М.Количество();
    Док = FormAttributeToValue("Док1");
    Ссылка = Док.Ссылка;
КонецПроцедуры
"#;
    let parsed = bsl_syntax::parse(source, &ParseOptions::default()).expect("parse ok");
    let deps = deps_with_form_attribute_to_value_signature();

    let full = TypeInferencer::new(deps.clone()).build_facts_internal(
        &parsed.program,
        "test.bsl",
        Some(source),
        None,
    );
    assert!(
        !full.facts.assignment_value_type_by_span.is_empty(),
        "full semantic build should still materialize assignment hint input facts"
    );
    assert!(
        !full.facts.call_arg_types_by_span.is_empty(),
        "full semantic build should still materialize call arg hint input facts"
    );
    assert!(
        !full.facts.call_receiver_type_by_span.is_empty(),
        "full semantic build should still materialize call receiver hint input facts"
    );
    assert!(
        !full.facts.member_access_object_type_by_span.is_empty(),
        "full semantic build should still materialize member access hint input facts"
    );
    assert!(
        !full.facts.call_method_targets_by_span.is_empty(),
        "full semantic build should still materialize exact call targets"
    );
    assert!(
        !full.facts.constructor_targets_by_span.is_empty(),
        "full semantic build should still materialize constructor targets"
    );

    let diagnostics_only = TypeInferencer::with_materialization_mode_and_checkpoint(
        deps,
        SemanticMaterializationMode::DiagnosticsOnly,
        None,
    )
    .build_facts_internal(&parsed.program, "test.bsl", None, None);
    assert!(
        diagnostics_only
            .facts
            .assignment_value_type_by_span
            .is_empty(),
        "diagnostics-only build should not keep legacy assignment hint maps in SemanticFacts"
    );
    assert!(
        diagnostics_only.facts.call_arg_types_by_span.is_empty(),
        "diagnostics-only build should not keep legacy call arg hint maps in SemanticFacts"
    );
    assert!(
        diagnostics_only.facts.call_receiver_type_by_span.is_empty(),
        "diagnostics-only build should not keep legacy call receiver hint maps in SemanticFacts"
    );
    assert!(
        diagnostics_only
            .facts
            .member_access_object_type_by_span
            .is_empty(),
        "diagnostics-only build should not keep legacy member-access hint maps in SemanticFacts"
    );
    assert!(
        diagnostics_only
            .facts
            .call_method_targets_by_span
            .is_empty(),
        "diagnostics-only build should not materialize exact call targets"
    );
    assert!(
        diagnostics_only
            .facts
            .member_method_targets_by_span
            .is_empty(),
        "diagnostics-only build should not materialize exact member targets"
    );
    assert!(
        diagnostics_only
            .facts
            .constructor_targets_by_span
            .is_empty(),
        "diagnostics-only build should not materialize constructor targets"
    );
    assert!(
        diagnostics_only
            .facts
            .definition_locations_by_span
            .is_empty(),
        "diagnostics-only build should not materialize definition locations"
    );
    let diagnostics_hints = diagnostics_only
        .diagnostics_type_hints
        .as_ref()
        .expect("diagnostics-only build should materialize direct type hints");
    assert!(
        !diagnostics_hints.assignment_value_type_by_span.is_empty(),
        "diagnostics-only build should materialize assignment hints directly"
    );
    assert!(
        !diagnostics_hints.call_arg_types_by_span.is_empty(),
        "diagnostics-only build should materialize call arg hints directly"
    );
    assert!(
        !diagnostics_hints.call_receiver_type_by_span.is_empty(),
        "diagnostics-only build should materialize call receiver hints directly"
    );
    assert!(
        !diagnostics_hints
            .member_access_object_type_by_span
            .is_empty(),
        "diagnostics-only build should materialize member-access hints directly"
    );
    assert!(
        diagnostics_only.facts.type_entries.is_empty(),
        "diagnostics-only build should no longer retain projection-only type entries"
    );
    assert!(
        diagnostics_only.profile.index_entry_count as usize
            >= diagnostics_only.facts.type_entries.len(),
        "diagnostics-only observability must remain truthful even without retained type entries"
    );
}

#[test]
fn diagnostics_type_hints_cover_program_requires_receiver_for_object_span_only_method_calls() {
    let mut program = bsl_shared::ir::SemanticProgram::new();
    let call_span = bsl_shared::ir::Span::new(10, 40);
    program.nodes.push(bsl_shared::ir::SemanticNode {
        kind: bsl_shared::ir::SemanticNodeKind::FunctionCall {
            function_name: "Метод".to_string(),
            object_name: None,
            object_node: None,
            object_span: Some(bsl_shared::ir::Span::new(10, 24)),
            arg_nodes: Vec::new(),
            arg_spans: Vec::new(),
        },
        span: call_span,
        scope_id: program.symbols.root_scope,
    });

    let mut hints = bsl_diagnostics::SemanticTypeHints::default();
    hints.call_arg_types_by_span.insert(call_span, Vec::new());
    assert!(
        !diagnostics_type_hints_cover_program(&program, &hints),
        "object_span-only method call must require receiver hint coverage"
    );

    hints.call_receiver_type_by_span.insert(
        call_span,
        bsl_shared::domain::types::TypeResolution::explicit("Массив"),
    );
    assert!(
        diagnostics_type_hints_cover_program(&program, &hints),
        "object_span-only method call should pass once receiver hint is present"
    );
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
fn singleton_non_recursive_local_summaries_skip_fixed_point_and_preserve_local_call_semantics() {
    let source = r#"Процедура Тест()
    x = СтрокаИзЛокальной();
КонецПроцедуры

Функция СтрокаИзЛокальной()
    Возврат "Тест";
КонецФункции
"#;
    let deps = deps_with_array_method();
    let profile = semantic_facts_profile(source, "test.bsl", deps.clone());
    assert_eq!(profile.local_function_summaries_function_count, 2);
    assert_eq!(profile.local_function_summaries_scc_count, 2);
    assert_eq!(
        profile.local_function_summaries_fixed_point_iteration_count,
        0
    );
    assert_eq!(
        profile.local_function_summaries_singleton_fast_path_count,
        2
    );
    assert_eq!(profile.local_function_summaries_recursive_scc_count, 0);

    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", deps);
    let offset = source.find("СтрокаИзЛокальной()").expect("call") as u32;
    let result = index.type_at_byte_offset(offset).expect("type at call");
    assert_eq!(result.type_name(), "Строка");

    let target = index
        .call_method_target_at_byte_offset(offset)
        .expect("local call target");
    assert_eq!(target.method_name, "СтрокаИзЛокальной");
    assert_eq!(
        target
            .signature
            .as_ref()
            .and_then(|signature| signature.return_type.as_deref()),
        Some("Строка")
    );
    assert_eq!(
        target.signature.as_ref().map(|signature| signature.source),
        Some(SignatureSource::UserCode)
    );
    assert!(target.definition_location.is_some());
}

#[test]
fn self_recursive_singleton_stays_on_convergence_path_and_preserves_local_call_semantics() {
    let source = r#"Функция Сама(Флаг)
    Если Флаг Тогда
        Возврат Сама(Ложь);
    КонецЕсли;
    Возврат 1;
КонецФункции

Процедура Тест()
    x = Сама(Истина);
КонецПроцедуры
"#;
    let deps = deps_with_array_method();
    let profile = semantic_facts_profile(source, "test.bsl", deps.clone());
    assert_eq!(profile.local_function_summaries_function_count, 2);
    assert_eq!(profile.local_function_summaries_scc_count, 2);
    assert!(profile.local_function_summaries_fixed_point_iteration_count > 0);
    assert_eq!(
        profile.local_function_summaries_singleton_fast_path_count,
        1
    );
    assert_eq!(profile.local_function_summaries_recursive_scc_count, 1);

    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", deps);
    let offset = source.find("Сама(Истина)").expect("call") as u32;
    let result = index.type_at_byte_offset(offset).expect("type at call");
    assert_eq!(result.type_name(), "Число");
    let target = index
        .call_method_target_at_byte_offset(offset)
        .expect("self-recursive call target");
    assert_eq!(target.method_name, "Сама");
    assert_eq!(
        target
            .signature
            .as_ref()
            .and_then(|signature| signature.return_type.as_deref()),
        Some("Число")
    );
    assert_eq!(
        target.signature.as_ref().map(|signature| signature.source),
        Some(SignatureSource::UserCode)
    );
    assert!(target.definition_location.is_some());
}

#[test]
fn mutually_recursive_local_summaries_reuse_stable_out_of_scc_semantics() {
    let source = r#"Функция ВнешняяСтрока()
    Возврат "Тест";
КонецФункции

Функция A(Флаг)
    Если Флаг Тогда
        Возврат ВнешняяСтрока();
    КонецЕсли;
    Возврат B();
КонецФункции

Функция B()
    Возврат A(Истина);
КонецФункции

Процедура Тест()
    x = A(Ложь);
КонецПроцедуры
"#;
    let deps = deps_with_array_method();
    let profile = semantic_facts_profile(source, "test.bsl", deps.clone());
    assert_eq!(profile.local_function_summaries_function_count, 4);
    assert_eq!(profile.local_function_summaries_scc_count, 3);
    assert!(profile.local_function_summaries_fixed_point_iteration_count > 0);
    assert_eq!(
        profile.local_function_summaries_singleton_fast_path_count,
        2
    );
    assert_eq!(profile.local_function_summaries_recursive_scc_count, 1);

    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", deps);
    let offset = source.find("A(Ложь)").expect("call") as u32;
    let result = index.type_at_byte_offset(offset).expect("type at call");
    assert_eq!(result.type_name(), "Строка");
    let target = index
        .call_method_target_at_byte_offset(offset)
        .expect("mutually recursive call target");
    assert_eq!(target.method_name, "A");
    assert_eq!(
        target
            .signature
            .as_ref()
            .and_then(|signature| signature.return_type.as_deref()),
        Some("Строка")
    );
    assert_eq!(
        target.signature.as_ref().map(|signature| signature.source),
        Some(SignatureSource::UserCode)
    );
}

#[test]
fn local_procedure_cycles_do_not_force_local_function_summary_convergence() {
    let source = r#"Процедура P()
    F();
КонецПроцедуры

Функция F()
    P();
    Возврат 1;
КонецФункции

Процедура Тест()
    x = F();
КонецПроцедуры
"#;
    let deps = deps_with_array_method();
    let profile = semantic_facts_profile(source, "test.bsl", deps.clone());
    assert_eq!(profile.local_function_summaries_function_count, 3);
    assert_eq!(profile.local_function_summaries_scc_count, 3);
    assert_eq!(
        profile.local_function_summaries_fixed_point_iteration_count,
        0
    );
    assert_eq!(
        profile.local_function_summaries_singleton_fast_path_count,
        3
    );
    assert_eq!(profile.local_function_summaries_recursive_scc_count, 0);

    let program = parse(source);
    let index = build_type_index_with_path(&program, "test.bsl", deps);
    let function_offset = source.rfind("F()").expect("test call") as u32;
    let result = index
        .type_at_byte_offset(function_offset)
        .expect("type at call");
    assert_eq!(result.type_name(), "Число");

    let function_target = index
        .call_method_target_at_byte_offset(function_offset)
        .expect("function call target");
    assert_eq!(function_target.method_name, "F");
    assert_eq!(
        function_target
            .signature
            .as_ref()
            .and_then(|signature| signature.return_type.as_deref()),
        Some("Число")
    );
    assert!(function_target.definition_location.is_some());

    let procedure_offset = source.find("P();").expect("procedure call") as u32;
    let procedure_target = index
        .call_method_target_at_byte_offset(procedure_offset)
        .expect("procedure call target");
    assert_eq!(procedure_target.method_name, "P");
    assert_eq!(
        procedure_target
            .signature
            .as_ref()
            .and_then(|signature| signature.return_type.as_deref()),
        None
    );
    assert!(procedure_target.definition_location.is_some());
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
                    collection_item_type: None,
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
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
        global_context_index: Default::default(),
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
fn object_module_bare_identifier_without_canonical_binding_stays_undeclared() {
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
                collection_item_type: None,
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
        global_context_index: Default::default(),
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
        .expect("type at bare identifier");
    assert_eq!(
        resolved.is_undeclared_variable(),
        Some("ДоговорКонтрагента"),
        "bare owner members without canonical binding must stay undeclared"
    );
}

#[test]
fn recordset_module_bare_identifier_without_canonical_binding_stays_undeclared() {
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
                collection_item_type: None,
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
        global_context_index: Default::default(),
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
        .expect("type at bare identifier");
    assert_eq!(
        resolved.is_undeclared_variable(),
        Some("ОбменДанными"),
        "recordset bare owner members without canonical binding must stay undeclared"
    );
}

#[test]
fn object_module_explicit_this_object_member_stays_available() {
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
                collection_item_type: None,
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
        global_context_index: Default::default(),
    });

    let source = r#"Процедура Тест()
    x = ЭтотОбъект.ДоговорКонтрагента;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "Documents/Док1/Ext/ObjectModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let offset = source.rfind("ДоговорКонтрагента").expect("member") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at explicit owner member");
    assert_eq!(resolved.type_name(), "Строка");
}

#[test]
fn recordset_module_explicit_object_member_stays_available() {
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
                collection_item_type: None,
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
        global_context_index: Default::default(),
    });

    let source = r#"Процедура Тест()
    x = Объект.ОбменДанными;
КонецПроцедуры
"#;
    let program = parse(source);
    let file_path = "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl";
    let index = build_type_index_with_path(&program, file_path, deps);

    let offset = source.rfind("ОбменДанными").expect("member") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at explicit owner member");
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
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
fn map_effect_store_uses_generic_value_when_literal_specialization_is_missing() {
    let mut effects = InstanceEffectStore::default();
    let base_resolution = TypeResolution::generic(
        "Соответствие",
        &["Строка", "ДокументСсылка.Док1"],
        Certainty::Known,
    );
    let binding = effects.new_map_instance(&base_resolution);
    let instance_id = InstanceEffectStore::direct_instance(&binding).expect("instance id");

    let resolved = effects
        .resolve_map_value(instance_id, Some("ЛюбойКлюч"))
        .expect("generic value resolution");

    assert_eq!(resolved.type_name(), "ДокументСсылка.Док1");
}

#[test]
fn map_index_access_without_effects_falls_back_to_proizvolny() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    map = Новый Соответствие;
    Ключ = "k";
    probe = map[Ключ];
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let offset = source
        .find("map[Ключ]")
        .map(|idx| idx + "map[Ключ]".len() - 1)
        .expect("map index") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at map index");

    assert_eq!(resolved.type_name(), "Произвольный");
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
                    collection_item_type: None,
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
        global_context_index: Default::default(),
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
fn typed_structure_insert_preserves_field_source_span() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    S.Вставить("Идентификатор", "A-01");
    probe = S.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner_offset = source.rfind("S.Идентификатор").expect("owner") as u32;
    let owner = index
        .type_at_byte_offset(owner_offset)
        .expect("type at structure owner");
    let member = owner
        .find_structural_member("идентификатор")
        .expect("typed structure field");

    assert_eq!(
        member.source_span,
        Some(structural_member_span_for_literal(
            source,
            "\"Идентификатор\""
        ))
    );
}

#[test]
fn typed_structure_alias_preserves_member_identity() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    S.Вставить("Идентификатор", "A-01");
    S2 = S;
    probe1 = S.Идентификатор;
    probe2 = S2.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner1_offset = source.find("S.Идентификатор").expect("owner1") as u32;
    let owner2_offset = source.rfind("S2.Идентификатор").expect("owner2") as u32;
    let owner1 = index
        .type_at_byte_offset(owner1_offset)
        .expect("type at first structure owner");
    let owner2 = index
        .type_at_byte_offset(owner2_offset)
        .expect("type at second structure owner");
    let member1 = owner1
        .find_structural_member("идентификатор")
        .expect("first typed structure field");
    let member2 = owner2
        .find_structural_member("идентификатор")
        .expect("second typed structure field");

    assert_eq!(member1.member_id, member2.member_id);
}

#[test]
fn typed_structure_case_insensitive_update_preserves_identity_and_canonical_name() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    S.Вставить("Идентификатор", "A-01");
    S.Вставить("идентификатор", 10);
    probe = S.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner_offset = source.rfind("S.Идентификатор").expect("owner") as u32;
    let owner = index
        .type_at_byte_offset(owner_offset)
        .expect("type at structure owner");
    let member = owner
        .find_structural_member("идентификатор")
        .expect("typed structure field");

    assert_eq!(owner.structural_members().len(), 1);
    assert_eq!(member.canonical_name, "Идентификатор");
    assert_eq!(
        member.member_id,
        StructuralMemberId::new(
            "Идентификатор",
            Some(structural_member_span_for_literal(
                source,
                "\"Идентификатор\""
            )),
        )
    );
    assert_eq!(
        member.source_span,
        Some(structural_member_span_for_literal(
            source,
            "\"идентификатор\""
        ))
    );
    assert_eq!(member.member_type.type_name(), "Число");
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
fn typed_value_table_row_preserves_column_source_span() {
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

    let owner_offset = source.rfind("Стр.Идентификатор").expect("row owner") as u32;
    let owner = index
        .type_at_byte_offset(owner_offset)
        .expect("type at row owner");
    let member = owner
        .find_structural_member("идентификатор")
        .expect("typed row column");

    assert_eq!(
        member.source_span,
        Some(structural_member_span_for_literal(
            source,
            "\"Идентификатор\""
        ))
    );
}

#[test]
fn typed_value_table_row_alias_preserves_column_identity() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    Стр = ТЗ.Добавить();
    Стр2 = Стр;
    probe1 = Стр.Идентификатор;
    probe2 = Стр2.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner1_offset = source.find("Стр.Идентификатор").expect("owner1") as u32;
    let owner2_offset = source.rfind("Стр2.Идентификатор").expect("owner2") as u32;
    let owner1 = index
        .type_at_byte_offset(owner1_offset)
        .expect("type at first row owner");
    let owner2 = index
        .type_at_byte_offset(owner2_offset)
        .expect("type at second row owner");
    let member1 = owner1
        .find_structural_member("идентификатор")
        .expect("first typed row column");
    let member2 = owner2
        .find_structural_member("идентификатор")
        .expect("second typed row column");

    assert_eq!(member1.member_id, member2.member_id);
}

#[test]
fn typed_value_table_column_case_insensitive_update_preserves_identity_and_canonical_name() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    ТЗ.Колонки.Добавить("идентификатор", Новый ОписаниеТипов("Число"));
    Стр = ТЗ.Добавить();
    probe = Стр.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner_offset = source.rfind("Стр.Идентификатор").expect("owner") as u32;
    let owner = index
        .type_at_byte_offset(owner_offset)
        .expect("type at row owner");
    let member = owner
        .find_structural_member("идентификатор")
        .expect("typed row column");

    assert_eq!(owner.structural_members().len(), 1);
    assert_eq!(member.canonical_name, "Идентификатор");
    assert_eq!(
        member.member_id,
        StructuralMemberId::new(
            "Идентификатор",
            Some(structural_member_span_for_literal(
                source,
                "\"Идентификатор\""
            )),
        )
    );
    assert_eq!(
        member.source_span,
        Some(structural_member_span_for_literal(
            source,
            "\"идентификатор\""
        ))
    );
    assert_eq!(member.member_type.type_name(), "Число");
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

#[test]
fn structure_field_identity_survives_branch_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    S.Вставить("Идентификатор", "A-01");
    probe_before = S.Идентификатор;
    Если Истина Тогда
        S.Вставить("Идентификатор", "B-02");
    Иначе
        S.Вставить("Идентификатор", "C-03");
    КонецЕсли;
    probe_after = S.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let before_owner_offset = source.find("S.Идентификатор").expect("before owner") as u32;
    let after_owner_offset = source.rfind("S.Идентификатор").expect("after owner") as u32;
    let before_owner = index
        .type_at_byte_offset(before_owner_offset)
        .expect("type before branch merge");
    let after_owner = index
        .type_at_byte_offset(after_owner_offset)
        .expect("type after branch merge");
    let before_member = before_owner
        .find_structural_member("идентификатор")
        .expect("structural member before merge");
    let after_member = after_owner
        .find_structural_member("идентификатор")
        .expect("structural member after merge");

    assert_eq!(before_member.member_id, after_member.member_id);
}

#[test]
fn structure_fields_survive_else_branch_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    Если Ложь Тогда
    Иначе
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

#[test]
fn structure_member_identity_survives_else_branch_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    Если Ложь Тогда
    Иначе
        S.Вставить("Идентификатор", "A-01");
        probe1 = S.Идентификатор;
    КонецЕсли;
    probe2 = S.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner1_offset = source.find("S.Идентификатор").expect("branch owner") as u32;
    let owner2_offset = source.rfind("S.Идентификатор").expect("merged owner") as u32;
    let owner1 = index
        .type_at_byte_offset(owner1_offset)
        .expect("type at branch structure owner");
    let owner2 = index
        .type_at_byte_offset(owner2_offset)
        .expect("type at merged structure owner");
    let member1 = owner1
        .find_structural_member("идентификатор")
        .expect("branch structural member");
    let member2 = owner2
        .find_structural_member("идентификатор")
        .expect("merged structural member");

    assert_eq!(member1.member_id, member2.member_id);
}

#[test]
fn map_literal_value_survives_else_branch_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    Map = Новый Соответствие;
    Если Ложь Тогда
    Иначе
        Map.Вставить("k", 10);
    КонецЕсли;
    probe = Map["k"];
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let offset = source
        .find("Map[\"k\"]")
        .map(|idx| idx + "Map[\"k\"]".len() - 1)
        .expect("map access") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at map access");
    assert_eq!(resolved.type_name(), "Число");
}

#[test]
fn value_table_columns_survive_else_branch_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    Если Ложь Тогда
    Иначе
        ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    КонецЕсли;
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
fn value_table_column_identity_survives_else_branch_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    Если Ложь Тогда
    Иначе
        ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
        Стр1 = ТЗ.Добавить();
        probe1 = Стр1.Идентификатор;
    КонецЕсли;
    Стр2 = ТЗ.Добавить();
    probe2 = Стр2.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let owner1_offset = source.find("Стр1.Идентификатор").expect("branch owner") as u32;
    let owner2_offset = source.rfind("Стр2.Идентификатор").expect("merged owner") as u32;
    let owner1 = index
        .type_at_byte_offset(owner1_offset)
        .expect("type at branch row owner");
    let owner2 = index
        .type_at_byte_offset(owner2_offset)
        .expect("type at merged row owner");
    let member1 = owner1
        .find_structural_member("идентификатор")
        .expect("branch row member");
    let member2 = owner2
        .find_structural_member("идентификатор")
        .expect("merged row member");

    assert_eq!(member1.member_id, member2.member_id);
}

#[test]
fn fresh_structure_instances_in_branches_merge_members_for_same_variable() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест(Флаг)
    Если Флаг Тогда
        S = Новый Структура;
        S.Вставить("Идентификатор", "A-01");
    Иначе
        S = Новый Структура;
        S.Вставить("Количество", 10);
    КонецЕсли;
    probe1 = S.Идентификатор;
    probe2 = S.Количество;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let string_offset = source.find("S.Идентификатор").expect("string property") as u32;
    let string_type = index
        .type_at_byte_offset(string_offset + "S.Идентификатор".len() as u32 - 1)
        .expect("type at string property");
    assert_eq!(string_type.type_name(), "Строка");

    let number_offset = source.rfind("S.Количество").expect("number property") as u32;
    let number_type = index
        .type_at_byte_offset(number_offset + "S.Количество".len() as u32 - 1)
        .expect("type at number property");
    assert_eq!(number_type.type_name(), "Число");
}

#[test]
fn map_alias_keeps_literal_specialization() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    Map = Новый Соответствие;
    Map.Вставить("k", 10);
    Map2 = Map;
    probe = Map2["k"];
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let offset = source
        .find("Map2[\"k\"]")
        .map(|idx| idx + "Map2[\"k\"]".len() - 1)
        .expect("aliased map access") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at aliased map access");
    assert_eq!(resolved.type_name(), "Число");
}

#[test]
fn value_table_row_alias_keeps_typed_columns() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    Стр = ТЗ.Добавить();
    Стр2 = Стр;
    probe = Стр2.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("Идентификатор").expect("row alias property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at row alias property");
    assert_eq!(property.type_name(), "Строка");
}

#[test]
fn branch_assigned_structure_binding_survives_then_only_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест(Флаг)
    S = Неопределено;
    Если Флаг Тогда
        S = Новый Структура;
        S.Вставить("Идентификатор", "A-01");
    КонецЕсли;
    probe = S.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("Идентификатор").expect("branch property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at branch property");
    assert_eq!(property.type_name(), "Строка");
}

#[test]
fn branch_assigned_map_binding_survives_else_only_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест(Флаг)
    Map = Неопределено;
    Если Флаг Тогда
    Иначе
        Map = Новый Соответствие;
        Map.Вставить("k", 10);
    КонецЕсли;
    probe = Map["k"];
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let offset = source
        .find("Map[\"k\"]")
        .map(|idx| idx + "Map[\"k\"]".len() - 1)
        .expect("branch map access") as u32;
    let resolved = index
        .type_at_byte_offset(offset)
        .expect("type at branch map access");
    assert_eq!(resolved.type_name(), "Число");
}

#[test]
fn branch_assigned_value_table_binding_survives_then_only_merge() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест(Флаг)
    ТЗ = Неопределено;
    Если Флаг Тогда
        ТЗ = Новый ТаблицаЗначений;
        ТЗ.Колонки.Добавить("Идентификатор", Новый ОписаниеТипов("Строка"));
    КонецЕсли;
    Стр = ТЗ.Добавить();
    probe = Стр.Идентификатор;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("Идентификатор").expect("branch row property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at branch row property");
    assert_eq!(property.type_name(), "Строка");
}

#[test]
fn structure_field_with_unknown_value_type_falls_back_to_proizvolny() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    S = Новый Структура;
    S.Вставить("СложноеПоле", ПолучитьНечёткийТип());
    probe = S.СложноеПоле;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("СложноеПоле").expect("field property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at structure property");

    assert_eq!(property.type_name(), "Произвольный");
}

#[test]
fn value_table_column_description_variable_string_type_materializes_string() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ОписаниеТиповСтрока150 = Новый ОписаниеТипов(КвалификаторыСтрок.StringType);
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("Идентификатор", ОписаниеТиповСтрока150);
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
fn value_table_column_with_unsupported_description_falls_back_to_proizvolny() {
    let deps = deps_with_universal_collection_types();
    let source = r#"Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Колонки.Добавить("СложнаяКолонка", ВычислитьОписаниеТипов());
    Стр = ТЗ.Добавить();
    probe = Стр.СложнаяКолонка;
КонецПроцедуры
"#;
    let program = parse(source);
    let index = build_type_index_with_path(&program, "Documents/Док1/Ext/ObjectModule.bsl", deps);

    let property_offset = source.rfind("СложнаяКолонка").expect("row property") as u32;
    let property = index
        .type_at_byte_offset(property_offset)
        .expect("type at row property");

    assert_eq!(property.type_name(), "Произвольный");
}
