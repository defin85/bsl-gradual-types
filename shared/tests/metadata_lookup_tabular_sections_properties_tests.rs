//! Unit тесты для валидации добавления табличных частей как свойств в get_facet_properties()
//!
//! Тестируемый модуль: shared/src/domain/metadata_lookup.rs
//! Функция: get_facet_properties() - добавление табличных частей как свойств
//!
//! Контекст: Баг 1 - Табличные части теперь видны при валидации свойств
//! Табличные части должны присутствовать в свойствах для Object/Reference фасетов
//! с типом "ТабличнаяЧасть<ИмяТЧ>" и is_readonly = true

#![allow(clippy::vec_init_then_push)]

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, ConfigurationType, FacetKind, MetadataKind, RawAttributeData,
    RawDataSource, RawPropertyData, RawTabularSectionData, RawTypeData, ResolutionMetadata,
    ResolutionResult, ResolutionSource, TypeResolution,
};
use std::sync::Arc;

// === Helper функции для создания тестовых данных ===

/// Создает репозиторий с документом, имеющим табличные части и реквизиты
fn create_test_repository_with_tabular_sections_and_properties() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Создаём документ "ЗаказНаряды" с реквизитами и табличными частями
    let document = RawTypeData {
        name: "Документы.ЗаказНаряды".to_string(),
        english_name: "Documents.WorkOrders".to_string(),
        description: "Документ заказ-наряды".to_string(),
        category: "Документы".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![
            RawPropertyData {
                name: "Номер".to_string(),
                prop_type: "Строка".to_string(),
                is_readonly: false,
            },
            RawPropertyData {
                name: "Дата".to_string(),
                prop_type: "Дата".to_string(),
                is_readonly: false,
            },
            RawPropertyData {
                name: "Организация".to_string(),
                prop_type: "СправочникСсылка.Организации".to_string(),
                is_readonly: false,
            },
        ],
        facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        kind: Some(MetadataKind::Document),
        attributes: vec![],
        tabular_sections: vec![
            RawTabularSectionData {
                name: "Работы".to_string(),
                attributes: vec![
                    RawAttributeData {
                        name: "Номенклатура".to_string(),
                        attr_type: "СправочникСсылка.Номенклатура".to_string(),
                    },
                    RawAttributeData {
                        name: "Количество".to_string(),
                        attr_type: "Число".to_string(),
                    },
                ],
            },
            RawTabularSectionData {
                name: "Стороны".to_string(),
                attributes: vec![
                    RawAttributeData {
                        name: "Сторона".to_string(),
                        attr_type: "СправочникСсылка.Контрагенты".to_string(),
                    },
                    RawAttributeData {
                        name: "Роль".to_string(),
                        attr_type: "ПеречислениеСсылка.РолиСторон".to_string(),
                    },
                ],
            },
        ],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    // Создаём платформенные фасетные типы
    let document_object = RawTypeData {
        name: "ДокументОбъект".to_string(),
        english_name: "DocumentObject".to_string(),
        description: "Объект документа".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![],
        properties: vec![
            RawPropertyData {
                name: "Ссылка".to_string(),
                prop_type: "ДокументСсылка".to_string(),
                is_readonly: true,
            },
            RawPropertyData {
                name: "ЭтоНовый".to_string(),
                prop_type: "Булево".to_string(),
                is_readonly: true,
            },
        ],
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    let document_reference = RawTypeData {
        name: "ДокументСсылка".to_string(),
        english_name: "DocumentRef".to_string(),
        description: "Ссылка на документ".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![],
        properties: vec![RawPropertyData {
            name: "Пустая".to_string(),
            prop_type: "Булево".to_string(),
            is_readonly: true,
        }],
        facets: vec![FacetKind::Reference],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    let document_manager = RawTypeData {
        name: "ДокументМенеджер".to_string(),
        english_name: "DocumentManager".to_string(),
        description: "Менеджер документа".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![],
        properties: vec![], // Manager НЕ показывает свойства
        facets: vec![FacetKind::Manager],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![document, document_object, document_reference, document_manager])
        .expect("Failed to load types");
    repo
}

/// Создает TypeResolution для конфигурационного типа с указанным фасетом
fn create_resolution_for_config_type(
    kind: MetadataKind,
    name: &str,
    facet: FacetKind,
) -> TypeResolution {
    TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind,
            name: name.to_string(),
            facet: Some(facet),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(facet),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![facet],
    }
}

// === Тесты для Object фасета ===

#[test]
fn test_object_facet_includes_tabular_sections_as_properties() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution =
        create_resolution_for_config_type(MetadataKind::Document, "ЗаказНаряды", FacetKind::Object);

    let properties = lookup.get_properties(&resolution);

    // Должны найти табличные части в свойствах
    let raboty_prop = properties
        .iter()
        .find(|p| p.name == "Работы")
        .expect("Табличная часть 'Работы' должна быть в свойствах");

    let storony_prop = properties
        .iter()
        .find(|p| p.name == "Стороны")
        .expect("Табличная часть 'Стороны' должна быть в свойствах");

    // Проверяем тип табличной части
    assert_eq!(
        raboty_prop.prop_type, "ТабличнаяЧасть<Работы>",
        "Тип табличной части должен быть 'ТабличнаяЧасть<Работы>'"
    );
    assert_eq!(
        storony_prop.prop_type, "ТабличнаяЧасть<Стороны>",
        "Тип табличной части должен быть 'ТабличнаяЧасть<Стороны>'"
    );

    // Проверяем is_readonly
    assert!(
        raboty_prop.is_readonly,
        "Табличная часть 'Работы' должна быть readonly"
    );
    assert!(
        storony_prop.is_readonly,
        "Табличная часть 'Стороны' должна быть readonly"
    );
}

