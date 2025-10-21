//! Тесты для Intersection Types в TypeResolver (Milestone 2.3 Task 2)

use crate::domain::repository::InMemoryTypeRepository;
use crate::domain::resolver::TypeResolver;
use crate::domain::types::{Certainty, ConcreteType, ResolutionResult, TypeResolution};
use std::sync::Arc;

fn create_test_resolver() -> TypeResolver {
    let repo = Arc::new(InMemoryTypeRepository::new());
    TypeResolver::new(repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_intersection_simple() {
        let resolver = create_test_resolver();

        let intersection = resolver.resolve_intersection("Строка & Число");

        // Должен вернуть Intersection
        assert!(matches!(intersection.certainty, Certainty::Known));
        match intersection.result {
            ResolutionResult::Intersection(types) => {
                assert_eq!(types.len(), 2, "Intersection должен содержать 2 типа");
            }
            _ => panic!(
                "Ожидался Intersection тип, получен {:?}",
                intersection.result
            ),
        }
    }

    #[test]
    fn test_resolve_intersection_single_type() {
        let resolver = create_test_resolver();

        let intersection = resolver.resolve_intersection("Строка");

        // Один тип должен вернуться как Concrete, не Intersection
        assert!(
            matches!(intersection.result, ResolutionResult::Concrete(_)),
            "Single type should return Concrete, not Intersection"
        );
    }

    #[test]
    fn test_resolve_intersection_with_spaces() {
        let resolver = create_test_resolver();

        let intersection = resolver.resolve_intersection("  Строка  &  Число  ");

        // Пробелы должны обрабатываться корректно
        match intersection.result {
            ResolutionResult::Intersection(types) => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Intersection type"),
        }
    }

    #[test]
    fn test_resolve_intersection_deduplication() {
        let resolver = create_test_resolver();

        let intersection = resolver.resolve_intersection("Строка & Строка");

        // Дубликаты должны быть удалены → Concrete
        assert!(matches!(intersection.result, ResolutionResult::Concrete(_)));
    }

    #[test]
    fn test_are_compatible_for_intersection_primitives_same() {
        let resolver = create_test_resolver();

        let string1 = resolver.resolve_expression_sync("Строка");
        let string2 = resolver.resolve_expression_sync("Строка");

        // Одинаковые примитивы совместимы (согласно логике: format!("{:?}", a) == format!("{:?}", b))
        assert!(
            resolver.are_compatible_for_intersection(&string1, &string2),
            "Same primitive types should be compatible"
        );
    }

    #[test]
    fn test_are_compatible_for_intersection_primitives_different() {
        let resolver = create_test_resolver();

        let string = resolver.resolve_expression_sync("Строка");
        let number = resolver.resolve_expression_sync("Число");

        // Разные примитивы несовместимы
        assert!(
            !resolver.are_compatible_for_intersection(&string, &number),
            "Different primitive types should be incompatible"
        );
    }

    #[test]
    fn test_resolve_expression_sync_with_intersection() {
        let resolver = create_test_resolver();

        // resolve_expression_sync должен автоматически распознать Intersection через &
        let intersection = resolver.resolve_expression_sync("Строка & Число");

        assert!(matches!(intersection.certainty, Certainty::Known));
        match intersection.result {
            ResolutionResult::Intersection(types) => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Intersection from resolve_expression_sync"),
        }
    }

    #[test]
    fn test_format_intersection_type() {
        let types = vec![ConcreteType::string(), ConcreteType::number()];

        let formatted = TypeResolver::format_intersection_type(&types);

        // Проверяем, что типы соединены через " & "
        assert!(
            formatted.contains("&"),
            "Format should contain '&' separator"
        );
        assert!(
            formatted.contains("String") || formatted.contains("Строка"),
            "Format should contain String type"
        );
    }

    #[test]
    fn test_are_compatible_for_intersection_with_dynamic() {
        let resolver = create_test_resolver();

        // Dynamic type (создаём через unknown, который имеет Dynamic result)
        let dynamic = TypeResolution::unknown();
        let string = resolver.resolve_expression_sync("Строка");

        // Dynamic совместим с любым типом
        assert!(
            resolver.are_compatible_for_intersection(&dynamic, &string),
            "Dynamic should be compatible with any type"
        );

        assert!(
            resolver.are_compatible_for_intersection(&string, &dynamic),
            "Any type should be compatible with Dynamic"
        );
    }
}
