use super::*;
use crate::data::loaders::syntax_helper::types::{
    GlobalFunctionInfo, MethodInfo, TypeDocumentation, TypeIdentity, TypeInfo, TypeMetadata,
    TypeStructure,
};
use crate::data::loaders::syntax_helper::{
    DocumentParser, PropertyInfo, PropertySourceKind, SyntaxHelperDatabase, SyntaxHelperLoader,
    SyntaxNode,
};
use bsl_shared::domain::signature_index::ContextRequirements;
use scraper::Html;
use std::path::Path;

#[test]
fn test_form_data_collection_collection_item_type_is_exported_to_raw() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let html_path = root.join(
        "examples/syntax_helper/rebuilt.shcntx_ru/objects/catalog1649/catalog1614/FormDataCollection.html",
    );

    let content =
        std::fs::read_to_string(&html_path).expect("failed to read FormDataCollection.html");
    let document = Html::parse_document(&content);

    let parser = DocumentParser::new();
    let type_info = parser
        .parse_type(&html_path, &document)
        .expect("failed to parse type info");

    assert_eq!(
        type_info.structure.collection_element.as_deref(),
        Some("ДанныеФормыЭлементКоллекции"),
        "Должны извлечь item type из реального HTML"
    );

    let mut db = SyntaxHelperDatabase::default();
    db.nodes
        .insert("k".to_string(), SyntaxNode::Type(type_info));

    let raw = convert_syntax_helper_to_raw(&db);
    let form_data_collection = raw
        .iter()
        .find(|t| t.name == "ДанныеФормыКоллекция")
        .expect("RawTypeData for ДанныеФормыКоллекция should be present");

    assert_eq!(
        form_data_collection.collection_item_type.as_deref(),
        Some("ДанныеФормыЭлементКоллекции")
    );
}

#[test]
fn test_fallback_collection_item_type_from_get_return_type() {
    let type_info = TypeInfo {
        identity: TypeIdentity {
            russian_name: "ТестКоллекция".to_string(),
            english_name: "TestCollection".to_string(),
            catalog_path: "objects/x/TestCollection.html".to_string(),
            aliases: Vec::new(),
            category_path: "Тесты".to_string(),
        },
        documentation: TypeDocumentation {
            category_description: None,
            type_description: "Коллекция без блока элементов".to_string(),
            examples: Vec::new(),
            availability: Vec::new(),
            since_version: "0".to_string(),
        },
        structure: TypeStructure {
            collection_element: None,
            methods: vec![("Получить".to_string(), "Get".to_string())],
            properties: Vec::new(),
            constructors: Vec::new(),
            iterable: true,
            indexable: true,
            enum_values: Vec::new(),
        },
        metadata: TypeMetadata {
            available_facets: Vec::new(),
            default_facet: None,
            serializable: false,
            exchangeable: false,
            xdto_namespace: None,
            xdto_type: None,
        },
    };

    let element_type_info = TypeInfo {
        identity: TypeIdentity {
            russian_name: "ЭлементТеста".to_string(),
            english_name: "TestItem".to_string(),
            catalog_path: "objects/x/TestItem.html".to_string(),
            aliases: Vec::new(),
            category_path: "Тесты".to_string(),
        },
        documentation: TypeDocumentation {
            category_description: None,
            type_description: String::new(),
            examples: Vec::new(),
            availability: Vec::new(),
            since_version: "0".to_string(),
        },
        structure: TypeStructure {
            collection_element: None,
            methods: Vec::new(),
            properties: Vec::new(),
            constructors: Vec::new(),
            iterable: false,
            indexable: false,
            enum_values: Vec::new(),
        },
        metadata: TypeMetadata {
            available_facets: Vec::new(),
            default_facet: None,
            serializable: false,
            exchangeable: false,
            xdto_namespace: None,
            xdto_type: None,
        },
    };

    let mut db = SyntaxHelperDatabase::default();
    db.nodes
        .insert("c".to_string(), SyntaxNode::Type(type_info));
    db.nodes
        .insert("i".to_string(), SyntaxNode::Type(element_type_info));
    db.methods.insert(
        "method_ТестКоллекция.Получить".to_string(),
        MethodInfo {
            name: "ТестКоллекция.Получить".to_string(),
            english_name: Some("Get".to_string()),
            description: None,
            overloads: Vec::new(),
            parameters: Vec::new(),
            return_type: Some("ЭлементТеста".to_string()),
            return_description: None,
            contexts: Vec::new(),
        },
    );

    let raw = convert_syntax_helper_to_raw(&db);
    let coll = raw
        .iter()
        .find(|t| t.name == "ТестКоллекция")
        .expect("RawTypeData for ТестКоллекция should be present");

    assert_eq!(coll.collection_item_type.as_deref(), Some("ЭлементТеста"));
}

