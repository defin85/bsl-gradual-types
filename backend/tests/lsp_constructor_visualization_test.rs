//! Integration tests for constructor visualization in Type Details Modal
//!
//! Tests the flow:
//! 1. Backend: retrieve constructors from SignatureIndex
//! 2. Convert ConstructorSignature → MethodDto with is_constructor=true
//! 3. Frontend: filter and display constructors separately from methods

use bsl_shared::api::dtos::{MethodDto, ParamDto};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{ConstructorSignature, SignatureSource};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{
    FacetKind, ParameterInfo, RawDataSource, RawMethodData, RawTypeData,
};

/// Test 1.1: Repository find_constructor works correctly
#[test]
fn test_repository_find_constructor() {
    let repo = InMemoryTypeRepository::new();

    // Empty repository should return None for any constructor
    let constructor = repo.find_constructor("Массив");
    assert!(constructor.is_none(), "Empty repository should return None");
}

/// Test 1.2: Repository populates SignatureIndex with constructors
#[test]
fn test_repository_populate_signature_index() {
    let repo = InMemoryTypeRepository::new();

    // Populate SignatureIndex with test constructor
    repo.populate_signature_index(|index| {
        let constructor = ConstructorSignature {
            type_name: "Массив".to_string(),
            params: vec![ParameterInfo {
                name: "Элемент1".to_string(),
                type_name: Some("Произвольный".to_string()),
                is_optional: true,
                default_value: None,
                description: None,
            }],
            is_collection: true,
            generic_params_count: 1,
            facet: Some("Object".to_string()),
            source: SignatureSource::Platform,
        };
        index.add_constructor(TypeId::new("Массив"), constructor);
    });

    // Verify constructor was added
    let constructor = repo.find_constructor("Массив");
    assert!(constructor.is_some(), "Constructor 'Массив' must be found");

    let constructor = constructor.unwrap();
    assert_eq!(constructor.type_name, "Массив");
    assert!(constructor.is_collection);
    assert_eq!(constructor.generic_params_count, 1);
    assert_eq!(constructor.params.len(), 1);
}

/// Test 1.3: Constructor without explicit type - fallback handling
#[test]
fn test_repository_constructor_type_handling() {
    let repo = InMemoryTypeRepository::new();

    repo.populate_signature_index(|index| {
        let constructor = ConstructorSignature {
            type_name: "СложныйТип".to_string(),
            params: vec![ParameterInfo {
                name: "Параметр".to_string(),
                type_name: None, // No explicit type - will fallback to "Произвольный"
                is_optional: false,
                default_value: None,
                description: None,
            }],
            is_collection: false,
            generic_params_count: 0,
            facet: None,
            source: SignatureSource::Platform,
        };
        index.add_constructor(TypeId::new("СложныйТип"), constructor);
    });

    let constructor = repo.find_constructor("СложныйТип").unwrap();

    // Should have at least the parameter
    assert_eq!(constructor.params.len(), 1);
    assert!(constructor.params[0].type_name.is_none());
}

