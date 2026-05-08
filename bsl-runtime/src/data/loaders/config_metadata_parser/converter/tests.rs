use super::super::types::{AttributeInfo, TabularSectionInfo};
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
fn test_convert_preserves_metadata_xml_path() {
    use std::path::PathBuf;

    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док1".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.metadata_xml_path = Some(PathBuf::from("Documents/Док1.xml"));

    let raw_type = obj.to_raw_type_data(None);

    assert_eq!(
        raw_type.metadata_path.as_deref(),
        Some(PathBuf::from("Documents/Док1.xml").as_path())
    );
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
fn test_convert_metadata_attributes_normalize_cfg_and_xs_types() {
    use super::super::form_types::FormMetadata;

    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док1".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.attributes = vec![
        AttributeInfo {
            name: "Контрагент".to_string(),
            type_name: "cfg:CatalogRef.Контрагенты".to_string(),
            synonym: None,
        },
        AttributeInfo {
            name: "Основание".to_string(),
            type_name: "cfg:DocumentRef.ЗаказПокупателя".to_string(),
            synonym: None,
        },
        AttributeInfo {
            name: "ВидОперации".to_string(),
            type_name: "cfg:EnumRef.ВидыОпераций".to_string(),
            synonym: None,
        },
        AttributeInfo {
            name: "Комментарий".to_string(),
            type_name: "xs:string".to_string(),
            synonym: None,
        },
    ];
    obj.tabular_sections.push(TabularSectionInfo {
        name: "Товары".to_string(),
        synonym: None,
        attributes: vec![
            AttributeInfo {
                name: "Номенклатура".to_string(),
                type_name: "cfg:CatalogRef.Номенклатура".to_string(),
                synonym: None,
            },
            AttributeInfo {
                name: "Количество".to_string(),
                type_name: "xs:decimal".to_string(),
                synonym: None,
            },
        ],
    });
    obj.forms.push(FormMetadata {
        name: "Форма1".to_string(),
        owner_type: "Document.Док1".to_string(),
        attributes: Vec::new(),
        events: Vec::new(),
        module_path: None,
        execution_contexts: Vec::new(),
        elements: Vec::new(),
    });

    let raw_type = obj.to_raw_type_data(None);
    let prop_type = |name: &str| {
        raw_type
            .properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.prop_type.as_str())
            .expect("expected metadata attribute as property")
    };
    assert_eq!(prop_type("Контрагент"), "СправочникСсылка.Контрагенты");
    assert_eq!(prop_type("Основание"), "ДокументСсылка.ЗаказПокупателя");
    assert_eq!(prop_type("ВидОперации"), "ПеречислениеСсылка.ВидыОпераций");
    assert_eq!(prop_type("Комментарий"), "Строка");

    let attr_type = |name: &str| {
        raw_type
            .attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.attr_type.as_str())
            .expect("expected raw attribute")
    };
    assert_eq!(attr_type("Контрагент"), "СправочникСсылка.Контрагенты");
    assert_eq!(attr_type("Основание"), "ДокументСсылка.ЗаказПокупателя");

    let ts = raw_type
        .tabular_sections
        .iter()
        .find(|ts| ts.name == "Товары")
        .expect("expected tabular section");
    let ts_attr_type = |name: &str| {
        ts.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.attr_type.as_str())
            .expect("expected tabular section attribute")
    };
    assert_eq!(
        ts_attr_type("Номенклатура"),
        "СправочникСсылка.Номенклатура"
    );
    assert_eq!(ts_attr_type("Количество"), "Число");

    let raw_types = obj.to_raw_type_data_with_forms(None);
    let row_type = raw_types
        .iter()
        .find(|t| t.name == "СтрокаТовары")
        .expect("expected tabular row type");
    let row_prop_type = |name: &str| {
        row_type
            .properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.prop_type.as_str())
            .expect("expected row property")
    };
    assert_eq!(
        row_prop_type("Номенклатура"),
        "СправочникСсылка.Номенклатура"
    );
    assert_eq!(row_prop_type("Количество"), "Число");
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
        attributes: vec![
            FormAttribute {
                name: "СчетФактура".to_string(),
                id: 1,
                type_description: TypeDescription {
                    types: vec!["cfg:DocumentRef.СчетФактураВыданный".to_string()],
                },
                is_main_attribute: false,
                saved_data: false,
            },
            FormAttribute {
                name: "Контрагент".to_string(),
                id: 2,
                type_description: TypeDescription {
                    types: vec!["cfg:CatalogRef.Контрагенты".to_string()],
                },
                is_main_attribute: false,
                saved_data: false,
            },
            FormAttribute {
                name: "ВидОперации".to_string(),
                id: 3,
                type_description: TypeDescription {
                    types: vec!["cfg:EnumRef.ВидыОпераций".to_string()],
                },
                is_main_attribute: false,
                saved_data: false,
            },
        ],
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

    let prop_type = |name: &str| {
        form_type
            .properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.prop_type.as_str())
            .expect("expected form attribute as property")
    };

    assert_eq!(
        prop_type("СчетФактура"),
        "ДокументСсылка.СчетФактураВыданный"
    );
    assert_eq!(prop_type("Контрагент"), "СправочникСсылка.Контрагенты");
    assert_eq!(prop_type("ВидОперации"), "ПеречислениеСсылка.ВидыОпераций");
}

