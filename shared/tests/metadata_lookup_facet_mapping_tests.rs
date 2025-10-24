//! Unit тесты для валидации mapping функции get_platform_facet_type()
//!
//! Тестируемый модуль: shared/src/domain/metadata_lookup.rs
//! Функция: get_platform_facet_type(MetadataKind, FacetKind) -> Option<&'static str>
//!
//! Задача: Проверить все 10 поддерживаемых комбинаций + edge cases

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::{TypeRepository, InMemoryTypeRepository};
use bsl_shared::domain::types::{
    TypeResolution, ResolutionResult, ConcreteType, ConfigurationType,
    MetadataKind, FacetKind, RawTypeData, RawMethodData, RawDataSource,
    Certainty, ResolutionSource, ResolutionMetadata,
};
use std::sync::Arc;

// === Helper функции для создания тестовых данных ===

/// Создает тестовый репозиторий с платформенными типами для всех фасетов
fn create_test_repository_with_all_platform_facets() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());

    let mut platform_types = Vec::new();

    // === Документы (5 фасетов) ===

    // ДокументМенеджер
    platform_types.push(RawTypeData {
        name: "ДокументМенеджер".to_string(),
        english_name: "DocumentManager".to_string(),
        description: "Менеджер документа".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "СоздатьДокумент".to_string(),
                english_name: "CreateDocument".to_string(),
                return_type: "ДокументОбъект".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "НайтиПоНомеру".to_string(),
                english_name: "FindByNumber".to_string(),
                return_type: "ДокументСсылка".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Manager],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // ДокументОбъект
    platform_types.push(RawTypeData {
        name: "ДокументОбъект".to_string(),
        english_name: "DocumentObject".to_string(),
        description: "Объект документа".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Записать".to_string(),
                english_name: "Write".to_string(),
                return_type: "".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Провести".to_string(),
                english_name: "Post".to_string(),
                return_type: "".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // ДокументСсылка
    platform_types.push(RawTypeData {
        name: "ДокументСсылка".to_string(),
        english_name: "DocumentRef".to_string(),
        description: "Ссылка на документ".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "ПолучитьОбъект".to_string(),
                english_name: "GetObject".to_string(),
                return_type: "ДокументОбъект".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Reference],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // ДокументВыборка
    platform_types.push(RawTypeData {
        name: "ДокументВыборка".to_string(),
        english_name: "DocumentSelection".to_string(),
        description: "Выборка документов".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Следующий".to_string(),
                english_name: "Next".to_string(),
                return_type: "Булево".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Selection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // ДокументСписок
    platform_types.push(RawTypeData {
        name: "ДокументСписок".to_string(),
        english_name: "DocumentList".to_string(),
        description: "Список документов в форме".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Обновить".to_string(),
                english_name: "Refresh".to_string(),
                return_type: "".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::List],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // === Справочники (5 фасетов) ===

    // СправочникМенеджер
    platform_types.push(RawTypeData {
        name: "СправочникМенеджер".to_string(),
        english_name: "CatalogManager".to_string(),
        description: "Менеджер справочника".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "СоздатьЭлемент".to_string(),
                english_name: "CreateItem".to_string(),
                return_type: "СправочникОбъект".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "НайтиПоКоду".to_string(),
                english_name: "FindByCode".to_string(),
                return_type: "СправочникСсылка".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Manager],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // СправочникОбъект
    platform_types.push(RawTypeData {
        name: "СправочникОбъект".to_string(),
        english_name: "CatalogObject".to_string(),
        description: "Объект справочника".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Записать".to_string(),
                english_name: "Write".to_string(),
                return_type: "".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // СправочникСсылка
    platform_types.push(RawTypeData {
        name: "СправочникСсылка".to_string(),
        english_name: "CatalogRef".to_string(),
        description: "Ссылка на справочник".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "ПолучитьОбъект".to_string(),
                english_name: "GetObject".to_string(),
                return_type: "СправочникОбъект".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Пустая".to_string(),
                english_name: "IsEmpty".to_string(),
                return_type: "Булево".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Reference],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // СправочникВыборка
    platform_types.push(RawTypeData {
        name: "СправочникВыборка".to_string(),
        english_name: "CatalogSelection".to_string(),
        description: "Выборка справочника".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Следующий".to_string(),
                english_name: "Next".to_string(),
                return_type: "Булево".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Selection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    // СправочникСписок
    platform_types.push(RawTypeData {
        name: "СправочникСписок".to_string(),
        english_name: "CatalogList".to_string(),
        description: "Список справочника в форме".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Обновить".to_string(),
                english_name: "Refresh".to_string(),
                return_type: "".to_string(),
                params: vec![],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::List],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    });

    repo.load_types(platform_types).expect("Failed to load platform types");
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

// === Unit тесты для Documents (5 фасетов) ===

#[test]
fn test_document_manager_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Manager,
    );

    let methods = lookup.get_methods(&resolution);

    // Должны найти методы из ДокументМенеджер
    assert!(!methods.is_empty(), "Lazy lookup должен найти методы ДокументМенеджер");
    assert_eq!(methods.len(), 2, "Должно быть 2 метода");

    assert!(
        methods.iter().any(|m| m.name == "СоздатьДокумент"),
        "Должен быть метод СоздатьДокумент"
    );
    assert!(
        methods.iter().any(|m| m.name == "НайтиПоНомеру"),
        "Должен быть метод НайтиПоНомеру"
    );
}

#[test]
fn test_document_object_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Object,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы ДокументОбъект");
    assert_eq!(methods.len(), 2, "Должно быть 2 метода");

    assert!(
        methods.iter().any(|m| m.name == "Записать"),
        "Должен быть метод Записать"
    );
    assert!(
        methods.iter().any(|m| m.name == "Провести"),
        "Должен быть метод Провести"
    );
}

#[test]
fn test_document_reference_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Reference,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы ДокументСсылка");
    assert_eq!(methods.len(), 1, "Должен быть 1 метод");

    assert!(
        methods.iter().any(|m| m.name == "ПолучитьОбъект"),
        "Должен быть метод ПолучитьОбъект"
    );
}

#[test]
fn test_document_selection_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Selection,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы ДокументВыборка");
    assert_eq!(methods.len(), 1, "Должен быть 1 метод");

    assert!(
        methods.iter().any(|m| m.name == "Следующий"),
        "Должен быть метод Следующий"
    );
}

#[test]
fn test_document_list_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::List,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы ДокументСписок");
    assert_eq!(methods.len(), 1, "Должен быть 1 метод");

    assert!(
        methods.iter().any(|m| m.name == "Обновить"),
        "Должен быть метод Обновить"
    );
}

// === Unit тесты для Catalogs (5 фасетов) ===

#[test]
fn test_catalog_manager_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Catalog,
        "Контрагенты",
        FacetKind::Manager,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы СправочникМенеджер");
    assert_eq!(methods.len(), 2, "Должно быть 2 метода");

    assert!(
        methods.iter().any(|m| m.name == "СоздатьЭлемент"),
        "Должен быть метод СоздатьЭлемент"
    );
    assert!(
        methods.iter().any(|m| m.name == "НайтиПоКоду"),
        "Должен быть метод НайтиПоКоду"
    );
}

#[test]
fn test_catalog_object_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Catalog,
        "Контрагенты",
        FacetKind::Object,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы СправочникОбъект");
    assert_eq!(methods.len(), 1, "Должен быть 1 метод");

    assert!(
        methods.iter().any(|m| m.name == "Записать"),
        "Должен быть метод Записать"
    );
}

#[test]
fn test_catalog_reference_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Catalog,
        "Контрагенты",
        FacetKind::Reference,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы СправочникСсылка");
    assert_eq!(methods.len(), 2, "Должно быть 2 метода");

    assert!(
        methods.iter().any(|m| m.name == "ПолучитьОбъект"),
        "Должен быть метод ПолучитьОбъект"
    );
    assert!(
        methods.iter().any(|m| m.name == "Пустая"),
        "Должен быть метод Пустая"
    );
}

#[test]
fn test_catalog_selection_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Catalog,
        "Контрагенты",
        FacetKind::Selection,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы СправочникВыборка");
    assert_eq!(methods.len(), 1, "Должен быть 1 метод");

    assert!(
        methods.iter().any(|m| m.name == "Следующий"),
        "Должен быть метод Следующий"
    );
}

#[test]
fn test_catalog_list_facet_mapping() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Catalog,
        "Контрагенты",
        FacetKind::List,
    );

    let methods = lookup.get_methods(&resolution);

    assert!(!methods.is_empty(), "Lazy lookup должен найти методы СправочникСписок");
    assert_eq!(methods.len(), 1, "Должен быть 1 метод");

    assert!(
        methods.iter().any(|m| m.name == "Обновить"),
        "Должен быть метод Обновить"
    );
}

