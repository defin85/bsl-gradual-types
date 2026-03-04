use super::*;
use std::collections::BTreeSet;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn parses_enum_values_from_xml() {
    let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
<Enum uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
        <Name>ТестПеречисление</Name>
    </Properties>
    <ChildObjects>
        <EnumValue uuid="11111111-1111-1111-1111-111111111111">
            <Properties>
                <Name>Первое</Name>
            </Properties>
        </EnumValue>
        <EnumValue uuid="22222222-2222-2222-2222-222222222222">
            <Properties>
                <Name>Второе</Name>
            </Properties>
        </EnumValue>
    </ChildObjects>
</Enum>
</MetaDataObject>
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(xml.as_bytes()).unwrap();

    let parsed = UniversalMetadataParser::parse_any_object(file.path()).unwrap();
    assert_eq!(parsed.enum_values.len(), 2);
    assert!(parsed.enum_values.contains(&"Первое".to_string()));
    assert!(parsed.enum_values.contains(&"Второе".to_string()));
}

#[test]
fn parses_predefined_items_from_xml_with_nested_items() {
    let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<PredefinedData xmlns="http://v8.1c.ru/8.3/xcf/predef">
<Item id="root">
    <Name>Корень</Name>
    <ChildItems>
        <Item id="child-1">
            <Name>Потомок1</Name>
        </Item>
        <Item id="child-2">
            <PredefinedDataName>Потомок2</PredefinedDataName>
        </Item>
    </ChildItems>
</Item>
</PredefinedData>
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(xml.as_bytes()).unwrap();

    let items = UniversalMetadataParser::parse_predefined_items(file.path()).unwrap();
    assert_eq!(items, vec!["Корень", "Потомок1", "Потомок2"]);
}

#[test]
fn parses_document_standard_attributes_with_fallback_from_properties() {
    let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
<Document uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
        <Name>Док1</Name>
        <NumberType>String</NumberType>
        <Posting>Allow</Posting>
    </Properties>
</Document>
</MetaDataObject>
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(xml.as_bytes()).unwrap();

    let parsed = UniversalMetadataParser::parse_any_object(file.path()).unwrap();
    let attrs: BTreeSet<_> = parsed.standard_attributes.iter().cloned().collect();
    assert!(attrs.contains("Ref"));
    assert!(attrs.contains("DeletionMark"));
    assert!(attrs.contains("Date"));
    assert!(attrs.contains("Number"));
    assert!(attrs.contains("Posted"));
    assert_eq!(
        parsed.properties.get("NumberType").map(String::as_str),
        Some("String")
    );
    assert_eq!(
        parsed.properties.get("Posting").map(String::as_str),
        Some("Allow")
    );
}

#[test]
fn excludes_posted_standard_attribute_for_non_posting_document() {
    let xml = r#"
<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:xr="http://v8.1c.ru/8.3/xcf/readable">
<Document uuid="00000000-0000-0000-0000-000000000002">
    <Properties>
        <Name>Док2</Name>
        <Posting>Deny</Posting>
        <StandardAttributes>
            <xr:StandardAttribute name="Posted"/>
            <xr:StandardAttribute name="Ref"/>
        </StandardAttributes>
    </Properties>
</Document>
</MetaDataObject>
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(xml.as_bytes()).unwrap();

    let parsed = UniversalMetadataParser::parse_any_object(file.path()).unwrap();
    assert!(parsed.standard_attributes.iter().any(|name| name == "Ref"));
    assert!(
        parsed
            .standard_attributes
            .iter()
            .all(|name| !name.eq_ignore_ascii_case("Posted")),
        "Posted should not be present for non-posting document"
    );
}
