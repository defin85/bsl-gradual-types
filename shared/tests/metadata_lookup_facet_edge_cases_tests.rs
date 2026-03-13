//! Edge case тесты для Lazy Lookup методов по фасетам
//!
//! Файл: shared/tests/metadata_lookup_facet_edge_cases_tests.rs
//! Тестируемый модуль: shared/src/domain/metadata_lookup.rs
//!
//! Содержит комплексные тесты для:
//! - Примитивных типов с active_facet
//! - Generic типов с фасетами
//! - Union, Dynamic, Nullable, Intersection типов
//! - Множественных фасетов для одной конфигурации
//! - Fallback механизма

#![allow(clippy::vec_init_then_push)]

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, ConfigurationType, FacetKind, GenericType, MetadataKind, PlatformType,
    PrimitiveType, RawDataSource, RawMethodData, RawTypeData, ResolutionMetadata, ResolutionResult,
    ResolutionSource, TabularRowType, TypeResolution, WeightedType,
};
use std::sync::Arc;

// === Helper функции ===

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
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "НайтиПоНомеру".to_string(),
                english_name: "FindByNumber".to_string(),
                return_type: "ДокументСсылка".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Manager],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
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
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "Провести".to_string(),
                english_name: "Post".to_string(),
                return_type: "".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    });

    // ДокументСсылка
    platform_types.push(RawTypeData {
        name: "ДокументСсылка".to_string(),
        english_name: "DocumentRef".to_string(),
        description: "Ссылка на документ".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "ПолучитьОбъект".to_string(),
            english_name: "GetObject".to_string(),
            return_type: "ДокументОбъект".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::Reference],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    });

    // ДокументВыборка
    platform_types.push(RawTypeData {
        name: "ДокументВыборка".to_string(),
        english_name: "DocumentSelection".to_string(),
        description: "Выборка документов".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Следующий".to_string(),
            english_name: "Next".to_string(),
            return_type: "Булево".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::Selection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    });

    // ДокументСписок
    platform_types.push(RawTypeData {
        name: "ДокументСписок".to_string(),
        english_name: "DocumentList".to_string(),
        description: "Список документов в форме".to_string(),
        category: "Документ".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Обновить".to_string(),
            english_name: "Refresh".to_string(),
            return_type: "".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::List],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
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
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "НайтиПоКоду".to_string(),
                english_name: "FindByCode".to_string(),
                return_type: "СправочникСсылка".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Manager],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    });

    // СправочникОбъект
    platform_types.push(RawTypeData {
        name: "СправочникОбъект".to_string(),
        english_name: "CatalogObject".to_string(),
        description: "Объект справочника".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Записать".to_string(),
            english_name: "Write".to_string(),
            return_type: "".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
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
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "Пустая".to_string(),
                english_name: "IsEmpty".to_string(),
                return_type: "Булево".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Reference],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    });

    // СправочникВыборка
    platform_types.push(RawTypeData {
        name: "СправочникВыборка".to_string(),
        english_name: "CatalogSelection".to_string(),
        description: "Выборка справочника".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Следующий".to_string(),
            english_name: "Next".to_string(),
            return_type: "Булево".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::Selection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    });

    // СправочникСписок
    platform_types.push(RawTypeData {
        name: "СправочникСписок".to_string(),
        english_name: "CatalogList".to_string(),
        description: "Список справочника в форме".to_string(),
        category: "Справочник".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Обновить".to_string(),
            english_name: "Refresh".to_string(),
            return_type: "".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::List],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    });

    repo.load_types(platform_types)
        .expect("Failed to load platform types");
    repo
}

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

// === Edge cases: Platform типы с active_facet (примитивные типы) ===

#[test]
fn test_primitive_type_string_with_active_facet() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Примитивный тип Строка с active_facet
    let resolution = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "Строка".to_string(),
        })),
        active_facet: Some(FacetKind::Collection),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);

    // Примитивные типы игнорируют active_facet в lazy lookup
    // Используется fallback логика → вернуть пустой массив (Строка нет в репозитории)
    assert!(
        methods.is_empty(),
        "Примитивный тип должен использовать fallback"
    );
}

#[test]
fn test_primitive_type_number_returns_empty() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
        active_facet: None,
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);
    assert!(
        methods.is_empty(),
        "Примитивный тип Число должен вернуть пустой массив"
    );
}

// === Integration тесты: множественные фасеты ===