#[test]
fn test_property_collection_item_type_is_exported_to_raw_property() {
    let type_info = TypeInfo {
        identity: TypeIdentity {
            russian_name: "ОбъектМетаданныхКонфигурация".to_string(),
            english_name: "ConfigurationMetadataObject".to_string(),
            catalog_path: "objects/x/ConfigurationMetadataObject.html".to_string(),
            aliases: Vec::new(),
            category_path: "Тесты".to_string(),
        },
        documentation: TypeDocumentation {
            category_description: None,
            type_description: String::new(),
            examples: Vec::new(),
            availability: Vec::new(),
            since_version: "0".to_string(),
        },
        structure: TypeStructure {
            collection_element: None,
            methods: Vec::new(),
            properties: vec![(
                "РегистрыНакопления".to_string(),
                "AccumulationRegisters".to_string(),
            )],
            constructors: Vec::new(),
            iterable: false,
            indexable: false,
            enum_values: Vec::new(),
        },
        metadata: TypeMetadata {
            available_facets: Vec::new(),
            default_facet: None,
            serializable: false,
            exchangeable: false,
            xdto_namespace: None,
            xdto_type: None,
        },
    };

    let mut db = SyntaxHelperDatabase::default();
    db.nodes
        .insert("metadata".to_string(), SyntaxNode::Type(type_info));
    db.properties.insert(
        "property_ОбъектМетаданныхКонфигурация.РегистрыНакопления".to_string(),
        PropertyInfo {
            name: "ОбъектМетаданныхКонфигурация.РегистрыНакопления".to_string(),
            english_name: Some("ConfigurationMetadataObject.AccumulationRegisters".to_string()),
            property_type: Some("КоллекцияОбъектовМетаданных".to_string()),
            is_readonly: true,
            description: None,
            contexts: Vec::new(),
            source_key: Some(
                "objects/catalog/ConfigurationMetadataObject/properties/AccumulationRegisters.html"
                    .to_string(),
            ),
            source_path: None,
            source_kind: PropertySourceKind::TypeProperty,
            collection_item_type: Some("ОбъектМетаданных: РегистрНакопления".to_string()),
        },
    );

    let raw = convert_syntax_helper_to_raw(&db);
    let metadata_type = raw
        .iter()
        .find(|type_data| type_data.name == "ОбъектМетаданныхКонфигурация")
        .expect("metadata configuration type should be converted");
    let property = metadata_type
        .properties
        .iter()
        .find(|property| property.name == "РегистрыНакопления")
        .expect("metadata collection property should be converted");

    assert_eq!(
        property.collection_item_type.as_deref(),
        Some("ОбъектМетаданных: РегистрНакопления")
    );
}

#[test]
fn test_convert_global_function_signatures_from_syntax_helper() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let html_path = root.join(
        "examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/methods/catalog20/NStr871.html",
    );

    let content = std::fs::read_to_string(&html_path).expect("failed to read NStr871.html");
    let document = Html::parse_document(&content);
    let parser = DocumentParser::new();
    let func_info = parser
        .parse_global_function(&html_path, &document)
        .expect("failed to parse global function info");

    let mut db = SyntaxHelperDatabase::default();
    db.global_functions.insert("k".to_string(), func_info);

    let signatures = convert_syntax_helper_global_functions(&db);
    let nstr_sig = signatures
        .iter()
        .find(|sig| sig.name == "НСтр")
        .expect("НСтр signature should exist");

    assert_eq!(nstr_sig.return_type.as_deref(), Some("Строка"));
    assert_eq!(
        nstr_sig.context_requirements,
        ContextRequirements::Universal
    );
    assert!(
        signatures.iter().any(|sig| sig.name == "NStr"),
        "English alias should be exported"
    );
}

#[test]
fn test_convert_global_context_index_from_provenance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let html_path = root.join(
        "examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Metadata974.html",
    );
    let loader = SyntaxHelperLoader::new();
    let node = loader
        .parse_html_file(&html_path)
        .expect("failed to parse Metadata974.html");
    loader.save_node(node);

    let db = loader.export_database();
    let index = convert_syntax_helper_global_context_index(&db);
    let metadata = index
        .get("Метаданные")
        .expect("Метаданные should be resolved from global-context index");
    let metadata_en = index
        .get("Metadata")
        .expect("Metadata should be resolved from global-context index");

    assert!(index.is_loaded());
    assert_eq!(index.len(), 1);
    assert_eq!(metadata.source_key, metadata_en.source_key);
    assert_eq!(
        metadata.prop_type.as_deref(),
        Some("ОбъектМетаданныхКонфигурация")
    );
    assert_eq!(
        index
            .get_by_source_key("objects/Global context/properties/Metadata974")
            .map(|property| property.normalized_key.as_str()),
        Some("метаданные")
    );
}

#[test]
fn test_convert_global_context_index_ignores_legacy_property_keys() {
    let mut db = SyntaxHelperDatabase::default();
    db.properties.insert(
        "property_Глобальный контекст.Фейк".to_string(),
        PropertyInfo {
            name: "Глобальный контекст.Фейк".to_string(),
            english_name: Some("Global context.Fake".to_string()),
            property_type: Some("Строка".to_string()),
            is_readonly: true,
            description: Some("legacy only".to_string()),
            contexts: Vec::new(),
            source_key: Some("objects/Global context/properties/Fake".to_string()),
            source_path: None,
            source_kind: PropertySourceKind::TypeProperty,
            collection_item_type: None,
        },
    );

    let index = convert_syntax_helper_global_context_index(&db);

    assert!(index.is_loaded());
    assert!(index.is_empty());
    assert!(
        index.get("Фейк").is_none(),
        "legacy property_<name> keys must not populate GlobalContextIndex"
    );
}

