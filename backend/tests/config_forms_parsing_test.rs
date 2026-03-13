//! Тесты парсинга форм конфигурации 1С (Forms)
//!
//! Проверяет корректность извлечения информации из Form.xml:
//! - Реквизиты формы (Attributes)
//! - События формы (Events)
//! - Контексты выполнения модуля формы
//! - Интеграция с discovery

use bsl_backend::data::loaders::config_metadata_parser::{ConfigurationDiscovery, FormParser};
use std::path::PathBuf;

/// Путь к тестовой конфигурации с реальными примерами форм
fn test_config_path() -> PathBuf {
    // Тесты запускаются из backend/, поднимаемся на уровень workspace
    let backend_root = std::env::current_dir().expect("Failed to get current dir");
    let workspace_root = backend_root.parent().expect("Failed to get workspace root");
    workspace_root
        .join("examples")
        .join("conf")
        .join("conf_test")
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

    assert!(form_xml.exists(), "Form.xml should exist at {:?}", form_xml);

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
    assert!(
        main_attr.saved_data,
        "Main attribute should have SavedData=true"
    );
    assert!(
        !main_attr.type_description.types.is_empty(),
        "Main attribute should have types"
    );

    // Проверяем, что тип содержит ссылку на DocumentObject
    assert!(
        main_attr
            .type_description
            .types
            .iter()
            .any(|t| t.contains("DocumentObject.ЗаказНаряды")),
        "Main attribute should have DocumentObject type, got: {:?}",
        main_attr.type_description.types
    );

    // Проверяем, что UI элементы распарсены (ChildItems)
    assert!(
        !form.elements.is_empty(),
        "Form should have UI elements parsed from ChildItems"
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

    assert!(form.module_path.is_some(), "Form should have module_path");

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
        .discover_forms_for_object("Documents", "Document", "ЗаказНаряды")
        .expect("Failed to discover forms");

    assert_eq!(
        forms.len(),
        1,
        "Should discover 1 form for Document.ЗаказНаряды"
    );
    assert_eq!(forms[0].name, "ФормаДокумента");
    assert_eq!(forms[0].owner_type, "Document.ЗаказНаряды");
}

#[test]
fn test_discover_forms_no_forms_directory() {
    let discovery = ConfigurationDiscovery::new(test_config_path(), false);

    // Константы не имеют форм
    let forms = discovery
        .discover_forms_for_object("Constants", "Constant", "НекаяКонстанта")
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

    assert!(
        !configurations.is_empty(),
        "Should find at least one configuration"
    );

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
    assert_eq!(doc.forms.len(), 1, "Document should have 1 form parsed");

    let form = &doc.forms[0];
    assert_eq!(form.name, "ФормаДокумента");
    assert!(!form.attributes.is_empty(), "Form should have attributes");
    assert!(!form.events.is_empty(), "Form should have events");
    assert!(!form.elements.is_empty(), "Form should have elements");
}

#[test]
fn test_parse_form_tables_and_bindings_from_real_example() {
    fn flatten<'a>(
        acc: &mut Vec<&'a bsl_backend::data::loaders::config_metadata_parser::FormElementBinding>,
        nodes: &'a [bsl_backend::data::loaders::config_metadata_parser::FormElementBinding],
    ) {
        for n in nodes {
            acc.push(n);
            flatten(acc, &n.children);
        }
    }

    let form_xml = test_config_path()
        .join("Documents")
        .join("ЗаказНаряды")
        .join("Forms")
        .join("ФормаДокумента")
        .join("Ext")
        .join("Form.xml");

    let form = FormParser::parse_form_xml(&form_xml, "Document.ЗаказНаряды", "ФормаДокумента")
        .expect("Failed to parse form");

    let mut all = Vec::new();
    flatten(&mut all, &form.elements);

    let works_table = all
        .iter()
        .find(|e| e.kind == "Table" && e.name.as_deref() == Some("Работы"))
        .expect("Should find Table 'Работы'");
    assert_eq!(
        works_table.data_path.as_deref(),
        Some("Объект.Работы"),
        "Table 'Работы' should be bound to Объект.Работы"
    );

    let sides_table = all
        .iter()
        .find(|e| e.kind == "Table" && e.name.as_deref() == Some("Стороны"))
        .expect("Should find Table 'Стороны'");
    assert_eq!(
        sides_table.data_path.as_deref(),
        Some("Объект.Стороны"),
        "Table 'Стороны' should be bound to Объект.Стороны"
    );

    // Колонки/дочерние элементы таблицы (ChildItems внутри Table) тоже должны иметь DataPath
    let works_line_number = all
        .iter()
        .find(|e| e.kind == "LabelField" && e.name.as_deref() == Some("РаботыНомерСтроки"))
        .expect("Should find LabelField 'РаботыНомерСтроки'");
    assert_eq!(
        works_line_number.data_path.as_deref(),
        Some("Объект.Работы.LineNumber"),
        "LabelField 'РаботыНомерСтроки' should be bound to Объект.Работы.LineNumber"
    );

    let works_work_kind = all
        .iter()
        .find(|e| e.kind == "InputField" && e.name.as_deref() == Some("РаботыВидРаботы"))
        .expect("Should find InputField 'РаботыВидРаботы'");
    assert_eq!(
        works_work_kind.data_path.as_deref(),
        Some("Объект.Работы.ВидРаботы"),
        "InputField 'РаботыВидРаботы' should be bound to Объект.Работы.ВидРаботы"
    );
}

