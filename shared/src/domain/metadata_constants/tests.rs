use super::*;

#[test]
fn test_metadata_collections() {
    // Русские имена (регистрозависимые)
    assert!(is_metadata_collection("Справочники"));
    assert!(is_metadata_collection("Документы"));
    assert!(is_metadata_collection("РегистрыСведений"));

    // Английские имена
    assert!(is_metadata_collection("Catalogs"));
    assert!(is_metadata_collection("Documents"));
    assert!(is_metadata_collection("InformationRegisters"));

    // Регистронезависимость только для латиницы
    assert!(is_metadata_collection("CATALOGS"));
    assert!(is_metadata_collection("catalogs"));

    // Кириллица регистрозависима
    assert!(!is_metadata_collection("справочники")); // lowercase не работает

    // Не коллекции
    assert!(!is_metadata_collection("СправочникМенеджер"));
    assert!(!is_metadata_collection("Unknown"));
}

#[test]
fn test_faceted_types() {
    // Русские имена
    assert!(is_faceted_type("СправочникМенеджер"));
    assert!(is_faceted_type("ДокументОбъект"));
    assert!(is_faceted_type("РегистрСведенийНаборЗаписей"));

    // Английские имена
    assert!(is_faceted_type("CatalogManager"));
    assert!(is_faceted_type("DocumentObject"));

    // Не фасетные типы
    assert!(!is_faceted_type("Справочники"));
    assert!(!is_faceted_type("Unknown"));
}

#[test]
fn test_get_collection_kind() {
    assert_eq!(
        get_collection_kind("Справочники"),
        Some(MetadataKind::Catalog)
    );
    assert_eq!(get_collection_kind("Catalogs"), Some(MetadataKind::Catalog));
    assert_eq!(
        get_collection_kind("Документы"),
        Some(MetadataKind::Document)
    );
    assert_eq!(get_collection_kind("Unknown"), None);
}

#[test]
fn test_get_faceted_type_info() {
    assert_eq!(
        get_faceted_type_info("СправочникМенеджер"),
        Some((MetadataKind::Catalog, FacetKind::Manager))
    );
    assert_eq!(
        get_faceted_type_info("ДокументОбъект"),
        Some((MetadataKind::Document, FacetKind::Object))
    );
    assert_eq!(
        get_faceted_type_info("ПеречислениеМенеджер"),
        Some((MetadataKind::Enum, FacetKind::Manager))
    );
    assert_eq!(
        get_faceted_type_info("EnumRef"),
        Some((MetadataKind::Enum, FacetKind::Reference))
    );
    assert_eq!(get_faceted_type_info("Unknown"), None);
}

#[test]
fn test_get_base_type_info() {
    // Коллекции возвращают Manager facet
    assert_eq!(
        get_base_type_info("Справочники"),
        Some((MetadataKind::Catalog, FacetKind::Manager))
    );

    // Фасетные типы возвращают свой facet
    assert_eq!(
        get_base_type_info("СправочникОбъект"),
        Some((MetadataKind::Catalog, FacetKind::Object))
    );
    assert_eq!(
        get_base_type_info("ПеречислениеМенеджер"),
        Some((MetadataKind::Enum, FacetKind::Manager))
    );
}

#[test]
fn test_is_configuration_type_pattern() {
    // Паттерны с точкой
    assert!(is_configuration_type_pattern("Справочники.Контрагенты"));
    assert!(is_configuration_type_pattern("Catalogs.Counterparties"));
    assert!(is_configuration_type_pattern(
        "СправочникМенеджер.Контрагенты"
    ));
    assert!(is_configuration_type_pattern(
        "ПеречислениеМенеджер.ВидыОпераций"
    ));

    // Без точки - сами коллекции/фасеты
    assert!(is_configuration_type_pattern("Справочники"));
    assert!(is_configuration_type_pattern("СправочникМенеджер"));
    assert!(is_configuration_type_pattern("ПеречислениеСсылка"));

    // Не конфигурационные
    assert!(!is_configuration_type_pattern("Массив"));
    assert!(!is_configuration_type_pattern("Unknown.Something"));
}
