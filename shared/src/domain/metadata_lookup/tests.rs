//! Тесты для TypeMetadataLookup.

use super::TypeMetadataLookup;
use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
use crate::domain::type_id::TypeId;
use crate::domain::types::{
    Certainty, ConcreteType, ConfigurationType, FacetKind, GenericType, MetadataKind, PlatformType,
    RawAttributeData, RawDataSource, RawMethodData, RawParamData, RawTabularSectionData,
    RawTypeData, ResolutionMetadata, ResolutionResult, ResolutionSource, TabularRowType,
    TypeResolution,
};
use std::sync::Arc;

fn create_test_repository() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Создаем тестовый тип "Массив" с методами
    let array_type = RawTypeData {
        name: "Массив".to_string(),
        english_name: "Array".to_string(),
        description: "Коллекция элементов".to_string(),
        category: "Типы коллекций".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Добавить".to_string(),
                english_name: "Add".to_string(),
                return_type: "".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "Количество".to_string(),
                english_name: "Count".to_string(),
                return_type: "Число".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Collection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![array_type]).unwrap();
    repo
}

#[test]
fn test_generic_collection_item_type_parameterized_in_return_type() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    repo.load_types(vec![RawTypeData {
        name: "ДанныеФормыКоллекция".to_string(),
        english_name: "DataFormCollection".to_string(),
        description: "".to_string(),
        category: "Platform".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Получить".to_string(),
            english_name: "Get".to_string(),
            return_type: "ДанныеФормыЭлементКоллекции".to_string(),
            params: vec![RawParamData {
                name: "Индекс".to_string(),
                param_type: "Число".to_string(),
                is_optional: false,
                default_value: None,
            }],
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
        collection_item_type: Some("ДанныеФормыЭлементКоллекции".to_string()),
        module_paths: None,
    }])
    .unwrap();

    let lookup = TypeMetadataLookup::new(repo);
    let methods = lookup.get_methods_for_generic(&GenericType {
        base_type: "ДанныеФормыКоллекция".to_string(),
        type_params: vec![ConcreteType::Platform(PlatformType {
            name: "СтрокаРаботы".to_string(),
        })],
    });

    let get = methods
        .iter()
        .find(|m| m.name == "Получить")
        .expect("Should have method Получить");
    assert_eq!(get.return_type, "ДанныеФормыЭлементКоллекции<СтрокаРаботы>");
}

