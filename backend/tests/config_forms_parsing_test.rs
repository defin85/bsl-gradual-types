//! Тесты парсинга форм конфигурации 1С (Forms)
//!
//! Проверяет корректность извлечения информации из Form.xml:
//! - Реквизиты формы (Attributes)
//! - События формы (Events)
//! - Контексты выполнения модуля формы
//! - Интеграция с discovery

use bsl_backend::data::loaders::config_metadata_parser::{
    ConfigurationDiscovery, FormParser,
};
use std::path::PathBuf;

/// Путь к тестовой конфигурации с реальными примерами форм
fn test_config_path() -> PathBuf {
    // Тесты запускаются из backend/, поднимаемся на уровень workspace
    let backend_root = std::env::current_dir().expect("Failed to get current dir");
    let workspace_root = backend_root
        .parent()
        .expect("Failed to get workspace root");
    workspace_root.join("examples").join("conf").join("conf_test")
}

#[test]
fn test_parse_document_form_from_real_example() {
    let form_xml = test_config_path()
        .join("Documents")
        .join("ЗаказНаряды")
        .join("Forms")
        .join("ФормаДокумента")
        .join("Ext")
        .join("Form.xml");

    assert!(
        form_xml.exists(),
        "Form.xml should exist at {:?}",
        form_xml
    );

    let form = FormParser::parse_form_xml(&form_xml, "Document.ЗаказНаряды", "ФормаДокумента")
        .expect("Failed to parse form");

    assert_eq!(form.name, "ФормаДокумента");
    assert_eq!(form.owner_type, "Document.ЗаказНаряды");
    assert!(!form.attributes.is_empty(), "Form should have attributes");

    // Проверка основного реквизита "Объект"
    let main_attr = form
        .attributes
        .iter()
        .find(|a| a.is_main_attribute)
        .expect("Form should have a main attribute");

    assert_eq!(main_attr.name, "Объект");
    assert!(main_attr.saved_data, "Main attribute should have SavedData=true");
    assert!(
        !main_attr.type_description.types.is_empty(),
        "Main attribute should have types"
    );

    // Проверяем, что тип содержит ссылку на DocumentObject
    assert!(
        main_attr.type_description.types.iter().any(|t| t.contains("DocumentObject.ЗаказНаряды")),
        "Main attribute should have DocumentObject type, got: {:?}",
        main_attr.type_description.types
    );
}

#[test]
fn test_parse_form_events() {
    let form_xml = test_config_path()
        .join("Documents")
        .join("ЗаказНаряды")
        .join("Forms")
        .join("ФормаДокумента")
        .join("Ext")
        .join("Form.xml");

    let form = FormParser::parse_form_xml(&form_xml, "Document.ЗаказНаряды", "ФормаДокумента")
        .expect("Failed to parse form");

    assert!(!form.events.is_empty(), "Form should have events");

    // Проверка события OnCreateAtServer
    let on_create_event = form
        .events
        .iter()
        .find(|e| e.name == "OnCreateAtServer")
        .expect("Form should have OnCreateAtServer event");

    assert_eq!(
        on_create_event.handler_name, "ПриСозданииНаСервере",
        "OnCreateAtServer event handler should be 'ПриСозданииНаСервере'"
    );
}

#[test]
fn test_parse_form_module_contexts() {
    let form_xml = test_config_path()
        .join("Documents")
        .join("ЗаказНаряды")
        .join("Forms")
        .join("ФормаДокумента")
        .join("Ext")
        .join("Form.xml");

    let form = FormParser::parse_form_xml(&form_xml, "Document.ЗаказНаряды", "ФормаДокумента")
        .expect("Failed to parse form");

    assert!(
        form.module_path.is_some(),
        "Form should have module_path"
    );

    // Проверяем, что Module.bsl существует
    let module_path = form.module_path.unwrap();
    assert!(
        module_path.exists(),
        "Module.bsl should exist at {:?}",
        module_path
    );

    // Проверяем контексты выполнения
    assert!(
        !form.execution_contexts.is_empty(),
        "Form module should have execution contexts"
    );

    // В реальном примере Module.bsl содержит &НаСервере
    use bsl_backend::data::loaders::config_metadata_parser::ExecutionContext;
    assert!(
        form.execution_contexts.contains(&ExecutionContext::Server),
        "Form module should have Server context, got: {:?}",
        form.execution_contexts
    );
}

#[test]
fn test_discover_all_forms_for_document() {
    let discovery = ConfigurationDiscovery::new(test_config_path(), false);
    let forms = discovery
        .discover_forms("Documents", "ЗаказНаряды")
        .expect("Failed to discover forms");

    assert_eq!(forms.len(), 1, "Should discover 1 form for Document.ЗаказНаряды");
    assert_eq!(forms[0].name, "ФормаДокумента");
    assert_eq!(forms[0].owner_type, "Document.ЗаказНаряды");
}

#[test]
fn test_discover_forms_no_forms_directory() {
    let discovery = ConfigurationDiscovery::new(test_config_path(), false);

    // Константы не имеют форм
    let forms = discovery
        .discover_forms("Constants", "НекаяКонстанта")
        .expect("Should not fail for missing Forms directory");

    assert!(
        forms.is_empty(),
        "Should return empty vector for non-existent Forms directory"
    );
}

