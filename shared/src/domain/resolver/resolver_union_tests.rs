//! Tests for Union Types integration in TypeResolver (Milestone 2.3)

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::domain::repository::InMemoryTypeRepository;
    use crate::domain::types::{ConcreteType, PrimitiveType, ResolutionResult};
    use std::sync::Arc;

    fn create_test_resolver() -> TypeResolver {
        let repo = Arc::new(InMemoryTypeRepository::new());
        TypeResolver::new(repo)
    }

    #[test]
    fn test_resolve_union_simple() {
        let resolver = create_test_resolver();

        let union = resolver.resolve_union("Строка | Число");

        // Должен вернуть Union type
        if let ResolutionResult::Union(types) = union.result {
            assert_eq!(types.len(), 2);

            // Проверяем типы (они отсортированы по весу, но веса равны)
            let has_string = types
                .iter()
                .any(|wt| matches!(wt.type_, ConcreteType::Primitive(PrimitiveType::String)));
            let has_number = types
                .iter()
                .any(|wt| matches!(wt.type_, ConcreteType::Primitive(PrimitiveType::Number)));

            assert!(has_string, "Union should contain String");
            assert!(has_number, "Union should contain Number");
        } else {
            panic!("Expected Union type, got {:?}", union.result);
        }
    }

    #[test]
    fn test_resolve_union_single_type() {
        let resolver = create_test_resolver();

        let union = resolver.resolve_union("Строка");

        // Один тип должен вернуться как Concrete, не Union
        assert!(
            matches!(union.result, ResolutionResult::Concrete(_)),
            "Single type should return Concrete, not Union"
        );
    }

    #[test]
    fn test_resolve_union_with_spaces() {
        let resolver = create_test_resolver();

        let union = resolver.resolve_union(" Строка  |  Число | Булево ");

        if let ResolutionResult::Union(types) = union.result {
            assert_eq!(types.len(), 3, "Should parse 3 types despite spaces");
        } else {
            panic!("Expected Union type");
        }
    }

    #[test]
    fn test_resolve_union_deduplication() {
        let resolver = create_test_resolver();

        // Дубликаты должны быть удалены через normalize_union
        let union = resolver.resolve_union("Строка | Строка | Число");

        if let ResolutionResult::Union(types) = union.result {
            // После дедупликации должно остаться 2 типа
            assert_eq!(types.len(), 2);

            // Вес для String должен быть объединён
            let string_type = types
                .iter()
                .find(|wt| matches!(wt.type_, ConcreteType::Primitive(PrimitiveType::String)))
                .expect("String should be present");

            // Два String с весом 0.33 каждый → 0.66 после объединения
            assert!(
                (string_type.weight - 0.66).abs() < 0.01,
                "String weight should be ~0.66"
            );
        } else {
            panic!("Expected Union type");
        }
    }

    #[test]
    fn test_is_assignable_to_union() {
        let resolver = create_test_resolver();

        let string_value = resolver.resolve_expression_sync("Строка");
        let union = resolver.resolve_union("Строка | Число");

        // String совместимо с Union(String | Number)
        assert!(
            resolver.is_assignable_to_union(&string_value, &union),
            "String should be assignable to Union(String | Number)"
        );
    }

    #[test]
    fn test_is_assignable_to_union_incompatible() {
        let resolver = create_test_resolver();

        let boolean_value = resolver.resolve_expression_sync("Булево");
        let union = resolver.resolve_union("Строка | Число");

        // Boolean несовместимо с Union(String | Number)
        assert!(
            !resolver.is_assignable_to_union(&boolean_value, &union),
            "Boolean should NOT be assignable to Union(String | Number)"
        );
    }

    #[test]
    fn test_is_assignment_compatible_to_union() {
        let resolver = create_test_resolver();

        let string_value = resolver.resolve_expression_sync("Строка");
        let union = resolver.resolve_union("Строка | Число");

        // Проверка через общий метод is_assignment_compatible
        assert!(
            resolver.is_assignment_compatible(&string_value, &union),
            "String should be compatible with Union(String | Number)"
        );
    }

    #[test]
    fn test_is_assignment_compatible_from_union() {
        let resolver = create_test_resolver();

        let union = resolver.resolve_union("Строка | Строка"); // После нормализации → Concrete(String)
        let string_target = resolver.resolve_expression_sync("Строка");

        // Union из одного типа после нормализации становится Concrete
        // и должен быть совместим
        assert!(
            resolver.is_assignment_compatible(&union, &string_target),
            "Normalized union should be compatible"
        );
    }

    #[test]
    fn test_format_union_type() {
        use crate::domain::types::WeightedType;

        let union_types = vec![
            WeightedType::with_weight(ConcreteType::string(), 0.7),
            WeightedType::with_weight(ConcreteType::number(), 0.3),
        ];

        let formatted = TypeResolver::format_union_type(&union_types);

        // Должно показать типы с процентами
        assert!(formatted.contains("String"));
        assert!(formatted.contains("Number"));
        assert!(formatted.contains("70%") || formatted.contains("30%"));
        assert!(formatted.contains("|"));
    }

    #[test]
    fn test_format_union_type_equal_weights() {
        use crate::domain::types::WeightedType;

        let union_types = vec![
            WeightedType::new(ConcreteType::string()),
            WeightedType::new(ConcreteType::number()),
        ];

        let formatted = TypeResolver::format_union_type(&union_types);

        // При весе 1.0 не должно быть процентов
        assert!(!formatted.contains("%"));
        assert!(formatted.contains("String"));
        assert!(formatted.contains("Number"));
        assert!(formatted.contains("|"));
    }

    #[test]
    fn test_resolve_expression_sync_with_union() {
        let resolver = create_test_resolver();

        // resolve_expression_sync должен автоматически распознать Union
        let result = resolver.resolve_expression_sync("Строка | Число");

        assert!(
            matches!(result.result, ResolutionResult::Union(_)),
            "resolve_expression_sync should recognize Union syntax"
        );
    }
}