fn create_test_resolution(type_name: &str) -> TypeResolution {
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: type_name.to_string(),
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

#[test]
fn test_get_raw_type() {
    let repo = create_test_repository();
    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("Массив");

    let raw_type = lookup.get_raw_type(&resolution);
    assert!(raw_type.is_some());
    let raw = raw_type.unwrap();
    assert_eq!(raw.name, "Массив");
    assert_eq!(raw.english_name, "Array");
}

#[test]
fn test_get_methods() {
    let repo = create_test_repository();
    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("Массив");

    let methods = lookup.get_methods(&resolution);
    assert_eq!(methods.len(), 2);
    assert_eq!(methods[0].name, "Добавить");
    assert_eq!(methods[1].name, "Количество");
}

#[test]
fn test_has_member_existing() {
    let repo = create_test_repository();
    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("Массив");

    assert!(lookup.has_member(&resolution, "Добавить"));
    assert!(lookup.has_member(&resolution, "Количество"));
    // Проверка английского имени
    assert!(lookup.has_member(&resolution, "Add"));
}

#[test]
fn test_has_member_nonexistent() {
    let repo = create_test_repository();
    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("Массив");

    assert!(!lookup.has_member(&resolution, "НеСуществующийМетод"));
}

#[test]
fn test_get_description() {
    let repo = create_test_repository();
    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("Массив");

    let description = lookup.get_description(&resolution);
    assert_eq!(description, "Коллекция элементов");
}

#[test]
fn test_unknown_type_returns_empty() {
    let repo = create_test_repository();
    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("НеСуществующийТип");

    assert!(lookup.get_raw_type(&resolution).is_none());
    assert!(lookup.get_methods(&resolution).is_empty());
    assert!(!lookup.has_member(&resolution, "Метод"));
    assert_eq!(lookup.get_description(&resolution), "");
}

// === Тесты для Generic типов ===

/// Вспомогательная функция для создания тестового репозитория с Generic типами
fn create_test_repository_with_generic_types() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Создаём платформенный тип "ТабличнаяЧасть" с Generic методами
    let tabular_type = RawTypeData {
        name: "ТабличнаяЧасть".to_string(),
        english_name: "TabularSection".to_string(),
        category: "PlatformType".to_string(),
        description: "Табличная часть с Generic методами".to_string(),
        source: RawDataSource::Platform,
        facets: vec![FacetKind::Collection],
        methods: vec![
            RawMethodData {
                name: "Добавить".to_string(),
                english_name: "Add".to_string(),
                return_type: "T".to_string(), // <- Generic!
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "Получить".to_string(),
                english_name: "Get".to_string(),
                return_type: "T".to_string(), // <- Generic!
                params: vec![RawParamData {
                    name: "Индекс".to_string(),
                    param_type: "Число".to_string(),
                    is_optional: false,
                    default_value: None,
                }],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "Количество".to_string(),
                english_name: "Count".to_string(),
                return_type: "Число".to_string(), // НЕ Generic
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
            RawMethodData {
                name: "Индекс".to_string(),
                english_name: "IndexOf".to_string(),
                return_type: "Число".to_string(),
                params: vec![RawParamData {
                    name: "Строка".to_string(),
                    param_type: "T".to_string(), // <- Generic параметр!
                    is_optional: false,
                    default_value: None,
                }],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
        ],
        properties: vec![],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![tabular_type]).unwrap();
    repo
}

#[test]
fn test_generic_method_return_type_substitution() {
    let repo = create_test_repository_with_generic_types();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Создаём Generic тип: ТабличнаяЧасть<СтрокаРаботы>
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
        active_facet: Some(FacetKind::Collection),
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    // Получаем методы
    let methods = lookup.get_methods(&resolution);

    // Проверяем метод "Добавить": return_type должен быть "СтрокаРаботы"
    let add_method = methods.iter().find(|m| m.name == "Добавить").unwrap();
    assert_eq!(add_method.return_type, "СтрокаРаботы");

    // Проверяем метод "Получить": return_type должен быть "СтрокаРаботы"
    let get_method = methods.iter().find(|m| m.name == "Получить").unwrap();
    assert_eq!(get_method.return_type, "СтрокаРаботы");
}

#[test]
fn test_generic_method_param_type_substitution() {
    let repo = create_test_repository_with_generic_types();
    let lookup = TypeMetadataLookup::new(repo.clone());

    // Создаём Generic тип
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
        active_facet: Some(FacetKind::Collection),
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);

    // Проверяем метод "Индекс": параметр "Строка" должен иметь тип "СтрокаРаботы"
    let index_method = methods.iter().find(|m| m.name == "Индекс").unwrap();
    assert_eq!(index_method.params.len(), 1);
    assert_eq!(index_method.params[0].name, "Строка");
    assert_eq!(index_method.params[0].param_type, "СтрокаРаботы");
}

#[test]
fn test_non_generic_methods_unchanged() {
    let repo = create_test_repository_with_generic_types();
    let lookup = TypeMetadataLookup::new(repo.clone());

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
        active_facet: Some(FacetKind::Collection),
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);

    // Проверяем метод "Количество": return_type должен остаться "Число"
    let count_method = methods.iter().find(|m| m.name == "Количество").unwrap();
    assert_eq!(count_method.return_type, "Число");
}

#[test]
fn test_all_methods_returned() {
    let repo = create_test_repository_with_generic_types();
    let lookup = TypeMetadataLookup::new(repo.clone());

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
        active_facet: Some(FacetKind::Collection),
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    let methods = lookup.get_methods(&resolution);

    // Должны вернуться все 4 метода
    assert_eq!(methods.len(), 4);

    let method_names: Vec<_> = methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"Добавить"));
    assert!(method_names.contains(&"Получить"));
    assert!(method_names.contains(&"Количество"));
    assert!(method_names.contains(&"Индекс"));
}