#[test]
fn test_object_facet_includes_both_regular_properties_and_tabular_sections() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution =
        create_resolution_for_config_type(MetadataKind::Document, "ЗаказНаряды", FacetKind::Object);

    let properties = lookup.get_properties(&resolution);

    // Должны быть и обычные свойства, и табличные части
    // Платформенные: Ссылка, ЭтоНовый
    // Конфигурационные: Номер, Дата, Организация
    // Табличные части: Работы, Стороны
    assert!(
        properties.len() >= 7,
        "Должно быть минимум 7 свойств (2 платформенных + 3 конфигурационных + 2 табличные части)"
    );

    // Проверяем наличие обычных свойств
    assert!(
        properties.iter().any(|p| p.name == "Номер"),
        "Должно быть обычное свойство 'Номер'"
    );
    assert!(
        properties.iter().any(|p| p.name == "Дата"),
        "Должно быть обычное свойство 'Дата'"
    );
    assert!(
        properties.iter().any(|p| p.name == "Организация"),
        "Должно быть обычное свойство 'Организация'"
    );

    // Проверяем наличие табличных частей
    assert!(
        properties.iter().any(|p| p.name == "Работы"),
        "Должна быть табличная часть 'Работы'"
    );
    assert!(
        properties.iter().any(|p| p.name == "Стороны"),
        "Должна быть табличная часть 'Стороны'"
    );
}

#[test]
fn test_object_facet_no_duplicate_tabular_sections() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution =
        create_resolution_for_config_type(MetadataKind::Document, "ЗаказНаряды", FacetKind::Object);

    let properties = lookup.get_properties(&resolution);

    // Подсчитываем количество свойств с именем "Работы"
    let raboty_count = properties.iter().filter(|p| p.name == "Работы").count();
    assert_eq!(
        raboty_count, 1,
        "Табличная часть 'Работы' должна встречаться только один раз"
    );

    let storony_count = properties.iter().filter(|p| p.name == "Стороны").count();
    assert_eq!(
        storony_count, 1,
        "Табличная часть 'Стороны' должна встречаться только один раз"
    );
}

// === Тесты для Reference фасета ===

#[test]
fn test_reference_facet_includes_tabular_sections_as_properties() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Reference,
    );

    let properties = lookup.get_properties(&resolution);

    // Reference фасет тоже должен показывать табличные части
    assert!(
        properties.iter().any(|p| p.name == "Работы"),
        "Reference фасет должен показывать табличную часть 'Работы'"
    );
    assert!(
        properties.iter().any(|p| p.name == "Стороны"),
        "Reference фасет должен показывать табличную часть 'Стороны'"
    );

    // Проверяем тип и readonly
    let raboty_prop = properties.iter().find(|p| p.name == "Работы").unwrap();
    assert_eq!(raboty_prop.prop_type, "ТабличнаяЧасть<Работы>");
    assert!(raboty_prop.is_readonly);
}

#[test]
fn test_reference_facet_properties_are_readonly() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Reference,
    );

    let properties = lookup.get_properties(&resolution);

    // У Reference фасета ВСЕ конфигурационные свойства (включая табличные части) должны быть readonly
    let config_properties: Vec<_> = properties
        .iter()
        .filter(|p| {
            // Платформенные свойства начинаются с заглавной буквы
            // Конфигурационные свойства и табличные части - с заглавной на кириллице
            p.name == "Номер" || p.name == "Дата" || p.name == "Организация"
                || p.name == "Работы" || p.name == "Стороны"
        })
        .collect();

    for prop in config_properties {
        assert!(
            prop.is_readonly,
            "Свойство '{}' у Reference фасета должно быть readonly",
            prop.name
        );
    }
}

// === Тесты для Manager фасета ===

#[test]
fn test_manager_facet_does_not_include_tabular_sections() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Manager,
    );

    let properties = lookup.get_properties(&resolution);

    // Manager фасет НЕ показывает свойства вообще
    assert!(
        properties.is_empty(),
        "Manager фасет не должен показывать свойства (в том числе табличные части)"
    );
}

#[test]
fn test_manager_facet_returns_empty_properties() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Manager,
    );

    let properties = lookup.get_properties(&resolution);

    // Проверяем, что НЕТ ни табличных частей, ни обычных свойств
    assert!(
        !properties.iter().any(|p| p.name == "Работы"),
        "Manager не должен показывать 'Работы'"
    );
    assert!(
        !properties.iter().any(|p| p.name == "Стороны"),
        "Manager не должен показывать 'Стороны'"
    );
    assert!(
        !properties.iter().any(|p| p.name == "Номер"),
        "Manager не должен показывать 'Номер'"
    );
}

