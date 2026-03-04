use super::*;

#[test]
fn test_parse_attributes_simple() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Form>
<Attributes>
    <Attribute name="Объект" id="1">
        <Type>
            <v8:Type>cfg:DocumentObject.ЗаказНаряды</v8:Type>
        </Type>
        <MainAttribute>true</MainAttribute>
        <SavedData>true</SavedData>
    </Attribute>
</Attributes>
</Form>"#;

    let attributes = FormParser::parse_attributes(xml).unwrap();

    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0].name, "Объект");
    assert_eq!(attributes[0].id, 1);
    assert!(attributes[0].is_main_attribute);
    assert!(attributes[0].saved_data);
    assert_eq!(attributes[0].type_description.types.len(), 1);
    assert_eq!(
        attributes[0].type_description.types[0],
        "cfg:DocumentObject.ЗаказНаряды"
    );
}

#[test]
fn test_parse_events_simple() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Form>
<Events>
    <Event name="OnCreateAtServer">ПриСозданииНаСервере</Event>
    <Event name="OnOpen">ПриОткрытии</Event>
</Events>
</Form>"#;

    let events = FormParser::parse_events(xml).unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].name, "OnCreateAtServer");
    assert_eq!(events[0].handler_name, "ПриСозданииНаСервере");
    assert_eq!(events[1].name, "OnOpen");
    assert_eq!(events[1].handler_name, "ПриОткрытии");
}

#[test]
fn test_parse_module_contexts_server_only() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "&НаСервере").unwrap();
    writeln!(temp_file, "Процедура ПриСозданииНаСервере()").unwrap();
    writeln!(temp_file, "КонецПроцедуры").unwrap();

    let contexts = FormParser::parse_module_contexts(temp_file.path()).unwrap();

    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0], ExecutionContext::Server);
}

#[test]
fn test_parse_module_contexts_no_directives() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "Процедура Тест()").unwrap();
    writeln!(temp_file, "КонецПроцедуры").unwrap();

    let contexts = FormParser::parse_module_contexts(temp_file.path()).unwrap();

    // Без директив - оба контекста
    assert_eq!(contexts.len(), 2);
    assert!(contexts.contains(&ExecutionContext::Server));
    assert!(contexts.contains(&ExecutionContext::ClientManaged));
}