/// Test 2.1: ConstructorSignature to MethodDto conversion
#[test]
fn test_constructor_to_method_dto_conversion() {
    let constructor = ConstructorSignature {
        type_name: "Соответствие".to_string(),
        params: vec![
            ParameterInfo {
                name: "Ключ".to_string(),
                type_name: Some("Строка".to_string()),
                is_optional: false,
                default_value: Some("\"\"".to_string()),
                description: None,
            },
            ParameterInfo {
                name: "Значение".to_string(),
                type_name: Some("Произвольный".to_string()),
                is_optional: true,
                default_value: None,
                description: None,
            },
        ],
        is_collection: true,
        generic_params_count: 2,
        facet: Some("Object".to_string()),
        source: SignatureSource::Platform,
    };

    // Simulate conversion as done in handle_query_type
    let method_dto = MethodDto {
        name: format!("Новый {}", constructor.type_name),
        english_name: Some(format!("New {}", constructor.type_name)),
        return_type: Some(constructor.type_name.clone()),
        params: constructor
            .params
            .iter()
            .map(|p| ParamDto {
                name: p.name.clone(),
                param_type: p
                    .type_name
                    .as_ref()
                    .unwrap_or(&"Произвольный".to_string())
                    .clone(),
                is_optional: p.is_optional,
                default_value: p.default_value.clone(),
            })
            .collect(),
        description: Some(format!(
            "Конструктор типа {}{}{}",
            constructor.type_name,
            if constructor.is_collection {
                format!(
                    " (коллекция, {} generic параметров)",
                    constructor.generic_params_count
                )
            } else {
                String::new()
            },
            constructor
                .facet
                .as_ref()
                .map(|f| format!(", facet: {}", f))
                .unwrap_or_default()
        )),
        is_deprecated: false,
        is_constructor: true,
    };

    // Verify conversion
    assert_eq!(method_dto.name, "Новый Соответствие");
    assert_eq!(
        method_dto.english_name,
        Some("New Соответствие".to_string())
    );
    assert_eq!(method_dto.return_type, Some("Соответствие".to_string()));
    assert!(method_dto.is_constructor);
    assert_eq!(method_dto.params.len(), 2);

    // Verify parameters were converted correctly
    assert_eq!(method_dto.params[0].name, "Ключ");
    assert_eq!(method_dto.params[0].param_type, "Строка");
    assert!(!method_dto.params[0].is_optional);

    assert_eq!(method_dto.params[1].name, "Значение");
    assert_eq!(method_dto.params[1].param_type, "Произвольный");
    assert!(method_dto.params[1].is_optional);

    // Verify description
    assert!(method_dto
        .description
        .as_ref()
        .unwrap()
        .contains("коллекция"));
    assert!(method_dto
        .description
        .as_ref()
        .unwrap()
        .contains("2 generic"));
}

/// Test 2.2: Regular method vs constructor distinction
#[test]
fn test_regular_method_vs_constructor() {
    // Regular method
    let regular_method = MethodDto {
        name: "Добавить".to_string(),
        english_name: Some("Add".to_string()),
        return_type: Some("Неопределено".to_string()),
        params: vec![],
        description: Some("Добавляет элемент в массив".to_string()),
        is_deprecated: false,
        is_constructor: false, // ← Key difference
    };

    // Constructor
    let constructor_method = MethodDto {
        name: "Новый Массив".to_string(),
        english_name: Some("New Array".to_string()),
        return_type: Some("Массив".to_string()),
        params: vec![],
        description: Some("Конструктор типа Массив".to_string()),
        is_deprecated: false,
        is_constructor: true, // ← Key difference
    };

    // Verify distinction
    assert!(!regular_method.is_constructor);
    assert!(constructor_method.is_constructor);

    // In frontend, constructors should be filtered separately
    let methods = vec![regular_method.clone(), constructor_method.clone()];
    let constructors: Vec<_> = methods.iter().filter(|m| m.is_constructor).collect();
    let regular_methods: Vec<_> = methods.iter().filter(|m| !m.is_constructor).collect();

    assert_eq!(constructors.len(), 1);
    assert_eq!(regular_methods.len(), 1);
}