// === Тесты для Selection/List фасетов ===

#[test]
fn test_selection_facet_does_not_include_tabular_sections() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Selection,
    );

    let properties = lookup.get_properties(&resolution);

    // Selection фасет НЕ показывает свойства
    assert!(
        properties.is_empty(),
        "Selection фасет не должен показывать свойства"
    );
}

#[test]
fn test_list_facet_does_not_include_tabular_sections() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution =
        create_resolution_for_config_type(MetadataKind::Document, "ЗаказНаряды", FacetKind::List);

    let properties = lookup.get_properties(&resolution);

    // List фасет НЕ показывает свойства
    assert!(
        properties.is_empty(),
        "List фасет не должен показывать свойства"
    );
}

// === Edge case тесты ===

#[test]
fn test_document_without_tabular_sections_still_works() {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Создаём документ БЕЗ табличных частей
    let document = RawTypeData {
        name: "Документы.ПростойДокумент".to_string(),
        english_name: "Documents.SimpleDocument".to_string(),
        description: "Простой документ без табличных частей".to_string(),
        category: "Документы".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![RawPropertyData {
            name: "Номер".to_string(),
            prop_type: "Строка".to_string(),
            is_readonly: false,
        }],
        facets: vec![FacetKind::Object],
        kind: Some(MetadataKind::Document),
        attributes: vec![],
        tabular_sections: vec![], // Пустой список табличных частей
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    let document_object = RawTypeData {
        name: "ДокументОбъект".to_string(),
        english_name: "DocumentObject".to_string(),
        description: "Объект документа".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![document, document_object])
        .expect("Failed to load types");

    let lookup = TypeMetadataLookup::new(repo);

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ПростойДокумент",
        FacetKind::Object,
    );

    let properties = lookup.get_properties(&resolution);

    // Должны быть только обычные свойства, без табличных частей
    assert!(
        properties.iter().any(|p| p.name == "Номер"),
        "Должно быть обычное свойство 'Номер'"
    );
    assert!(
        !properties.iter().any(|p| p.prop_type.starts_with("ТабличнаяЧасть<")),
        "Не должно быть табличных частей"
    );
}

#[test]
fn test_tabular_section_type_format() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution =
        create_resolution_for_config_type(MetadataKind::Document, "ЗаказНаряды", FacetKind::Object);

    let properties = lookup.get_properties(&resolution);

    // Проверяем точный формат типа табличной части
    let raboty_prop = properties.iter().find(|p| p.name == "Работы").unwrap();

    // Формат должен быть ТОЧНО: "ТабличнаяЧасть<Работы>"
    assert_eq!(
        raboty_prop.prop_type, "ТабличнаяЧасть<Работы>",
        "Формат типа табличной части должен быть 'ТабличнаяЧасть<ИмяТЧ>'"
    );

    // Не должно быть пробелов или других символов
    assert!(
        !raboty_prop.prop_type.contains(" "),
        "Тип не должен содержать пробелы"
    );
}

// === Интеграционный тест ===

#[test]
fn test_integration_object_facet_complete_property_list() {
    let repo = create_test_repository_with_tabular_sections_and_properties();
    let lookup = TypeMetadataLookup::new(repo);

    let resolution =
        create_resolution_for_config_type(MetadataKind::Document, "ЗаказНаряды", FacetKind::Object);

    let properties = lookup.get_properties(&resolution);

    // Собираем все имена свойств
    let property_names: Vec<_> = properties.iter().map(|p| p.name.as_str()).collect();

    // Платформенные свойства
    assert!(
        property_names.contains(&"Ссылка"),
        "Должно быть платформенное свойство 'Ссылка'"
    );
    assert!(
        property_names.contains(&"ЭтоНовый"),
        "Должно быть платформенное свойство 'ЭтоНовый'"
    );

    // Конфигурационные свойства (реквизиты)
    assert!(
        property_names.contains(&"Номер"),
        "Должно быть конфигурационное свойство 'Номер'"
    );
    assert!(
        property_names.contains(&"Дата"),
        "Должно быть конфигурационное свойство 'Дата'"
    );
    assert!(
        property_names.contains(&"Организация"),
        "Должно быть конфигурационное свойство 'Организация'"
    );

    // Табличные части
    assert!(
        property_names.contains(&"Работы"),
        "Должна быть табличная часть 'Работы'"
    );
    assert!(
        property_names.contains(&"Стороны"),
        "Должна быть табличная часть 'Стороны'"
    );

    // Проверяем типы табличных частей
    let raboty = properties.iter().find(|p| p.name == "Работы").unwrap();
    let storony = properties.iter().find(|p| p.name == "Стороны").unwrap();

    assert_eq!(raboty.prop_type, "ТабличнаяЧасть<Работы>");
    assert_eq!(storony.prop_type, "ТабличнаяЧасть<Стороны>");
    assert!(raboty.is_readonly);
    assert!(storony.is_readonly);
}