#[test]
fn test_form_attributes_all_parsed() {
    let form_xml = test_config_path()
        .join("Documents")
        .join("ЗаказНаряды")
        .join("Forms")
        .join("ФормаДокумента")
        .join("Ext")
        .join("Form.xml");

    let form = FormParser::parse_form_xml(&form_xml, "Document.ЗаказНаряды", "ФормаДокумента")
        .expect("Failed to parse form");

    // В реальном Form.xml есть только один атрибут "Объект"
    assert_eq!(
        form.attributes.len(),
        1,
        "Form should have exactly 1 attribute"
    );

    let attr = &form.attributes[0];
    assert_eq!(attr.id, 1, "Attribute should have id=1");
    assert!(attr.is_main_attribute, "Attribute should be main");
    assert!(attr.saved_data, "Attribute should have SavedData=true");
}

#[test]
fn test_form_metadata_integration_with_discovery() {
    let discovery = ConfigurationDiscovery::new(test_config_path(), false);

    // Обнаруживаем все конфигурации
    let configurations = discovery
        .discover_all_configurations()
        .expect("Failed to discover configurations");

    assert!(!configurations.is_empty(), "Should find at least one configuration");

    // Парсим метаданные первой конфигурации
    let first_config = &configurations[0];
    let metadata = discovery
        .discover_metadata_in_configuration(first_config, None::<fn(_)>)
        .expect("Failed to discover metadata");

    // Ищем документ ЗаказНаряды
    let doc = metadata
        .iter()
        .find(|m| m.name == "ЗаказНаряды" && m.object_type_raw == "Document")
        .expect("Should find Document.ЗаказНаряды");

    // Проверяем, что формы распарсены
    assert_eq!(
        doc.forms.len(),
        1,
        "Document should have 1 form parsed"
    );

    let form = &doc.forms[0];
    assert_eq!(form.name, "ФормаДокумента");
    assert!(!form.attributes.is_empty(), "Form should have attributes");
    assert!(!form.events.is_empty(), "Form should have events");
}

#[test]
fn test_form_parser_graceful_degradation_no_module() {
    use tempfile::TempDir;
    use std::fs;

    // Создаём временную директорию с Form.xml БЕЗ Module.bsl
    let temp_dir = TempDir::new().unwrap();
    let form_dir = temp_dir.path().join("Ext");
    fs::create_dir_all(&form_dir).unwrap();

    let form_xml_path = form_dir.join("Form.xml");
    let form_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<Form xmlns="http://v8.1c.ru/8.3/xcf/logform">
    <Attributes>
        <Attribute name="TestAttr" id="1">
            <Type>
                <v8:Type>String</v8:Type>
            </Type>
            <MainAttribute>false</MainAttribute>
            <SavedData>false</SavedData>
        </Attribute>
    </Attributes>
    <Events>
        <Event name="OnOpen">ПриОткрытии</Event>
    </Events>
</Form>"#;

    fs::write(&form_xml_path, form_xml_content).unwrap();

    // Парсим форму
    let form = FormParser::parse_form_xml(&form_xml_path, "TestObject.Test", "TestForm")
        .expect("Should parse form without module");

    assert_eq!(form.name, "TestForm");
    assert_eq!(form.attributes.len(), 1);
    assert_eq!(form.events.len(), 1);
    assert!(
        form.module_path.is_none() || !form.module_path.as_ref().unwrap().exists(),
        "Module path should not exist"
    );
    assert!(
        form.execution_contexts.is_empty(),
        "Should have no execution contexts without module"
    );
}

#[test]
fn test_form_with_multiple_types() {
    use tempfile::TempDir;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let form_dir = temp_dir.path().join("Ext");
    fs::create_dir_all(&form_dir).unwrap();

    let form_xml_path = form_dir.join("Form.xml");
    let form_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<Form>
    <Attributes>
        <Attribute name="MultiTypeAttr" id="2">
            <Type>
                <v8:Type>String</v8:Type>
                <v8:Type>Number</v8:Type>
                <v8:Type>Boolean</v8:Type>
                <v8:Length>100</v8:Length>
            </Type>
            <MainAttribute>false</MainAttribute>
            <SavedData>true</SavedData>
        </Attribute>
    </Attributes>
</Form>"#;

    fs::write(&form_xml_path, form_xml_content).unwrap();

    let form = FormParser::parse_form_xml(&form_xml_path, "Test.Test", "TestForm")
        .expect("Should parse form with multiple types");

    assert_eq!(form.attributes.len(), 1);
    let attr = &form.attributes[0];
    assert_eq!(attr.name, "MultiTypeAttr");
    assert_eq!(attr.id, 2);

    // Проверяем, что все типы распарсены
    assert_eq!(
        attr.type_description.types.len(),
        3,
        "Should have 3 types (String, Number, Boolean)"
    );
    assert!(attr.type_description.types.contains(&"String".to_string()));
    assert!(attr.type_description.types.contains(&"Number".to_string()));
    assert!(attr.type_description.types.contains(&"Boolean".to_string()));
}
