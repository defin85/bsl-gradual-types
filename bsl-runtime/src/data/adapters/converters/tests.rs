use super::*;
use crate::data::loaders::syntax_helper::types::{
    GlobalFunctionInfo, MethodInfo, TypeDocumentation, TypeIdentity, TypeInfo, TypeMetadata,
    TypeStructure,
};
use crate::data::loaders::syntax_helper::{DocumentParser, SyntaxHelperDatabase, SyntaxNode};
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
