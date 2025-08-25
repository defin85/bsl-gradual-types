//! Простые unit-тесты для Configuration-guided Discovery парсера

use bsl_gradual_types::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;
use bsl_gradual_types::core::types::MetadataKind;

#[test]
fn test_parse_basic() {
    let mut parser = ConfigurationGuidedParser::new("tests/fixtures/xml_full");
    let resolutions = parser.parse_with_configuration_guide().unwrap();

    assert_eq!(resolutions.len(), 10);

    let stats = parser.get_guided_discovery_stats();
    assert_eq!(stats.found_objects, 4);
    assert_eq!(stats.catalogs, 2);
    assert_eq!(stats.documents, 1);
    assert_eq!(stats.registers, 1);
}

#[test]
fn test_catalog_kontragenty() {
    let mut parser = ConfigurationGuidedParser::new("tests/fixtures/xml_full");
    parser.parse_with_configuration_guide().unwrap();

    let metadata = parser
        .get_discovered_metadata("Справочники.Контрагенты")
        .unwrap();

    assert_eq!(metadata.name, "Контрагенты");
    assert_eq!(metadata.kind, MetadataKind::Catalog);
    assert_eq!(
        metadata.uuid,
        Some("e3cbde3c-52ed-494d-bd38-ed777a837517".to_string())
    );
    assert_eq!(metadata.attributes.len(), 3);
}

#[test]
fn test_register_attributes() {
    let mut parser = ConfigurationGuidedParser::new("tests/fixtures/xml_full");
    parser.parse_with_configuration_guide().unwrap();

    let metadata = parser
        .get_discovered_metadata("РегистрыСведений.ТестовыйРегистрСведений")
        .unwrap();

    assert_eq!(metadata.name, "ТестовыйРегистрСведений");
    assert_eq!(metadata.kind, MetadataKind::Register);
    assert_eq!(metadata.attributes.len(), 5);

    let attr_names: Vec<&str> = metadata
        .attributes
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert!(attr_names.contains(&"ТестовыйРесурс"));
    assert!(attr_names.contains(&"ТестовыйРеквизит"));
    assert!(attr_names.contains(&"ТестовоеИзмерение"));
    assert!(attr_names.contains(&"Период"));
    assert!(attr_names.contains(&"Активность"));
}