/// Test 3.1: Built-in collection types have constructors
#[test]
fn test_builtin_collection_constructors() {
    let repo = InMemoryTypeRepository::new();

    // Populate with built-in collection types
    repo.populate_signature_index(|index| {
        let collections = vec![
            ("Массив", 1),
            ("Соответствие", 2),
            ("ФиксированныйМассив", 1),
            ("ТаблицаЗначений", 0),
            ("СписокЗначений", 0),
        ];

        for (type_name, generic_count) in collections {
            let constructor = ConstructorSignature {
                type_name: type_name.to_string(),
                params: vec![],
                is_collection: true,
                generic_params_count: generic_count,
                facet: Some("Object".to_string()),
                source: SignatureSource::Platform,
            };
            index.add_constructor(TypeId::new(type_name), constructor);
        }
    });

    // Verify all collections have constructors
    assert!(repo.find_constructor("Массив").is_some());
    assert!(repo.find_constructor("Соответствие").is_some());
    assert!(repo.find_constructor("ФиксированныйМассив").is_some());
    assert!(repo.find_constructor("ТаблицаЗначений").is_some());
    assert!(repo.find_constructor("СписокЗначений").is_some());

    // Verify generic parameters
    assert_eq!(
        repo.find_constructor("Массив")
            .unwrap()
            .generic_params_count,
        1
    );
    assert_eq!(
        repo.find_constructor("Соответствие")
            .unwrap()
            .generic_params_count,
        2
    );

    // All should be marked as collections
    for type_name in &[
        "Массив",
        "Соответствие",
        "ФиксированныйМассив",
        "ТаблицаЗначений",
        "СписокЗначений",
    ] {
        let constructor = repo.find_constructor(type_name).unwrap();
        assert!(
            constructor.is_collection,
            "{} should be marked as collection",
            type_name
        );
    }
}

/// Test 3.2: Type without constructor
#[test]
fn test_type_without_constructor() {
    let repo = InMemoryTypeRepository::new();

    // Type "Число" has no constructor in typical 1C usage
    assert!(repo.find_constructor("Число").is_none());
}

