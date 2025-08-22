//! Тесты системы Union типов

use bsl_gradual_types::core::types::{
    Certainty, ConcreteType, PrimitiveType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use bsl_gradual_types::core::union_types::UnionTypeManager;

fn create_string_type() -> TypeResolution {
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

fn create_number_type() -> TypeResolution {
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

fn create_boolean_type() -> TypeResolution {
    TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean)),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

#[test]
fn test_create_union_from_multiple_types() {
    let types = vec![
        create_string_type(),
        create_number_type(),
        create_boolean_type(),
    ];

    let union = UnionTypeManager::create_union(types);

    match union.result {
        ResolutionResult::Union(union_types) => {
            assert_eq!(union_types.len(), 3);

            // Проверяем что все типы присутствуют
            let type_names: Vec<_> = union_types.iter().map(|wt| &wt.type_).collect();

            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::String)));
            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::Number)));
            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::Boolean)));

            // Проверяем что веса правильно распределены (примерно по 1/3)
            for weighted_type in &union_types {
                assert!((weighted_type.weight - 1.0 / 3.0).abs() < 0.1);
            }
        }
        _ => panic!("Expected Union type, got: {:?}", union.result),
    }
}

#[test]
fn test_simplify_single_type_union() {
    let types = vec![create_string_type()];
    let union = UnionTypeManager::create_union(types);

    // Один тип должен быть упрощен до конкретного типа
    match union.result {
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)) => {
            // OK
        }
        _ => panic!("Expected concrete String type, got: {:?}", union.result),
    }
}

#[test]
fn test_empty_union() {
    let types = vec![];
    let union = UnionTypeManager::create_union(types);

    // Пустой union должен стать "never" типом
    match union.result {
        ResolutionResult::Dynamic => {
            assert_eq!(union.certainty, Certainty::Unknown);
        }
        _ => panic!("Expected Dynamic (never) type, got: {:?}", union.result),
    }
}

#[test]
fn test_union_compatibility() {
    let union_types = vec![
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::String),
            weight: 0.6,
        },
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::Number),
            weight: 0.4,
        },
    ];

    // String совместим с union
    assert!(UnionTypeManager::is_compatible_with_union(
        &create_string_type(),
        &union_types
    ));

    // Number совместим с union
    assert!(UnionTypeManager::is_compatible_with_union(
        &create_number_type(),
        &union_types
    ));

    // Boolean не совместим с union
    assert!(!UnionTypeManager::is_compatible_with_union(
        &create_boolean_type(),
        &union_types
    ));
}

#[test]
fn test_most_likely_type() {
    let union_types = vec![
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::String),
            weight: 0.3,
        },
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::Number),
            weight: 0.7,
        },
    ];

    // Нормализуем типы, чтобы они были правильно отсортированы по весу
    let normalized = UnionTypeManager::normalize_union(union_types);
    let most_likely = UnionTypeManager::get_most_likely_type(&normalized).unwrap();

    // Должен быть самый вероятный тип (с наибольшим весом)
    assert_eq!(*most_likely, ConcreteType::Primitive(PrimitiveType::Number));
}

#[test]
fn test_add_type_to_union() {
    let original_union =
        UnionTypeManager::create_union(vec![create_string_type(), create_number_type()]);

    let extended_union =
        UnionTypeManager::add_type_to_union(&original_union, create_boolean_type());

    match extended_union.result {
        ResolutionResult::Union(union_types) => {
            assert_eq!(union_types.len(), 3);

            let type_names: Vec<_> = union_types.iter().map(|wt| &wt.type_).collect();

            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::String)));
            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::Number)));
            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::Boolean)));
        }
        _ => panic!(
            "Expected Union type after adding, got: {:?}",
            extended_union.result
        ),
    }
}

#[test]
fn test_contains_type() {
    let union_types = vec![
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::String),
            weight: 0.5,
        },
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::Number),
            weight: 0.5,
        },
    ];

    assert!(UnionTypeManager::contains_type(
        &union_types,
        &ConcreteType::Primitive(PrimitiveType::String)
    ));
    assert!(UnionTypeManager::contains_type(
        &union_types,
        &ConcreteType::Primitive(PrimitiveType::Number)
    ));
    assert!(!UnionTypeManager::contains_type(
        &union_types,
        &ConcreteType::Primitive(PrimitiveType::Boolean)
    ));
}

#[test]
fn test_get_type_weight() {
    let union_types = vec![
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::String),
            weight: 0.3,
        },
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::Number),
            weight: 0.7,
        },
    ];

    let string_weight = UnionTypeManager::get_type_weight(
        &union_types,
        &ConcreteType::Primitive(PrimitiveType::String),
    );
    assert!((string_weight - 0.3).abs() < 0.001);

    let number_weight = UnionTypeManager::get_type_weight(
        &union_types,
        &ConcreteType::Primitive(PrimitiveType::Number),
    );
    assert!((number_weight - 0.7).abs() < 0.001);

    let boolean_weight = UnionTypeManager::get_type_weight(
        &union_types,
        &ConcreteType::Primitive(PrimitiveType::Boolean),
    );
    assert_eq!(boolean_weight, 0.0); // Тип отсутствует
}

#[test]
fn test_filter_union() {
    let union_types = vec![
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::String),
            weight: 0.3,
        },
        bsl_gradual_types::core::types::WeightedType {
            type_: ConcreteType::Primitive(PrimitiveType::Number),
            weight: 0.7,
        },
    ];

    // Фильтруем только строковые типы
    let filtered = UnionTypeManager::filter_union(&union_types, |t| {
        matches!(t, ConcreteType::Primitive(PrimitiveType::String))
    });

    assert_eq!(filtered.len(), 1);
    assert_eq!(
        filtered[0].type_,
        ConcreteType::Primitive(PrimitiveType::String)
    );
}

#[test]
fn test_from_concrete_types() {
    let concrete_types = vec![
        ConcreteType::Primitive(PrimitiveType::String),
        ConcreteType::Primitive(PrimitiveType::Number),
    ];

    let union = UnionTypeManager::from_concrete_types(concrete_types);

    match union.result {
        ResolutionResult::Union(union_types) => {
            assert_eq!(union_types.len(), 2);

            let type_names: Vec<_> = union_types.iter().map(|wt| &wt.type_).collect();

            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::String)));
            assert!(type_names.contains(&&ConcreteType::Primitive(PrimitiveType::Number)));
        }
        _ => panic!("Expected Union type, got: {:?}", union.result),
    }
}