#[test]
fn test_form_parser_graceful_degradation_no_module() {
    use std::fs;
    use tempfile::TempDir;

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
    use std::fs;
    use tempfile::TempDir;

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

#[test]
fn test_form_synthetic_types_loaded_into_repository() {
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};

    let discovery = ConfigurationDiscovery::new(test_config_path(), false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Failed to discover configurations");

    let first_config = &configurations[0];
    let metadata = discovery
        .discover_metadata_in_configuration(first_config, None::<fn(_)>)
        .expect("Failed to discover metadata");

    let doc = metadata
        .iter()
        .find(|m| m.name == "ЗаказНаряды" && m.object_type_raw == "Document")
        .expect("Should find Document.ЗаказНаряды");

    let raw_types = doc.to_raw_type_data_with_forms(None);

    let repo = InMemoryTypeRepository::new();
    repo.load_types(raw_types).expect("Failed to load types");

    let form_type = repo
        .find_type("Формы.Документы.ЗаказНаряды.ФормаДокумента")
        .expect("Form type should exist in repository");
    let form_props: Vec<_> = form_type
        .properties
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert!(
        form_props.contains(&"Объект"),
        "Form type should have property 'Объект'"
    );
    assert!(
        form_props.contains(&"Работы"),
        "Form type should have property 'Работы'"
    );
    assert!(
        form_props.contains(&"Стороны"),
        "Form type should have property 'Стороны'"
    );

    let form_object = form_type
        .properties
        .iter()
        .find(|p| p.name == "Объект")
        .expect("Form type should have property 'Объект'");
    assert_eq!(
        form_object.prop_type, "ДанныеФормыСтруктура",
        "Form 'Объект' should use canonical form-data type"
    );
    assert!(
        repo.find_type("ДанныеФормыОбъект.Документы.ЗаказНаряды")
            .is_none(),
        "Legacy form object alias should not be generated"
    );

    let works = form_type
        .properties
        .iter()
        .find(|p| p.name == "Работы")
        .expect("Form type should have property 'Работы'");
    assert_eq!(
        works.prop_type, "ДанныеФормыКоллекция<СтрокаРаботы>",
        "Form 'Работы' should be a data forms collection"
    );

    let row_type = repo
        .find_type("СтрокаРаботы")
        .expect("Row type should exist in repository");
    assert!(
        row_type.attributes.iter().any(|a| a.name == "LineNumber"),
        "Row type should include system field LineNumber"
    );

    let form_elements_type = repo
        .find_type("ЭлементыФормы.Документы.ЗаказНаряды.ФормаДокумента")
        .expect("Form elements container type should exist in repository");

    let works = form_elements_type
        .properties
        .iter()
        .find(|p| p.name == "Работы")
        .expect("Elements container should have 'Работы'");
    assert_eq!(
        works.prop_type, "ТаблицаФормы",
        "Элементы.Работы should be ТаблицаФормы"
    );

    let works_line_number = form_elements_type
        .properties
        .iter()
        .find(|p| p.name == "РаботыНомерСтроки")
        .expect("Elements container should have 'РаботыНомерСтроки'");
    assert_eq!(
        works_line_number.prop_type, "ПолеФормы",
        "Элементы.РаботыНомерСтроки should be ПолеФормы"
    );

    let works_work_kind = form_elements_type
        .properties
        .iter()
        .find(|p| p.name == "РаботыВидРаботы")
        .expect("Elements container should have 'РаботыВидРаботы'");
    assert_eq!(
        works_work_kind.prop_type, "ПолеФормы",
        "Элементы.РаботыВидРаботы should be ПолеФормы"
    );
}

#[test]
fn test_discover_forms_for_object_handles_business_processes_folder_name() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();

    let form_xml_path = tmp
        .path()
        .join("BusinessProcesses")
        .join("BP1")
        .join("Forms")
        .join("MainForm")
        .join("Ext")
        .join("Form.xml");

    fs::create_dir_all(form_xml_path.parent().unwrap()).unwrap();
    fs::write(
        &form_xml_path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Form>
    <Attributes>
        <Attribute name="TestAttr" id="1">
            <Type>
                <v8:Type>String</v8:Type>
            </Type>
            <MainAttribute>false</MainAttribute>
            <SavedData>false</SavedData>
        </Attribute>
    </Attributes>
</Form>"#,
    )
    .unwrap();

    let discovery = ConfigurationDiscovery::new(tmp.path().to_path_buf(), false);
    let forms = discovery
        .discover_forms_for_object("BusinessProcesses", "BusinessProcess", "BP1")
        .expect("discover_forms_for_object");

    assert_eq!(forms.len(), 1);
    assert_eq!(forms[0].name, "MainForm");
    assert_eq!(forms[0].owner_type, "BusinessProcess.BP1");
}