// === Тесты для Milestone 3.16: MetadataLookup API ===

/// Создаёт репозиторий с конфигурационными типами для тестирования
fn create_test_repository_with_config_types() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Создаём тестовые справочники
    let catalog1 = RawTypeData {
        name: "Справочники.Контрагенты".to_string(),
        english_name: "Catalogs.Contractors".to_string(),
        description: "Справочник контрагентов".to_string(),
        category: "Справочники".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        kind: Some(MetadataKind::Catalog),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    let catalog2 = RawTypeData {
        name: "Справочники.Номенклатура".to_string(),
        english_name: "Catalogs.Products".to_string(),
        description: "Справочник номенклатуры".to_string(),
        category: "Справочники".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        kind: Some(MetadataKind::Catalog),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    let catalog3 = RawTypeData {
        name: "Справочники.Склады".to_string(),
        english_name: "Catalogs.Warehouses".to_string(),
        description: "Справочник складов".to_string(),
        category: "Справочники".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        kind: Some(MetadataKind::Catalog),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    // Создаём тестовый документ
    let document = RawTypeData {
        name: "Документы.ЗаказПокупателя".to_string(),
        english_name: "Documents.CustomerOrder".to_string(),
        description: "Заказ покупателя".to_string(),
        category: "Документы".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        kind: Some(MetadataKind::Document),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![catalog1, catalog2, catalog3, document])
        .unwrap();
    repo
}

#[test]
fn test_exists_metadata_object_found() {
    let repo = create_test_repository_with_config_types();
    let lookup = TypeMetadataLookup::new(repo);

    assert!(lookup.exists_metadata_object(MetadataKind::Catalog, "Контрагенты"));
    assert!(lookup.exists_metadata_object(MetadataKind::Catalog, "Номенклатура"));
    assert!(lookup.exists_metadata_object(MetadataKind::Document, "ЗаказПокупателя"));
}

#[test]
fn test_exists_metadata_object_not_found() {
    let repo = create_test_repository_with_config_types();
    let lookup = TypeMetadataLookup::new(repo);

    assert!(!lookup.exists_metadata_object(MetadataKind::Catalog, "НесуществующийСправочник"));
    assert!(!lookup.exists_metadata_object(MetadataKind::Document, "НесуществующийДокумент"));
    // Неправильный вид метаданных
    assert!(!lookup.exists_metadata_object(MetadataKind::Document, "Контрагенты"));
}

#[test]
fn test_suggest_similar_names_typo() {
    let repo = create_test_repository_with_config_types();
    let lookup = TypeMetadataLookup::new(repo);

    // Опечатка: "Контрогенты" вместо "Контрагенты"
    let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "Контрогенты", 3);

    assert!(!suggestions.is_empty());
    assert!(suggestions.contains(&"Контрагенты".to_string()));
}

#[test]
fn test_suggest_similar_names_no_match() {
    let repo = create_test_repository_with_config_types();
    let lookup = TypeMetadataLookup::new(repo);

    // Совсем непохожее имя
    let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "АбсолютноДругоеИмя", 3);

    // Должен вернуть пустой вектор - слишком большое расстояние
    assert!(suggestions.is_empty());
}

#[test]
fn test_suggest_similar_names_sorting() {
    let repo = create_test_repository_with_config_types();
    let lookup = TypeMetadataLookup::new(repo);

    // "Склад" близко к "Склады" (1 операция)
    let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "Склад", 3);

    assert!(!suggestions.is_empty());
    // Склады должен быть в списке (расстояние = 1)
    assert!(suggestions.contains(&"Склады".to_string()));
}

#[test]
fn test_suggest_similar_names_max_limit() {
    let repo = create_test_repository_with_config_types();
    let lookup = TypeMetadataLookup::new(repo);

    // Ограничение на количество предложений
    let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "Н", 1);

    assert!(suggestions.len() <= 1);
}

