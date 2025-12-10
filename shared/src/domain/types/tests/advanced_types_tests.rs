//! Tests for Milestone 2.3: Advanced Type System

use crate::domain::types::*;

// === TypeResolution Constructor Tests (Phase 2.1) ===

#[test]
fn test_explicit_constructor() {
    let explicit_str = TypeResolution::explicit("Строка");
    assert_eq!(explicit_str.type_name(), "Строка");
    assert!(matches!(explicit_str.certainty, Certainty::Known));
    assert!(matches!(explicit_str.source, ResolutionSource::Static));

    let explicit_num = TypeResolution::explicit("Число");
    assert_eq!(explicit_num.type_name(), "Число");
}

#[test]
fn test_generic_constructor_with_known_type() {
    let array = TypeResolution::generic("Массив", &["Строка"], 1.0);
    assert_eq!(array.type_name(), "Массив<Строка>");
    assert!(matches!(array.certainty, Certainty::Known));
}

#[test]
fn test_generic_constructor_with_inferred_certainty() {
    let array = TypeResolution::generic("Массив", &["Число"], 0.8);
    assert_eq!(array.type_name(), "Массив<Число>");
    assert!(matches!(array.certainty, Certainty::Inferred(c) if (c - 0.8).abs() < 0.001));
}

#[test]
fn test_generic_constructor_with_unknown_param() {
    let array = TypeResolution::generic("Массив", &["?"], 0.0);
    assert_eq!(array.type_name(), "Массив<Неопределено>");
    assert!(matches!(array.certainty, Certainty::Inferred(c) if (c - 0.0).abs() < 0.001));
}

#[test]
fn test_generic_constructor_with_two_params() {
    let map = TypeResolution::generic("Соответствие", &["Строка", "Число"], 0.9);
    assert_eq!(map.type_name(), "Соответствие<Строка, Число>");
}

#[test]
fn test_string_to_concrete_primitives() {
    // Test via generic constructor
    let array_str = TypeResolution::generic("Массив", &["String"], 1.0);
    assert_eq!(array_str.type_name(), "Массив<Строка>");

    let array_num = TypeResolution::generic("Массив", &["Number"], 1.0);
    assert_eq!(array_num.type_name(), "Массив<Число>");

    let array_bool = TypeResolution::generic("Массив", &["Boolean"], 1.0);
    assert_eq!(array_bool.type_name(), "Массив<Булево>");

    let array_date = TypeResolution::generic("Массив", &["Date"], 1.0);
    assert_eq!(array_date.type_name(), "Массив<Дата>");
}

#[test]
fn test_string_to_concrete_platform_type() {
    let array = TypeResolution::generic("Массив", &["ТаблицаЗначений"], 1.0);
    assert_eq!(array.type_name(), "Массив<ТаблицаЗначений>");
}

// === Task 1: Union Types Tests ===

#[test]
fn test_union_normalization_deduplicate() {
    // String | String -> String
    let types = vec![
        WeightedType::new(ConcreteType::string()),
        WeightedType::new(ConcreteType::string()),
    ];

    let result = ResolutionResult::normalize_union(types);

    assert!(matches!(result, ResolutionResult::Concrete(_)));
    if let ResolutionResult::Concrete(ct) = result {
        assert!(matches!(ct, ConcreteType::Primitive(PrimitiveType::String)));
    }
}

#[test]
fn test_union_normalization_sort_by_weight() {
    // Number(0.3) | String(0.7) -> String | Number (sorted by weight)
    let types = vec![
        WeightedType::with_weight(ConcreteType::number(), 0.3),
        WeightedType::with_weight(ConcreteType::string(), 0.7),
    ];

    let result = ResolutionResult::normalize_union(types);

    if let ResolutionResult::Union(normalized) = result {
        assert_eq!(normalized.len(), 2);
        // First should be String (higher weight)
        assert!(matches!(
            normalized[0].type_,
            ConcreteType::Primitive(PrimitiveType::String)
        ));
        assert_eq!(normalized[0].weight, 0.7);
        // Second should be Number (lower weight)
        assert!(matches!(
            normalized[1].type_,
            ConcreteType::Primitive(PrimitiveType::Number)
        ));
        assert_eq!(normalized[1].weight, 0.3);
    } else {
        panic!("Expected Union type, got {:?}", result);
    }
}

#[test]
fn test_union_with_dynamic_returns_dynamic() {
    // String | Dynamic -> Dynamic
    let types = vec![
        WeightedType::new(ConcreteType::string()),
        WeightedType::new(ConcreteType::undefined()),
    ];

    let result = ResolutionResult::normalize_union(types);

    assert!(matches!(result, ResolutionResult::Dynamic));
}