// === Edge cases: unsupported combinations ===

#[test]
fn test_unsupported_enum_metadata_kind() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Enum metadata kind пока не поддерживается в mapping
    let resolution = create_resolution_for_config_type(
        MetadataKind::Enum,
        "СтатусыДокументов",
        FacetKind::Manager,
    );

    let methods = lookup.get_methods(&resolution);

    // Должен использовать fallback логику → вернуть пустой массив
    assert!(methods.is_empty(), "Enum должен вернуть пустой массив (fallback)");
}

#[test]
fn test_unsupported_register_metadata_kind() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Register metadata kind пока не поддерживается в mapping
    let resolution = create_resolution_for_config_type(
        MetadataKind::InformationRegister,
        "Настройки",
        FacetKind::Manager,
    );

    let methods = lookup.get_methods(&resolution);

    // Должен использовать fallback логику → вернуть пустой массив
    assert!(methods.is_empty(), "Register должен вернуть пустой массив (fallback)");
}

#[test]
fn test_unsupported_facet_kind_for_document() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // FacetKind::Constructor не поддерживается для Document в mapping
    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Constructor,
    );

    let methods = lookup.get_methods(&resolution);

    // Должен использовать fallback логику
    assert!(methods.is_empty(), "Constructor facet должен вернуть пустой массив (fallback)");
}

// === Edge case: Платформенный тип отсутствует в репозитории ===

#[test]
fn test_platform_type_not_found_returns_empty() {
    // Создаём пустой репозиторий БЕЗ платформенных типов
    let repo = Arc::new(InMemoryTypeRepository::new());
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Manager,
    );

    let methods = lookup.get_methods(&resolution);

    // Lazy lookup попытается найти "ДокументМенеджер", но не найдёт
    // Должен использовать fallback логику → вернуть пустой массив
    assert!(methods.is_empty(), "Если платформенный тип не загружен, должен вернуть пустой массив");
}

// === Edge case: active_facet = None ===

#[test]
fn test_no_active_facet_uses_fallback() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let mut resolution = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Manager,
    );

    // Убираем active_facet
    resolution.active_facet = None;

    let methods = lookup.get_methods(&resolution);

    // Без active_facet lazy lookup не сработает
    // Должен использовать fallback через extract_type_name()
    // Но для конфигурационного типа "Документы.ЗаказНаряды" RawTypeData не существует
    // → Вернёт пустой массив
    assert!(methods.is_empty(), "Без active_facet должен использовать fallback (пустой массив)");
}