#[test]
fn test_is_configuration_loaded_true() {
    let repo = create_test_repository_with_config_types();
    let lookup = TypeMetadataLookup::new(repo);

    assert!(lookup.is_configuration_loaded());
}

#[test]
fn test_is_configuration_loaded_false() {
    // Репозиторий только с платформенными типами
    let repo = create_test_repository();
    let lookup = TypeMetadataLookup::new(repo);

    assert!(!lookup.is_configuration_loaded());
}

#[test]
fn test_get_metadata_objects_by_kind() {
    let repo = create_test_repository_with_config_types();

    let catalogs = repo.get_metadata_objects_by_kind(MetadataKind::Catalog);
    assert_eq!(catalogs.len(), 3);
    assert!(catalogs.contains(&"Контрагенты".to_string()));
    assert!(catalogs.contains(&"Номенклатура".to_string()));
    assert!(catalogs.contains(&"Склады".to_string()));

    let documents = repo.get_metadata_objects_by_kind(MetadataKind::Document);
    assert_eq!(documents.len(), 1);
    assert!(documents.contains(&"ЗаказПокупателя".to_string()));

    // Пустой результат для несуществующего вида
    let enums = repo.get_metadata_objects_by_kind(MetadataKind::Enum);
    assert!(enums.is_empty());
}

// === Тесты для приоритета signature_index над raw types ===

#[test]
fn test_get_methods_prefers_signature_index() {
    use crate::domain::signature_index::{ContextRequirements, MethodSignature, SignatureSource};
    use crate::domain::types::ParameterInfo;

    let repo = Arc::new(InMemoryTypeRepository::new());

    // 1. Создаём тип с методом БЕЗ return_type (как из syntax_helper)
    let manager_type = RawTypeData {
        name: "СправочникМенеджер".to_string(),
        english_name: "CatalogManager".to_string(),
        description: "Менеджер справочника".to_string(),
        category: "Справочники".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "НайтиПоКоду".to_string(),
            english_name: "FindByCode".to_string(),
            return_type: "".to_string(), // Пустой return_type в raw data!
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![FacetKind::Manager],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![manager_type]).unwrap();

    // 2. Добавляем метод в signature_index С return_type (как из platform_types.rs)
    repo.populate_signature_index(|index| {
        let sig = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![ParameterInfo {
                name: "Код".to_string(),
                type_name: Some("Число | Строка".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            Some("СправочникСсылка".to_string()), // Корректный return_type!
            None,
            None,
            SignatureSource::Platform,
            Some(FacetKind::Reference),
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig);
    });

    // 3. Проверяем через TypeMetadataLookup
    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("СправочникМенеджер");

    let methods = lookup.get_methods(&resolution);

    // Должен найти 1 метод
    assert_eq!(methods.len(), 1, "Should find 1 method");

    let method = &methods[0];
    assert_eq!(method.name, "НайтиПоКоду");

    // ГЛАВНАЯ ПРОВЕРКА: return_type должен быть из signature_index, не из raw data!
    assert_eq!(
        method.return_type, "СправочникСсылка",
        "return_type should come from signature_index, not raw data"
    );

    // Проверяем параметры тоже из signature_index
    assert_eq!(
        method.params.len(),
        1,
        "Should have 1 param from signature_index"
    );
    assert_eq!(method.params[0].name, "Код");
    assert_eq!(method.params[0].param_type, "Число | Строка");

    // Проверяем return_facet
    assert_eq!(method.return_facet, Some(FacetKind::Reference));
}

#[test]
fn test_get_methods_fallback_to_raw_when_no_signature_index() {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Создаём тип ТОЛЬКО в raw types (без signature_index)
    let simple_type = RawTypeData {
        name: "ПростойТип".to_string(),
        english_name: "SimpleType".to_string(),
        description: "Тип без signature_index".to_string(),
        category: "Тестовые".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Метод1".to_string(),
            english_name: "Method1".to_string(),
            return_type: "Строка".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        }],
        properties: vec![],
        facets: vec![],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![simple_type]).unwrap();

    // НЕ добавляем в signature_index

    let lookup = TypeMetadataLookup::new(repo.clone());
    let resolution = create_test_resolution("ПростойТип");

    let methods = lookup.get_methods(&resolution);

    // Должен найти метод из raw types (fallback)
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].name, "Метод1");
    assert_eq!(methods[0].return_type, "Строка");
}