#[test]
fn test_form_type_normalizes_v8_form_attribute_types() {
    use super::super::form_types::{FormAttribute, FormMetadata, TypeDescription};

    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док1".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    obj.forms.push(FormMetadata {
        name: "Форма1".to_string(),
        owner_type: "Document.Док1".to_string(),
        attributes: vec![
            FormAttribute {
                name: "СписокВидовОпераций".to_string(),
                id: 1,
                type_description: TypeDescription {
                    types: vec!["v8:ValueListType".to_string()],
                },
                is_main_attribute: false,
                saved_data: false,
            },
            FormAttribute {
                name: "ТаблицаПодбора".to_string(),
                id: 2,
                type_description: TypeDescription {
                    types: vec!["v8:ValueTable".to_string()],
                },
                is_main_attribute: false,
                saved_data: false,
            },
            FormAttribute {
                name: "ДеревоСтраниц".to_string(),
                id: 3,
                type_description: TypeDescription {
                    types: vec!["v8:ValueTree".to_string()],
                },
                is_main_attribute: false,
                saved_data: false,
            },
        ],
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

    let prop_type = |name: &str| {
        form_type
            .properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.prop_type.as_str())
            .expect("expected form attribute as property")
    };

    assert_eq!(prop_type("СписокВидовОпераций"), "СписокЗначений");
    assert_eq!(prop_type("ТаблицаПодбора"), "ТаблицаЗначений");
    assert_eq!(prop_type("ДеревоСтраниц"), "ДеревоЗначений");
}

#[test]
fn test_form_type_keeps_empty_type_form_attributes_as_dynamic() {
    use super::super::form_types::{FormAttribute, FormMetadata, TypeDescription};

    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Док1".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    obj.forms.push(FormMetadata {
        name: "Форма1".to_string(),
        owner_type: "Document.Док1".to_string(),
        attributes: vec![
            FormAttribute {
                name: "ЗначенияЗаполнения".to_string(),
                id: 1,
                type_description: TypeDescription { types: Vec::new() },
                is_main_attribute: false,
                saved_data: false,
            },
            FormAttribute {
                name: "ЗначениеКопирования".to_string(),
                id: 2,
                type_description: TypeDescription {
                    types: vec![" ".to_string()],
                },
                is_main_attribute: false,
                saved_data: false,
            },
        ],
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

    let prop_type = |name: &str| {
        form_type
            .properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.prop_type.as_str())
            .expect("expected form attribute as property")
    };

    assert_eq!(prop_type("ЗначенияЗаполнения"), "Dynamic");
    assert_eq!(prop_type("ЗначениеКопирования"), "Dynamic");
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
