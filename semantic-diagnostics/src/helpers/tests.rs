use super::*;

#[test]
fn test_is_metadata_collection_name() {
    // Russian names
    assert!(is_metadata_collection_name("Справочники"));
    assert!(is_metadata_collection_name("Документы"));
    assert!(is_metadata_collection_name("РегистрыСведений"));
    assert!(is_metadata_collection_name("Перечисления"));

    // English names
    assert!(is_metadata_collection_name("Catalogs"));
    assert!(is_metadata_collection_name("Documents"));
    assert!(is_metadata_collection_name("InformationRegisters"));
    assert!(is_metadata_collection_name("Enums"));

    // Not metadata collections
    assert!(!is_metadata_collection_name("Массив"));
    assert!(!is_metadata_collection_name("ТаблицаЗначений"));
    assert!(!is_metadata_collection_name("Строка"));
}

#[test]
fn test_collection_name_to_metadata_kind() {
    // Russian names
    assert_eq!(
        collection_name_to_metadata_kind("Справочники"),
        Some(MetadataKind::Catalog)
    );
    assert_eq!(
        collection_name_to_metadata_kind("Документы"),
        Some(MetadataKind::Document)
    );
    assert_eq!(
        collection_name_to_metadata_kind("РегистрыСведений"),
        Some(MetadataKind::InformationRegister)
    );
    assert_eq!(
        collection_name_to_metadata_kind("РегистрыНакопления"),
        Some(MetadataKind::AccumulationRegister)
    );

    // English names
    assert_eq!(
        collection_name_to_metadata_kind("Catalogs"),
        Some(MetadataKind::Catalog)
    );
    assert_eq!(
        collection_name_to_metadata_kind("Documents"),
        Some(MetadataKind::Document)
    );

    // Unknown
    assert_eq!(collection_name_to_metadata_kind("Массив"), None);
    assert_eq!(collection_name_to_metadata_kind("Unknown"), None);
}