// === Тесты для get_tabular_sections ===

/// Создаёт репозиторий с документом, имеющим табличные части
fn create_test_repository_with_tabular_sections() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Создаём документ с двумя табличными частями
    let document = RawTypeData {
        name: "Документы.ЗаказНаряды".to_string(),
        english_name: "Documents.WorkOrders".to_string(),
        description: "Документ заказ-наряды".to_string(),
        category: "Документы".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
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
                name: "Материалы".to_string(),
                attributes: vec![RawAttributeData {
                    name: "Материал".to_string(),
                    attr_type: "СправочникСсылка.Номенклатура".to_string(),
                }],
            },
        ],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    repo.load_types(vec![document]).unwrap();
    repo
}

fn create_config_resolution_with_facet(
    type_name: &str,
    kind: MetadataKind,
    facet: Option<FacetKind>,
) -> TypeResolution {
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind,
            name: type_name.to_string(),
            facet: None,
            attributes: vec![],
            tabular_sections: vec![],
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: facet,
        available_facets: vec![],
    }
}

#[test]
fn test_get_tabular_sections_returns_sections_for_object_facet() {
    let repo = create_test_repository_with_tabular_sections();
    let lookup = TypeMetadataLookup::new(repo);

    // Object фасет - должны вернуться табличные части
    let resolution = create_config_resolution_with_facet(
        "ЗаказНаряды",
        MetadataKind::Document,
        Some(FacetKind::Object),
    );

    let sections = lookup.get_tabular_sections(&resolution);
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].name, "Работы");
    assert_eq!(sections[0].attributes.len(), 2);
    assert_eq!(sections[1].name, "Материалы");
    assert_eq!(sections[1].attributes.len(), 1);
}

#[test]
fn test_get_tabular_sections_returns_sections_for_reference_facet() {
    let repo = create_test_repository_with_tabular_sections();
    let lookup = TypeMetadataLookup::new(repo);

    // Reference фасет - тоже должны вернуться табличные части
    let resolution = create_config_resolution_with_facet(
        "ЗаказНаряды",
        MetadataKind::Document,
        Some(FacetKind::Reference),
    );

    let sections = lookup.get_tabular_sections(&resolution);
    assert_eq!(sections.len(), 2);
}

#[test]
fn test_get_tabular_sections_empty_for_manager_facet() {
    let repo = create_test_repository_with_tabular_sections();
    let lookup = TypeMetadataLookup::new(repo);

    // Manager фасет - табличные части не актуальны
    let resolution = create_config_resolution_with_facet(
        "ЗаказНаряды",
        MetadataKind::Document,
        Some(FacetKind::Manager),
    );

    let sections = lookup.get_tabular_sections(&resolution);
    assert!(sections.is_empty());
}

#[test]
fn test_get_tabular_sections_empty_for_platform_type() {
    let repo = create_test_repository(); // Репозиторий с платформенным типом "Массив"
    let lookup = TypeMetadataLookup::new(repo);

    let resolution = create_test_resolution("Массив");

    let sections = lookup.get_tabular_sections(&resolution);
    assert!(sections.is_empty());
}

#[test]
fn test_get_tabular_sections_without_facet_returns_sections() {
    let repo = create_test_repository_with_tabular_sections();
    let lookup = TypeMetadataLookup::new(repo);

    // Без активного фасета - должны вернуться табличные части
    let resolution = create_config_resolution_with_facet(
        "ЗаказНаряды",
        MetadataKind::Document,
        None, // Нет активного фасета
    );

    let sections = lookup.get_tabular_sections(&resolution);
    assert_eq!(sections.len(), 2);
}