#[test]
fn test_same_config_type_different_facets_returns_different_methods() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Одна конфигурация (Документы.ЗаказНаряды), но разные фасеты
    let resolution_manager = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Manager,
    );

    let resolution_object =
        create_resolution_for_config_type(MetadataKind::Document, "ЗаказНаряды", FacetKind::Object);

    let methods_manager = lookup.get_methods(&resolution_manager);
    let methods_object = lookup.get_methods(&resolution_object);

    // Оба фасета имеют методы
    assert!(!methods_manager.is_empty(), "Manager должен иметь методы");
    assert!(!methods_object.is_empty(), "Object должен иметь методы");

    // Конкретные методы отличаются (это главное!)
    assert!(methods_manager.iter().any(|m| m.name == "СоздатьДокумент"));
    assert!(!methods_object.iter().any(|m| m.name == "СоздатьДокумент"));

    assert!(methods_object.iter().any(|m| m.name == "Записать"));
    assert!(!methods_manager.iter().any(|m| m.name == "Записать"));
}

#[test]
fn test_catalog_and_document_same_facet_returns_different_methods() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Разные metadata kinds (Документ vs Справочник), одинаковый фасет (Manager)
    let resolution_document = create_resolution_for_config_type(
        MetadataKind::Document,
        "ЗаказНаряды",
        FacetKind::Manager,
    );

    let resolution_catalog =
        create_resolution_for_config_type(MetadataKind::Catalog, "Контрагенты", FacetKind::Manager);

    let methods_document = lookup.get_methods(&resolution_document);
    let methods_catalog = lookup.get_methods(&resolution_catalog);

    // Методы должны быть РАЗНЫМИ (разные платформенные типы)
    assert_eq!(methods_document.len(), 2);
    assert_eq!(methods_catalog.len(), 2);

    // Конкретные методы отличаются
    assert!(methods_document.iter().any(|m| m.name == "СоздатьДокумент"));
    assert!(!methods_catalog.iter().any(|m| m.name == "СоздатьДокумент"));

    assert!(methods_catalog.iter().any(|m| m.name == "СоздатьЭлемент"));
    assert!(!methods_document.iter().any(|m| m.name == "СоздатьЭлемент"));
}

// === Integration тесты: Generic типы + facets ===

#[test]
fn test_generic_type_ignores_facet_lazy_lookup() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Generic тип с active_facet должен игнорировать facet
    // и использовать методы базового Generic типа
    let row_type = TabularRowType::new(
        "Документы.ЗаказНаряды".to_string(),
        "Работы".to_string(),
        vec![],
    );

    let generic_type = GenericType {
        base_type: "ТабличнаяЧасть".to_string(),
        type_params: vec![ConcreteType::TabularRow(row_type)],
    };

    let resolution = TypeResolution {
        result: ResolutionResult::Generic(generic_type),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        active_facet: Some(FacetKind::Manager), // ← active_facet, но это Generic!
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);

    // Generic типы имеют приоритет в get_methods()
    // Не используют lazy lookup через facet
    // Вместо этого используют get_methods_for_generic()
    // Для ТабличнаяЧасть нет в репозитории → пустой массив
    assert!(methods.is_empty(), "Generic тип игнорирует lazy lookup");
}

// === Edge case: Union типы ===

#[test]
fn test_union_type_returns_empty() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Union тип не может иметь active_facet
    let resolution = TypeResolution {
        result: ResolutionResult::Union(vec![
            WeightedType {
                type_: ConcreteType::Platform(PlatformType {
                    name: "Строка".to_string(),
                }),
                weight: 0.5,
            },
            WeightedType {
                type_: ConcreteType::Platform(PlatformType {
                    name: "Число".to_string(),
                }),
                weight: 0.5,
            },
        ]),
        certainty: Certainty::InferredWeak,
        source: ResolutionSource::Inferred,
        active_facet: None,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);
    assert!(methods.is_empty(), "Union тип должен вернуть пустой массив");
}

// === Edge case: Dynamic тип ===

#[test]
fn test_dynamic_type_returns_empty() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = TypeResolution {
        result: ResolutionResult::Dynamic,
        certainty: Certainty::Unknown,
        source: ResolutionSource::Inferred,
        active_facet: None,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);
    assert!(
        methods.is_empty(),
        "Dynamic тип должен вернуть пустой массив"
    );
}

// === Edge case: Nullable типы ===

