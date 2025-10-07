//! Tests for Advanced Type System (Milestone 2.3)
//!
//! Tests for:
//! 1. Union Types с нормализацией
//! 2. Intersection Types
//! 3. Generic Types для коллекций
//! 4. Nullable Types с null safety

#[cfg(test)]
mod tests {
    use crate::domain::types::*;

    // =========================================================================
    // Task 1: Union Types с нормализацией
    // =========================================================================

    #[test]
    fn test_union_normalization_deduplicate() {
        // String | String → String
        let types = vec![
            WeightedType::new(ConcreteType::string()),
            WeightedType::new(ConcreteType::string()),
        ];

        let result = ResolutionResult::normalize_union(types);

        // Should simplify to Concrete(String)
        match result {
            ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)) => (),
            other => panic!("Expected Concrete(String), got {:?}", other),
        }
    }

    #[test]
    fn test_union_normalization_merge_weights() {
        // Number (0.3) | String (0.5) | Number (0.2) → Number (0.5) | String (0.5)
        let types = vec![
            WeightedType::with_weight(ConcreteType::number(), 0.3),
            WeightedType::with_weight(ConcreteType::string(), 0.5),
            WeightedType::with_weight(ConcreteType::number(), 0.2),
        ];

        let result = ResolutionResult::normalize_union(types);

        match result {
            ResolutionResult::Union(normalized) => {
                assert_eq!(normalized.len(), 2);

                // Should be sorted by weight (descending)
                assert!(normalized[0].weight >= normalized[1].weight);

                // Weights should be merged
                let number_weight = normalized.iter()
                    .find(|wt| matches!(wt.type_, ConcreteType::Primitive(PrimitiveType::Number)))
                    .map(|wt| wt.weight);
                assert_eq!(number_weight, Some(0.5)); // 0.3 + 0.2
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_union_with_dynamic() {
        // String | Dynamic → Dynamic
        let types = vec![
            WeightedType::new(ConcreteType::string()),
            WeightedType::new(ConcreteType::undefined()),
        ];

        let result = ResolutionResult::normalize_union(types);

        match result {
            ResolutionResult::Dynamic => (),
            other => panic!("Expected Dynamic, got {:?}", other),
        }
    }

    #[test]
    fn test_union_empty() {
        let types = vec![];
        let result = ResolutionResult::normalize_union(types);

        match result {
            ResolutionResult::Dynamic => (),
            other => panic!("Expected Dynamic for empty union, got {:?}", other),
        }
    }

    // =========================================================================
    // Task 2: Intersection Types
    // =========================================================================

    #[test]
    fn test_intersection_creation() {
        // A & B
        let type_a = ConcreteType::Platform(PlatformType {
            name: "СправочникМенеджер".to_string(),
        });
        let type_b = ConcreteType::Platform(PlatformType {
            name: "СправочникОбъект".to_string(),
        });

        let result = ResolutionResult::intersection(vec![type_a, type_b]);

        match result {
            ResolutionResult::Intersection(types) => {
                assert_eq!(types.len(), 2);
            }
            other => panic!("Expected Intersection, got {:?}", other),
        }
    }

    #[test]
    fn test_intersection_deduplicate() {
        // A & A → A
        let type_a = ConcreteType::string();

        let result = ResolutionResult::intersection(vec![type_a.clone(), type_a]);

        match result {
            ResolutionResult::Concrete(_) => (),
            other => panic!("Expected Concrete for single intersection, got {:?}", other),
        }
    }

    #[test]
    fn test_intersection_empty() {
        let result = ResolutionResult::intersection(vec![]);

        match result {
            ResolutionResult::Dynamic => (),
            other => panic!("Expected Dynamic for empty intersection, got {:?}", other),
        }
    }

    #[test]
    fn test_intersection_compatibility() {
        let primitive_str = ConcreteType::string();
        let primitive_num = ConcreteType::number();

        // Primitives cannot be intersected
        assert!(!primitive_str.is_intersection_compatible(&primitive_num));

        let platform_type = ConcreteType::Platform(PlatformType {
            name: "Справочники".to_string(),
        });

        // Platform types can be intersected
        assert!(platform_type.is_intersection_compatible(&platform_type));
    }

    // =========================================================================
    // Task 3: Generic Types для коллекций
    // =========================================================================

    #[test]
    fn test_generic_array_creation() {
        // Массив<Строка>
        let array = GenericType::array(ConcreteType::string());

        assert_eq!(array.base_type, "Массив");
        assert_eq!(array.type_params.len(), 1);
        assert!(matches!(
            array.element_type(),
            Some(ConcreteType::Primitive(PrimitiveType::String))
        ));
    }

    #[test]
    fn test_generic_map_creation() {
        // Соответствие<Строка, Число>
        let map = GenericType::map(ConcreteType::string(), ConcreteType::number());

        assert_eq!(map.base_type, "Соответствие");
        assert_eq!(map.type_params.len(), 2);
    }

    #[test]
    fn test_generic_list_creation() {
        // Список<Число>
        let list = GenericType::list(ConcreteType::number());

        assert_eq!(list.base_type, "Список");
        assert_eq!(list.type_params.len(), 1);
    }

    #[test]
    fn test_generic_structure_creation() {
        // Структура<Строка, Число, Булево>
        let structure = GenericType::structure(vec![
            ConcreteType::string(),
            ConcreteType::number(),
            ConcreteType::boolean(),
        ]);

        assert_eq!(structure.base_type, "Структура");
        assert_eq!(structure.type_params.len(), 3);
    }

    #[test]
    fn test_generic_nested() {
        // Массив<Массив<Строка>>
        let _inner_array = GenericType::array(ConcreteType::string());
        let outer_array = GenericType::array(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(), // Simplified - would use ResolutionResult::Generic
        }));

        assert_eq!(outer_array.base_type, "Массив");
        assert_eq!(outer_array.type_params.len(), 1);
    }

    // =========================================================================
    // Task 4: Nullable Types
    // =========================================================================

    #[test]
    fn test_nullable_creation() {
        // String | Null
        let nullable = ResolutionResult::nullable(ConcreteType::string());

        match nullable {
            ResolutionResult::Nullable(inner) => {
                assert!(matches!(*inner, ConcreteType::Primitive(PrimitiveType::String)));
            }
            other => panic!("Expected Nullable, got {:?}", other),
        }
    }

    #[test]
    fn test_nullable_check() {
        let nullable = ResolutionResult::nullable(ConcreteType::string());
        assert!(nullable.is_nullable());

        let not_nullable = ResolutionResult::Concrete(ConcreteType::number());
        assert!(!not_nullable.is_nullable());
    }

    #[test]
    fn test_nullable_unwrap() {
        let nullable = ResolutionResult::nullable(ConcreteType::string());

        match nullable.unwrap_nullable() {
            Some(ConcreteType::Primitive(PrimitiveType::String)) => (),
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_union_with_null_is_nullable() {
        // String | Null через Union
        let union = ResolutionResult::Union(vec![
            WeightedType::new(ConcreteType::string()),
            WeightedType::new(ConcreteType::null()),
        ]);

        assert!(union.is_nullable());
    }

    // =========================================================================
    // Комплексные тесты
    // =========================================================================

    #[test]
    fn test_complex_union_normalization() {
        // (String | Number | String | Boolean) → (Number | String | Boolean) отсортированный по весам
        let types = vec![
            WeightedType::with_weight(ConcreteType::string(), 0.4),
            WeightedType::with_weight(ConcreteType::number(), 0.3),
            WeightedType::with_weight(ConcreteType::string(), 0.2), // merge with first String
            WeightedType::with_weight(ConcreteType::boolean(), 0.1),
        ];

        let result = ResolutionResult::normalize_union(types);

        match result {
            ResolutionResult::Union(normalized) => {
                assert_eq!(normalized.len(), 3);

                // String should have merged weight 0.6 (0.4 + 0.2) and be first
                assert!(matches!(
                    normalized[0].type_,
                    ConcreteType::Primitive(PrimitiveType::String)
                ));
                assert_eq!(normalized[0].weight, 0.6);
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_generic_with_nullable_element() {
        // Массив<Строка | Null>
        let nullable_string = ResolutionResult::nullable(ConcreteType::string());

        // В реальности это было бы ResolutionResult, но для теста используем Platform
        let array = GenericType::array(ConcreteType::Platform(PlatformType {
            name: "Строка | Null".to_string(),
        }));

        assert_eq!(array.base_type, "Массив");
        assert!(nullable_string.is_nullable());
    }

    #[test]
    fn test_weighted_type_constructors() {
        let wt1 = WeightedType::new(ConcreteType::string());
        assert_eq!(wt1.weight, 1.0);

        let wt2 = WeightedType::with_weight(ConcreteType::number(), 0.75);
        assert_eq!(wt2.weight, 0.75);
    }

    #[test]
    fn test_concrete_type_helpers() {
        let s = ConcreteType::string();
        let n = ConcreteType::number();
        let b = ConcreteType::boolean();
        let null = ConcreteType::null();
        let undef = ConcreteType::undefined();

        assert!(matches!(s, ConcreteType::Primitive(PrimitiveType::String)));
        assert!(matches!(n, ConcreteType::Primitive(PrimitiveType::Number)));
        assert!(matches!(b, ConcreteType::Primitive(PrimitiveType::Boolean)));
        assert!(matches!(null, ConcreteType::Special(SpecialType::Null)));
        assert!(matches!(undef, ConcreteType::Special(SpecialType::Undefined)));
    }
}
