use super::*;

#[test]
fn test_extract_pattern_catalog_manager() {
    let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
        "СправочникМенеджер.<Имя справочника>",
    );
    assert!(pattern.is_some());
    let p = pattern.unwrap();
    assert_eq!(p.prefix, "Справочник");
    assert_eq!(p.kind, MetadataKind::Catalog);
}

#[test]
fn test_extract_pattern_document_object() {
    let pattern =
        MetadataPatternRegistry::extract_pattern_from_type_name("ДокументОбъект.<Имя документа>");
    assert!(pattern.is_some());
    let p = pattern.unwrap();
    assert_eq!(p.prefix, "Документ");
    assert_eq!(p.kind, MetadataKind::Document);
}

#[test]
fn test_extract_pattern_info_register_recordset() {
    let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
        "РегистрСведенийНаборЗаписей.<Имя регистра сведений>",
    );
    assert!(pattern.is_some());
    let p = pattern.unwrap();
    assert_eq!(p.prefix, "РегистрСведений");
    assert_eq!(p.kind, MetadataKind::InformationRegister);
}

#[test]
fn test_extract_pattern_exchange_plan() {
    let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
        "ПланОбменаМенеджер.<Имя плана обмена>",
    );
    assert!(pattern.is_some());
    let p = pattern.unwrap();
    assert_eq!(p.prefix, "ПланОбмена");
    assert_eq!(p.kind, MetadataKind::ExchangePlan);
}

#[test]
fn test_extract_pattern_non_faceted() {
    let pattern = MetadataPatternRegistry::extract_pattern_from_type_name("Массив");
    assert!(pattern.is_none());
}

#[test]
fn test_registry_without_patterns_returns_none() {
    let registry = MetadataPatternRegistry::new();
    assert_eq!(registry.get_metadata_kind("СправочникМенеджер"), None);
    assert_eq!(registry.get_metadata_kind("ДокументОбъект"), None);
    assert_eq!(
        registry.get_metadata_kind("РегистрСведенийНаборЗаписей"),
        None
    );
    assert_eq!(registry.get_metadata_kind("ПланОбменаОбъект"), None);
}

#[test]
fn test_registry_with_extracted_patterns() {
    let mut registry = MetadataPatternRegistry::new();
    registry.update_from_patterns(vec![ExtractedPattern {
        prefix: "Справочник".to_string(),
        kind: MetadataKind::Catalog,
        placeholder_suffix: Some("справочника".to_string()),
    }]);

    assert!(registry.has_extracted_patterns());
    assert_eq!(registry.extracted_count(), 1);
    assert_eq!(
        registry.get_metadata_kind("СправочникМенеджер"),
        Some(MetadataKind::Catalog)
    );
}

#[test]
fn test_strip_facet_suffix() {
    assert_eq!(
        MetadataPatternRegistry::strip_facet_suffix("СправочникМенеджер"),
        "Справочник"
    );
    assert_eq!(
        MetadataPatternRegistry::strip_facet_suffix("ДокументОбъект"),
        "Документ"
    );
    assert_eq!(
        MetadataPatternRegistry::strip_facet_suffix("РегистрСведенийНаборЗаписей"),
        "РегистрСведений"
    );
    assert_eq!(
        MetadataPatternRegistry::strip_facet_suffix("Массив"),
        "Массив"
    );
    assert_eq!(
        MetadataPatternRegistry::strip_facet_suffix("ПланОбменаСсылка"),
        "ПланОбмена"
    );
}

#[test]
fn test_registry_extracted_patterns_match() {
    let mut registry = MetadataPatternRegistry::new();

    registry.update_from_patterns(vec![ExtractedPattern {
        prefix: "Справочник".to_string(),
        kind: MetadataKind::Catalog,
        placeholder_suffix: Some("справочника".to_string()),
    }]);

    assert_eq!(
        registry.get_metadata_kind("СправочникОбъект"),
        Some(MetadataKind::Catalog)
    );
}

#[test]
fn test_metadata_kind_from_placeholder_variations() {
    // Тест различных форматов placeholder текста
    assert_eq!(
        MetadataPatternRegistry::metadata_kind_from_placeholder("имя справочника"),
        Some(MetadataKind::Catalog)
    );
    assert_eq!(
        MetadataPatternRegistry::metadata_kind_from_placeholder("имя регистра сведений"),
        Some(MetadataKind::InformationRegister)
    );
    assert_eq!(
        MetadataPatternRegistry::metadata_kind_from_placeholder("имя плана обмена"),
        Some(MetadataKind::ExchangePlan)
    );
    // Неизвестный placeholder
    assert_eq!(
        MetadataPatternRegistry::metadata_kind_from_placeholder("неизвестный тип"),
        None
    );
}

#[test]
fn test_extract_pattern_with_html_entities() {
    // Тест для HTML-encoded placeholder (некоторые источники могут кодировать)
    let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
        "СправочникОбъект.&lt;Имя справочника&gt;",
    );
    // Должен распознать
    assert!(pattern.is_some());
}