#[test]
fn test_nullable_configuration_type_with_facet() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Nullable конфигурационный тип с active_facet
    let resolution = TypeResolution {
        result: ResolutionResult::Nullable(Box::new(ConcreteType::Configuration(
            ConfigurationType {
                kind: MetadataKind::Document,
                name: "ЗаказНаряды".to_string(),
                facet: Some(FacetKind::Manager),
                attributes: vec![],
                tabular_sections: vec![],
            },
        ))),
        certainty: Certainty::Inferred,
        source: ResolutionSource::Inferred,
        active_facet: Some(FacetKind::Manager),
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);

    // Nullable типы не распаковываются в get_methods
    // Используется fallback логика → пустой массив
    assert!(
        methods.is_empty(),
        "Nullable тип должен использовать fallback"
    );
}

// === Edge case: Intersection типы ===

#[test]
fn test_intersection_type_returns_empty() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = TypeResolution {
        result: ResolutionResult::Intersection(vec![
            ConcreteType::Platform(PlatformType {
                name: "Массив".to_string(),
            }),
            ConcreteType::Platform(PlatformType {
                name: "Соответствие".to_string(),
            }),
        ]),
        certainty: Certainty::InferredWeak,
        source: ResolutionSource::Inferred,
        active_facet: None,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);
    assert!(
        methods.is_empty(),
        "Intersection тип должен вернуть пустой массив"
    );
}

// === Integration тест: все 10 комбинаций возвращают методы ===

#[test]
fn test_all_10_facet_combinations_return_unique_methods() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Тестируем все 10 комбинаций
    let combinations = vec![
        (
            MetadataKind::Document,
            FacetKind::Manager,
            "ДокументМенеджер",
        ),
        (MetadataKind::Document, FacetKind::Object, "ДокументОбъект"),
        (
            MetadataKind::Document,
            FacetKind::Reference,
            "ДокументСсылка",
        ),
        (
            MetadataKind::Document,
            FacetKind::Selection,
            "ДокументВыборка",
        ),
        (MetadataKind::Document, FacetKind::List, "ДокументСписок"),
        (
            MetadataKind::Catalog,
            FacetKind::Manager,
            "СправочникМенеджер",
        ),
        (MetadataKind::Catalog, FacetKind::Object, "СправочникОбъект"),
        (
            MetadataKind::Catalog,
            FacetKind::Reference,
            "СправочникСсылка",
        ),
        (
            MetadataKind::Catalog,
            FacetKind::Selection,
            "СправочникВыборка",
        ),
        (MetadataKind::Catalog, FacetKind::List, "СправочникСписок"),
    ];

    for (kind, facet, expected_type_name) in combinations {
        let resolution = create_resolution_for_config_type(kind, "TestEntity", facet);

        let methods = lookup.get_methods(&resolution);
        assert!(
            !methods.is_empty(),
            "Комбинация ({:?}, {:?}) должна вернуть методы для {}",
            kind,
            facet,
            expected_type_name
        );
    }
}

// === Test: Active facet vs available facets ===

#[test]
fn test_active_facet_determines_lazy_lookup_not_available_facets() {
    let repo = create_test_repository_with_all_platform_facets();
    let lookup = TypeMetadataLookup::new(repo.clone());

    let resolution = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Document,
            name: "ЗаказНаряды".to_string(),
            facet: Some(FacetKind::Manager),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Manager), // <- определяет lookup
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference], // <- не используются для lazy lookup
    };

    let methods = lookup.get_methods(&resolution);

    // Должны вернуться методы только для Manager фасета
    assert!(
        methods.iter().any(|m| m.name == "СоздатьДокумент"),
        "Manager методы"
    );
    assert!(
        !methods.iter().any(|m| m.name == "Записать"),
        "Object методы не должны быть"
    );
}

// === Test: Platform тип напрямую с facet (игнорируется) ===

#[test]
fn test_platform_array_type_ignores_facet_in_lazy_lookup() {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Добавляем тип "Массив" в репозиторий
    repo.load_types(vec![RawTypeData {
        name: "Массив".to_string(),
        english_name: "Array".to_string(),
        description: "Массив элементов".to_string(),
        category: "Collection".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Добавить".to_string(),
            english_name: "Add".to_string(),
            return_type: "".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::Collection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
        metadata_path: None,
    }])
    .unwrap();

    let lookup = TypeMetadataLookup::new(repo);

    let resolution = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        })),
        active_facet: Some(FacetKind::Manager), // ← несоответствующий facet
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);

    // Platform типы не используют lazy lookup (нет ConfigurationType)
    // Используется fallback → вернуть методы "Массив"
    assert_eq!(methods.len(), 1);
    assert!(methods.iter().any(|m| m.name == "Добавить"));
}