#[test]
fn test_union_merge_weights() {
    // String(0.3) | Number(0.4) | String(0.3) -> String(0.6) | Number(0.4)
    let types = vec![
        WeightedType::with_weight(ConcreteType::string(), 0.3),
        WeightedType::with_weight(ConcreteType::number(), 0.4),
        WeightedType::with_weight(ConcreteType::string(), 0.3),
    ];

    let result = ResolutionResult::normalize_union(types);

    if let ResolutionResult::Union(normalized) = result {
        assert_eq!(normalized.len(), 2);
        // String weight should be merged: 0.3 + 0.3 = 0.6
        let string_type = normalized
            .iter()
            .find(|wt| matches!(wt.type_, ConcreteType::Primitive(PrimitiveType::String)))
            .expect("String type should be present");
        assert_eq!(string_type.weight, 0.6);
    } else {
        panic!("Expected Union type, got {:?}", result);
    }
}

// === Task 2: Intersection Types Tests ===

#[test]
fn test_intersection_deduplicate() {
    let types = vec![ConcreteType::string(), ConcreteType::string()];

    let result = ResolutionResult::intersection(types);

    // Should return Concrete (one type after deduplication)
    assert!(matches!(result, ResolutionResult::Concrete(_)));
}

#[test]
fn test_intersection_multiple_types() {
    let types = vec![ConcreteType::string(), ConcreteType::number()];

    let result = ResolutionResult::intersection(types);

    if let ResolutionResult::Intersection(inter_types) = result {
        assert_eq!(inter_types.len(), 2);
    } else {
        panic!("Expected Intersection type, got {:?}", result);
    }
}

#[test]
fn test_intersection_empty_returns_dynamic() {
    let types: Vec<ConcreteType> = vec![];

    let result = ResolutionResult::intersection(types);

    assert!(matches!(result, ResolutionResult::Dynamic));
}

// === Task 3: Generic Types Tests ===

#[test]
fn test_generic_array() {
    let array = GenericType::array(ConcreteType::string());

    assert_eq!(array.base_type, "Массив");
    assert_eq!(array.type_params.len(), 1);
    assert!(matches!(
        array.type_params[0],
        ConcreteType::Primitive(PrimitiveType::String)
    ));
}

#[test]
fn test_generic_map() {
    let map = GenericType::map(ConcreteType::string(), ConcreteType::number());

    assert_eq!(map.base_type, "Соответствие");
    assert_eq!(map.type_params.len(), 2);
    assert!(matches!(
        map.type_params[0],
        ConcreteType::Primitive(PrimitiveType::String)
    ));
    assert!(matches!(
        map.type_params[1],
        ConcreteType::Primitive(PrimitiveType::Number)
    ));
}

#[test]
fn test_generic_element_type() {
    let array = GenericType::array(ConcreteType::string());

    let element = array.element_type();
    assert!(element.is_some());
    assert!(matches!(
        element.unwrap(),
        ConcreteType::Primitive(PrimitiveType::String)
    ));
}

// === Task 4: Nullable Types Tests ===

#[test]
fn test_nullable_creation() {
    let nullable = ResolutionResult::nullable(ConcreteType::string());

    assert!(matches!(nullable, ResolutionResult::Nullable(_)));
}

#[test]
fn test_is_nullable_true() {
    let nullable = ResolutionResult::nullable(ConcreteType::string());

    assert!(nullable.is_nullable());
}

#[test]
fn test_is_nullable_false() {
    let concrete = ResolutionResult::Concrete(ConcreteType::string());

    assert!(!concrete.is_nullable());
}

#[test]
fn test_is_nullable_union_with_null() {
    let union = ResolutionResult::Union(vec![
        WeightedType::new(ConcreteType::string()),
        WeightedType::new(ConcreteType::null()),
    ]);

    assert!(union.is_nullable());
}

#[test]
fn test_unwrap_nullable() {
    let nullable = ResolutionResult::nullable(ConcreteType::string());

    let inner = nullable.unwrap_nullable();
    assert!(inner.is_some());
    assert!(matches!(
        inner.unwrap(),
        ConcreteType::Primitive(PrimitiveType::String)
    ));
}

#[test]
fn test_weighted_type_creation() {
    let weighted = WeightedType::new(ConcreteType::string());

    assert_eq!(weighted.weight, 1.0);
    assert!(matches!(
        weighted.type_,
        ConcreteType::Primitive(PrimitiveType::String)
    ));
}

#[test]
fn test_weighted_type_with_custom_weight() {
    let weighted = WeightedType::with_weight(ConcreteType::string(), 0.75);

    assert_eq!(weighted.weight, 0.75);
}