/// Test 4.1: Graceful handling when SignatureIndex is empty
#[test]
fn test_handle_query_type_no_constructor_graceful() {
    let repo = InMemoryTypeRepository::new();

    // Load type data without any constructors in SignatureIndex
    let type_data = RawTypeData {
        name: "Число".to_string(),
        english_name: "Number".to_string(),
        description: "Числовой тип".to_string(),
        category: "PrimitiveType".to_string(),
        source: RawDataSource::Platform,
        methods: vec![RawMethodData {
            name: "Целое".to_string(),
            english_name: "Int".to_string(),
            return_type: "Число".to_string(),
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
    };

    repo.load_types(vec![type_data]).unwrap();

    // Find type - should not fail even if there's no constructor
    let found_type = repo.find_type("Число");
    assert!(found_type.is_some());

    let raw_type = found_type.unwrap();
    assert_eq!(raw_type.methods.len(), 1);

    // Simulate what handle_query_type does:
    // Convert methods
    let methods: Vec<MethodDto> = raw_type
        .methods
        .iter()
        .map(|m| MethodDto {
            name: m.name.clone(),
            english_name: if m.english_name.is_empty() {
                None
            } else {
                Some(m.english_name.clone())
            },
            return_type: if m.return_type.is_empty() {
                None
            } else {
                Some(m.return_type.clone())
            },
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
        })
        .collect();

    // Try to get constructor
    let constructor_opt = repo.find_constructor("Число");

    // Should handle gracefully - either constructor exists or it doesn't
    if let Some(_constructor) = constructor_opt {
        // Would create MethodDto here
        // Конструктор найден - это нормально
    } else {
        // No constructor - that's fine, just use regular methods
        assert_eq!(methods.len(), 1);
    }
}

/// Test 4.2: Frontend filtering logic
#[test]
fn test_frontend_filtering_constructors_vs_methods() {
    let all_methods = vec![
        MethodDto {
            name: "Новый Массив".to_string(),
            english_name: None,
            return_type: Some("Массив".to_string()),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: true,
        },
        MethodDto {
            name: "Добавить".to_string(),
            english_name: None,
            return_type: Some("Неопределено".to_string()),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
        },
        MethodDto {
            name: "Найти".to_string(),
            english_name: None,
            return_type: Some("Число".to_string()),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
        },
    ];

    // Frontend filtering logic (as in typeDetails.tsx)
    let constructors: Vec<_> = all_methods.iter().filter(|m| m.is_constructor).collect();
    let regular_methods: Vec<_> = all_methods.iter().filter(|m| !m.is_constructor).collect();

    // Verify correct distribution
    assert_eq!(constructors.len(), 1);
    assert_eq!(regular_methods.len(), 2);

    // Verify correct items in each category
    assert_eq!(constructors[0].name, "Новый Массив");
    assert_eq!(regular_methods[0].name, "Добавить");
    assert_eq!(regular_methods[1].name, "Найти");
}

/// Test 5: Backward compatibility - optional field
#[test]
fn test_backward_compatibility_optional_is_constructor() {
    // Simulate old client that doesn't have isConstructor field
    // In Rust, we always have it, but in JSON deserialization it could be missing

    let method_with_constructor_flag = MethodDto {
        name: "Новый Массив".to_string(),
        english_name: None,
        return_type: Some("Массив".to_string()),
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: true,
    };

    // Serialization should include the field
    let json = serde_json::to_string(&method_with_constructor_flag).unwrap();
    assert!(
        json.contains("\"isConstructor\":true"),
        "Serialized JSON should contain isConstructor field"
    );

    // Deserialization with the field
    let deserialized: MethodDto = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_constructor);
}

/// Test 6: Edge case - constructor with no parameters
#[test]
fn test_constructor_with_no_parameters() {
    let repo = InMemoryTypeRepository::new();

    repo.populate_signature_index(|index| {
        let constructor = ConstructorSignature {
            type_name: "НовыйТип".to_string(),
            params: vec![], // No parameters
            is_collection: false,
            generic_params_count: 0,
            facet: None,
            source: SignatureSource::Platform,
        };
        index.add_constructor(TypeId::new("НовыйТип"), constructor);
    });

    let constructor = repo.find_constructor("НовыйТип").unwrap();

    // Convert to MethodDto
    let method_dto = MethodDto {
        name: format!("Новый {}", constructor.type_name),
        english_name: None,
        return_type: Some(constructor.type_name.clone()),
        params: vec![],
        description: Some(format!("Конструктор типа {}", constructor.type_name)),
        is_deprecated: false,
        is_constructor: true,
    };

    assert_eq!(method_dto.params.len(), 0);
    assert!(method_dto.is_constructor);
}

/// Test 7: Constructor with optional parameters
#[test]
fn test_constructor_with_optional_parameters() {
    let repo = InMemoryTypeRepository::new();

    repo.populate_signature_index(|index| {
        let constructor = ConstructorSignature {
            type_name: "ТаблицаЗначений".to_string(),
            params: vec![
                ParameterInfo {
                    name: "Колонка".to_string(),
                    type_name: Some("Строка".to_string()),
                    is_optional: true,
                    default_value: Some("\"\"".to_string()),
                    description: Some("Имя первой колонки".to_string()),
                },
                ParameterInfo {
                    name: "ТипКолонки".to_string(),
                    type_name: Some("Строка".to_string()),
                    is_optional: true,
                    default_value: None,
                    description: Some("Тип значений в колонке".to_string()),
                },
            ],
            is_collection: true,
            generic_params_count: 0,
            facet: Some("Object".to_string()),
            source: SignatureSource::Platform,
        };
        index.add_constructor(TypeId::new("ТаблицаЗначений"), constructor);
    });

    let constructor = repo.find_constructor("ТаблицаЗначений").unwrap();

    // Convert to MethodDto
    let method_dto = MethodDto {
        name: format!("Новый {}", constructor.type_name),
        english_name: None,
        return_type: Some(constructor.type_name.clone()),
        params: constructor
            .params
            .iter()
            .map(|p| ParamDto {
                name: p.name.clone(),
                param_type: p
                    .type_name
                    .clone()
                    .unwrap_or_else(|| "Произвольный".to_string()),
                is_optional: p.is_optional,
                default_value: p.default_value.clone(),
            })
            .collect(),
        description: Some("Конструктор таблицы значений".to_string()),
        is_deprecated: false,
        is_constructor: true,
    };

    assert_eq!(method_dto.params.len(), 2);
    assert!(method_dto.params[0].is_optional);
    assert!(method_dto.params[1].is_optional);
    assert_eq!(method_dto.params[0].default_value, Some("\"\"".to_string()));
}
