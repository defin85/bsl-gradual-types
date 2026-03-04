use super::super::types::AttributeInfo;
use super::*;
use bsl_shared::domain::types::MetadataKind;

#[test]
fn test_convert_catalog_to_raw_type() {
    let mut obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Контрагенты".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.synonym = Some("Контрагенты".to_string());
    obj.attributes.push(AttributeInfo {
        name: "ИНН".to_string(),
        type_name: "Строка".to_string(),
        synonym: Some("ИНН".to_string()),
    });

    let raw_type = obj.to_raw_type_data(None);

    assert_eq!(raw_type.name, "Справочники.Контрагенты");
    assert_eq!(raw_type.kind, Some(MetadataKind::Catalog));
    assert_eq!(raw_type.attributes.len(), 1);
    assert_eq!(raw_type.attributes[0].name, "ИНН");
    assert_eq!(raw_type.facets.len(), 5); // Manager, Object, Reference, Selection, List
}

#[test]
fn test_convert_chart_of_accounts_predefined_items_to_manager_marker_properties() {
    let mut obj = UniversalMetadataObject::new(
        "ChartOfAccounts".to_string(),
        "Хозрасчетный".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.predefined_items = vec![
        "ГотоваяПродукция".to_string(),
        "Товары".to_string(),
        "ГотоваяПродукция".to_string(), // duplicate
    ];

    let raw_type = obj.to_raw_type_data(None);
    let predefined_props: Vec<_> = raw_type
        .properties
        .iter()
        .filter(|p| p.prop_type.starts_with(PREDEFINED_MANAGER_PROP_TYPE_PREFIX))
        .collect();

    assert_eq!(predefined_props.len(), 2);
    assert!(predefined_props
        .iter()
        .any(|p| p.name == "ГотоваяПродукция"));
    assert!(predefined_props.iter().any(|p| p.name == "Товары"));
    assert!(predefined_props.iter().all(|p| p.is_readonly));
    assert!(predefined_props.iter().all(|p| {
        p.prop_type == format!("{PREDEFINED_MANAGER_PROP_TYPE_PREFIX}ПланСчетовСсылка.Хозрасчетный")
    }));
}

#[test]
fn test_convert_document_standard_attributes_to_properties() {
    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док1".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.standard_attributes = vec![
        "Ref".to_string(),
        "DeletionMark".to_string(),
        "Date".to_string(),
        "Number".to_string(),
        "Posted".to_string(),
    ];
    obj.properties
        .insert("NumberType".to_string(), "Number".to_string());
    obj.properties
        .insert("Posting".to_string(), "Allow".to_string());

    let raw_type = obj.to_raw_type_data(None);

    let link = raw_type
        .properties
        .iter()
        .find(|prop| prop.name == "Ссылка")
        .expect("missing Ссылка");
    assert_eq!(link.prop_type, "ДокументСсылка.Док1");
    assert!(link.is_readonly);

    let deletion_mark = raw_type
        .properties
        .iter()
        .find(|prop| prop.name == "ПометкаУдаления")
        .expect("missing ПометкаУдаления");
    assert_eq!(deletion_mark.prop_type, "Булево");
    assert!(deletion_mark.is_readonly);

    let date = raw_type
        .properties
        .iter()
        .find(|prop| prop.name == "Дата")
        .expect("missing Дата");
    assert_eq!(date.prop_type, "Дата");

    let number = raw_type
        .properties
        .iter()
        .find(|prop| prop.name == "Номер")
        .expect("missing Номер");
    assert_eq!(number.prop_type, "Число");

    let posted = raw_type
        .properties
        .iter()
        .find(|prop| prop.name == "Проведен")
        .expect("missing Проведен");
    assert_eq!(posted.prop_type, "Булево");
}

#[test]
fn test_convert_document_posted_property_only_for_posting_capable_documents() {
    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док2".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.standard_attributes = vec!["Posted".to_string()];
    obj.properties
        .insert("Posting".to_string(), "Deny".to_string());

    let raw_type = obj.to_raw_type_data(None);
    assert!(
        raw_type
            .properties
            .iter()
            .all(|prop| prop.name != "Проведен"),
        "Проведен must be absent for non-posting documents"
    );
}

#[test]
fn test_converter_dedup_keeps_existing_repository_property_on_standard_attr_conflict() {
    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док3".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.attributes.push(AttributeInfo {
        name: "Ссылка".to_string(),
        type_name: "ПроизвольныйТип".to_string(),
        synonym: None,
    });
    obj.standard_attributes = vec!["Ref".to_string()];

    let raw_type = obj.to_raw_type_data(None);
    let links: Vec<_> = raw_type
        .properties
        .iter()
        .filter(|prop| prop.name == "Ссылка")
        .collect();

    assert_eq!(links.len(), 1, "Ссылка must be deduplicated");
    assert_eq!(links[0].prop_type, "ПроизвольныйТип");
}

#[test]
fn test_convert_unknown_type() {
    let obj = UniversalMetadataObject::new(
        "UnknownType".to_string(),
        "СтранныйОбъект".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);

    assert_eq!(raw_type.name, "СтранныйОбъект");
    assert_eq!(raw_type.kind, None);
    assert_eq!(raw_type.facets.len(), 0); // Пустой список фасетов
}

#[test]
fn test_convert_with_extension_prefix() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Контрагенты".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    // Без префикса (основная конфигурация)
    let raw_type_no_prefix = obj.to_raw_type_data(None);
    assert_eq!(raw_type_no_prefix.name, "Справочники.Контрагенты");
    assert_eq!(raw_type_no_prefix.english_name, "Контрагенты");

    // С префиксом (расширение)
    let raw_type_with_prefix = obj.to_raw_type_data(Some("Тест_"));
    assert_eq!(raw_type_with_prefix.name, "Справочники.Тест_Контрагенты");
    assert_eq!(raw_type_with_prefix.english_name, "Тест_Контрагенты");

    // Пустой префикс (эквивалент None)
    let raw_type_empty_prefix = obj.to_raw_type_data(Some(""));
    assert_eq!(raw_type_empty_prefix.name, "Справочники.Контрагенты");
}