#[test]
fn test_convert_global_context_index_accepts_synthetic_property() {
    let mut db = SyntaxHelperDatabase::default();
    db.global_context_properties.insert(
        "synthetic/global-context/NewGlobal".to_string(),
        PropertyInfo {
            name: "Глобальный контекст.НовыйГлобал".to_string(),
            english_name: Some("Global context.NewGlobal".to_string()),
            property_type: Some("Строка".to_string()),
            is_readonly: true,
            description: Some("Synthetic global context property".to_string()),
            contexts: vec!["Сервер".to_string()],
            source_key: Some("synthetic/global-context/NewGlobal".to_string()),
            source_path: None,
            source_kind: PropertySourceKind::GlobalContextProperty,
            collection_item_type: None,
        },
    );

    let index = convert_syntax_helper_global_context_index(&db);

    assert_eq!(
        index
            .get("НовыйГлобал")
            .and_then(|property| property.prop_type.as_deref()),
        Some("Строка")
    );
    assert_eq!(
        index
            .get("NewGlobal")
            .map(|property| property.source_key.as_str()),
        Some("synthetic/global-context/NewGlobal")
    );
}

#[test]
fn test_convert_platform_docs_semantic_bundle_contains_all_surfaces() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let parser = DocumentParser::new();
    let mut db = SyntaxHelperDatabase::default();

    let type_path = root.join(
        "examples/syntax_helper/rebuilt.shcntx_ru/objects/catalog1649/catalog1614/FormDataCollection.html",
    );
    let type_document = Html::parse_document(
        &std::fs::read_to_string(&type_path).expect("failed to read FormDataCollection.html"),
    );
    db.nodes.insert(
        "form-data-collection".to_string(),
        SyntaxNode::Type(
            parser
                .parse_type(&type_path, &type_document)
                .expect("failed to parse FormDataCollection type"),
        ),
    );

    let function_path = root.join(
        "examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/methods/catalog20/NStr871.html",
    );
    let function_document = Html::parse_document(
        &std::fs::read_to_string(&function_path).expect("failed to read NStr871.html"),
    );
    db.global_functions.insert(
        "nstr".to_string(),
        parser
            .parse_global_function(&function_path, &function_document)
            .expect("failed to parse NStr global function"),
    );

    let metadata_path = root.join(
        "examples/syntax_helper/rebuilt.shcntx_ru/objects/Global context/properties/Metadata974.html",
    );
    let metadata_document = Html::parse_document(
        &std::fs::read_to_string(&metadata_path).expect("failed to read Metadata974.html"),
    );
    let metadata_property = parser
        .parse_property(&metadata_path, &metadata_document)
        .expect("failed to parse Metadata property");
    db.global_context_properties.insert(
        metadata_property
            .source_key
            .clone()
            .expect("Metadata property should have source key"),
        metadata_property,
    );

    let bundle = convert_syntax_helper_to_semantic_bundle(&db);

    assert_eq!(
        bundle.schema_version,
        PLATFORM_DOCS_SEMANTIC_BUNDLE_SCHEMA_VERSION
    );
    assert!(bundle
        .raw_types
        .iter()
        .any(|raw| raw.name == "ДанныеФормыКоллекция"));
    assert!(bundle
        .global_function_signatures
        .iter()
        .any(|signature| signature.name == "НСтр"));
    assert_eq!(
        bundle
            .global_context_index
            .get("Metadata")
            .and_then(|property| property.prop_type.as_deref()),
        Some("ОбъектМетаданныхКонфигурация")
    );
}

#[test]
fn test_convert_global_function_docs() {
    let mut db = SyntaxHelperDatabase::default();
    db.global_functions.insert(
        "k".to_string(),
        GlobalFunctionInfo {
            name: "ТестФункция".to_string(),
            english_name: Some("TestFunction".to_string()),
            description: Some("Описание функции".to_string()),
            parameters: Vec::new(),
            return_type: Some("Строка".to_string()),
            return_description: Some("Описание возврата".to_string()),
            polymorphic: false,
            pure: false,
            contexts: Vec::new(),
            category: None,
        },
    );

    let signatures = convert_syntax_helper_global_functions(&db);
    let ru_sig = signatures
        .iter()
        .find(|sig| sig.name == "ТестФункция")
        .expect("Русская сигнатура должна быть");
    let en_sig = signatures
        .iter()
        .find(|sig| sig.name == "TestFunction")
        .expect("Английская сигнатура должна быть");

    assert_eq!(ru_sig.description.as_deref(), Some("Описание функции"));
    assert_eq!(
        ru_sig.return_description.as_deref(),
        Some("Описание возврата")
    );
    assert_eq!(en_sig.description.as_deref(), Some("Описание функции"));
    assert_eq!(
        en_sig.return_description.as_deref(),
        Some("Описание возврата")
    );
}
