use super::*;

#[test]
fn test_resolve_nullable_string() {
    let resolver = create_test_resolver();

    let nullable = resolver.resolve_nullable("Строка?");

    assert!(matches!(nullable.certainty, Certainty::Known));
    match nullable.result {
        ResolutionResult::Nullable(base) => {
            assert!(matches!(*base, ConcreteType::Primitive(_)));
        }
        _ => panic!("Ожидался Nullable тип, получен {:?}", nullable.result),
    }
}

#[test]
fn test_resolve_nullable_number() {
    let resolver = create_test_resolver();

    let nullable = resolver.resolve_nullable("Число?");

    match nullable.result {
        ResolutionResult::Nullable(_) => {
            // Success
        }
        _ => panic!("Expected Nullable type for Number"),
    }
}

#[test]
fn test_resolve_nullable_without_question_mark() {
    let resolver = create_test_resolver();

    let result = resolver.resolve_nullable("Строка");

    // Без "?" должен вернуть Unknown
    assert!(matches!(result.certainty, Certainty::Unknown));
}

#[test]
fn test_resolve_nullable_empty() {
    let resolver = create_test_resolver();

    let result = resolver.resolve_nullable("?");

    // Пустой базовый тип - Unknown
    assert!(matches!(result.certainty, Certainty::Unknown));
}

#[test]
fn test_resolve_expression_sync_with_nullable() {
    let resolver = create_test_resolver();

    // resolve_expression_sync должен автоматически распознать Nullable через ?
    let nullable = resolver.resolve_expression_sync("Строка?");

    assert!(matches!(nullable.certainty, Certainty::Known));
    match nullable.result {
        ResolutionResult::Nullable(_) => {
            // Success
        }
        _ => panic!("Expected Nullable from resolve_expression_sync"),
    }
}

#[test]
fn test_narrow_nullable_to_concrete() {
    let resolver = create_test_resolver();

    let nullable = resolver.resolve_nullable("Строка?");

    // Type narrowing - убираем nullable
    let narrowed = resolver.narrow_nullable(&nullable);

    assert!(matches!(narrowed.certainty, Certainty::Known));
    match narrowed.result {
        ResolutionResult::Concrete(_) => {
            // Success - получили базовый тип
        }
        _ => panic!(
            "Expected Concrete type after narrowing, got {:?}",
            narrowed.result
        ),
    }
}

#[test]
fn test_narrow_non_nullable_unchanged() {
    let resolver = create_test_resolver();

    let concrete = resolver.resolve_expression_sync("Строка");

    // Narrowing non-nullable должен вернуть тип без изменений
    let narrowed = resolver.narrow_nullable(&concrete);

    match narrowed.result {
        ResolutionResult::Concrete(_) => {
            // Success - остался конкретным
        }
        _ => panic!("Non-nullable should remain unchanged"),
    }
}

#[test]
fn test_format_nullable_type() {
    let base_type = ConcreteType::string();

    let formatted = TypeResolver::format_nullable_type(&base_type);

    assert!(formatted.contains("?"), "Format should contain '?' suffix");
}

#[test]
fn test_is_nullable_true() {
    let nullable_result = ResolutionResult::nullable(ConcreteType::string());

    assert!(nullable_result.is_nullable(), "Should detect Nullable type");
}

#[test]
fn test_is_nullable_false() {
    let concrete_result = ResolutionResult::Concrete(ConcreteType::string());

    assert!(
        !concrete_result.is_nullable(),
        "Concrete type is not nullable"
    );
}

#[test]
fn test_unwrap_nullable() {
    let nullable_result = ResolutionResult::nullable(ConcreteType::string());

    let unwrapped = nullable_result.unwrap_nullable();

    assert!(unwrapped.is_some(), "Should unwrap Nullable");
    assert!(matches!(unwrapped.unwrap(), ConcreteType::Primitive(_)));
}

#[test]
fn test_unwrap_non_nullable() {
    let concrete_result = ResolutionResult::Concrete(ConcreteType::string());

    let unwrapped = concrete_result.unwrap_nullable();

    assert!(unwrapped.is_none(), "Non-nullable should return None");
}