// ============================================================================
// Milestone 3.14: Module Paths Integration
// ============================================================================

#[test]
fn test_convert_with_module_paths() {
    use std::path::PathBuf;

    let mut obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Контрагенты".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.object_module_path = Some(PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl"));
    obj.manager_module_path = Some(PathBuf::from("Catalogs/Контрагенты/Ext/ManagerModule.bsl"));

    let raw_type = obj.to_raw_type_data(None);

    assert!(raw_type.module_paths.is_some());
    let module_paths = raw_type.module_paths.unwrap();
    assert!(module_paths.object_module.is_some());
    assert!(module_paths.manager_module.is_some());
    assert!(module_paths.recordset_module.is_none());

    assert!(module_paths
        .object_module
        .unwrap()
        .to_string_lossy()
        .contains("ObjectModule.bsl"));
    assert!(module_paths
        .manager_module
        .unwrap()
        .to_string_lossy()
        .contains("ManagerModule.bsl"));
}

#[test]
fn test_convert_without_module_paths() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Товары".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);

    // Без путей к модулям - module_paths должен быть None
    assert!(raw_type.module_paths.is_none());
}

#[test]
fn test_convert_register_with_recordset_module() {
    use std::path::PathBuf;

    let mut obj = UniversalMetadataObject::new(
        "InformationRegister".to_string(),
        "КурсыВалют".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.manager_module_path = Some(PathBuf::from(
        "InformationRegisters/КурсыВалют/Ext/ManagerModule.bsl",
    ));
    obj.record_set_module_path = Some(PathBuf::from(
        "InformationRegisters/КурсыВалют/Ext/RecordSetModule.bsl",
    ));

    let raw_type = obj.to_raw_type_data(None);

    assert!(raw_type.module_paths.is_some());
    let module_paths = raw_type.module_paths.unwrap();
    assert!(module_paths.object_module.is_none());
    assert!(module_paths.manager_module.is_some());
    assert!(module_paths.recordset_module.is_some());

    assert!(module_paths
        .recordset_module
        .unwrap()
        .to_string_lossy()
        .contains("RecordSetModule.bsl"));
}

#[test]
fn test_convert_only_one_module_path() {
    use std::path::PathBuf;

    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "ЗаказПокупателя".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    // Только ObjectModule, без ManagerModule
    obj.object_module_path = Some(PathBuf::from(
        "Documents/ЗаказПокупателя/Ext/ObjectModule.bsl",
    ));

    let raw_type = obj.to_raw_type_data(None);

    assert!(raw_type.module_paths.is_some());
    let module_paths = raw_type.module_paths.unwrap();
    assert!(module_paths.object_module.is_some());
    assert!(module_paths.manager_module.is_none());
    assert!(module_paths.recordset_module.is_none());
}

#[test]
fn test_form_type_includes_form_xml_attributes() {
    use super::super::form_types::{FormAttribute, FormMetadata, TypeDescription};

    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док1".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    obj.forms.push(FormMetadata {
        name: "Форма1".to_string(),
        owner_type: "Document.Док1".to_string(),
        attributes: vec![FormAttribute {
            name: "СчетФактура".to_string(),
            id: 1,
            type_description: TypeDescription {
                types: vec!["cfg:DocumentRef.СчетФактураВыданный".to_string()],
            },
            is_main_attribute: false,
            saved_data: false,
        }],
        events: Vec::new(),
        module_path: None,
        execution_contexts: Vec::new(),
        elements: Vec::new(),
    });

    let raw_types = obj.to_raw_type_data_with_forms(None);
    let form_type = raw_types
        .iter()
        .find(|t| t.name == "Формы.Документы.Док1.Форма1")
        .expect("expected form type");

    let attr = form_type
        .properties
        .iter()
        .find(|p| p.name == "СчетФактура")
        .expect("expected form attribute as property");

    assert_eq!(attr.prop_type, "cfg:DocumentRef.СчетФактураВыданный");
}

#[test]
fn test_form_elements_container_includes_usual_group() {
    use super::super::form_types::{
        FormAttribute, FormElementBinding, FormMetadata, TypeDescription,
    };

    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док1".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    obj.forms.push(FormMetadata {
        name: "Форма1".to_string(),
        owner_type: "Document.Док1".to_string(),
        attributes: vec![FormAttribute {
            name: "Объект".to_string(),
            id: 1,
            type_description: TypeDescription {
                types: vec!["cfg:DocumentObject.Док1".to_string()],
            },
            is_main_attribute: true,
            saved_data: true,
        }],
        events: Vec::new(),
        module_path: None,
        execution_contexts: Vec::new(),
        elements: vec![FormElementBinding {
            kind: "UsualGroup".to_string(),
            name: Some("СчетФактураПросмотр".to_string()),
            id: Some(10),
            data_path: None,
            children: Vec::new(),
        }],
    });

    let raw_types = obj.to_raw_type_data_with_forms(None);
    let elements_type = raw_types
        .iter()
        .find(|t| t.name == "ЭлементыФормы.Документы.Док1.Форма1")
        .expect("expected elements container type");

    let group = elements_type
        .properties
        .iter()
        .find(|p| p.name == "СчетФактураПросмотр")
        .expect("expected group element property");

    assert_eq!(group.prop_type, "ГруппаФормы");
}
