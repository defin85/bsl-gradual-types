use super::*;

#[test]
fn test_resolve_generic_array() {
    let resolver = create_test_resolver();

    let generic = resolver.resolve_generic("Массив<Строка>");

    assert!(matches!(generic.certainty, Certainty::Known));
    match generic.result {
        ResolutionResult::Generic(gt) => {
            assert_eq!(gt.base_type, "Массив");
            assert_eq!(gt.type_params.len(), 1);
        }
        _ => panic!("Ожидался Generic тип, получен {:?}", generic.result),
    }
}

#[test]
fn test_resolve_generic_map() {
    let resolver = create_test_resolver();

    let generic = resolver.resolve_generic("Соответствие<Строка, Число>");

    match generic.result {
        ResolutionResult::Generic(gt) => {
            assert_eq!(gt.base_type, "Соответствие");
            assert_eq!(gt.type_params.len(), 2, "Map должен иметь 2 параметра типа");
        }
        _ => panic!("Expected Generic type for Map"),
    }
}

#[test]
fn test_resolve_generic_list() {
    let resolver = create_test_resolver();

    let generic = resolver.resolve_generic("Список<Число>");

    match generic.result {
        ResolutionResult::Generic(gt) => {
            assert_eq!(gt.base_type, "Список");
            assert_eq!(gt.type_params.len(), 1);
        }
        _ => panic!("Expected Generic type for List"),
    }
}

#[test]
fn test_parse_generic_syntax_simple() {
    let result = GenericStrategy::parse_syntax("Массив<Строка>");

    assert_eq!(result, Some(("Массив", "Строка")));
}

#[test]
fn test_parse_generic_syntax_multiple_params() {
    let result = GenericStrategy::parse_syntax("Соответствие<Строка, Число>");

    assert_eq!(result, Some(("Соответствие", "Строка, Число")));
}

#[test]
fn test_parse_generic_syntax_with_spaces() {
    let result = GenericStrategy::parse_syntax("  Массив < Строка >  ");

    assert!(result.is_some());
    let (base, params) = result.unwrap();
    assert_eq!(base, "Массив");
    assert_eq!(params, "Строка");
}

#[test]
fn test_parse_generic_syntax_invalid_no_close() {
    let result = GenericStrategy::parse_syntax("Массив<Строка");

    assert!(result.is_none(), "Should return None for unclosed bracket");
}

#[test]
fn test_parse_generic_syntax_invalid_empty_params() {
    let result = GenericStrategy::parse_syntax("Массив<>");

    assert!(result.is_none(), "Should return None for empty parameters");
}

#[test]
fn test_resolve_expression_sync_with_generic() {
    let resolver = create_test_resolver();

    // resolve_expression_sync должен автоматически распознать Generic через <>
    let generic = resolver.resolve_expression_sync("Массив<Строка>");

    assert!(matches!(generic.certainty, Certainty::Known));
    match generic.result {
        ResolutionResult::Generic(gt) => {
            assert_eq!(gt.base_type, "Массив");
        }
        _ => panic!("Expected Generic from resolve_expression_sync"),
    }
}

#[test]
fn test_format_generic_type() {
    let generic = GenericType {
        base_type: "Массив".to_string(),
        type_params: vec![crate::domain::types::ConcreteType::string()],
    };

    let formatted = TypeResolver::format_generic_type(&generic);

    assert!(
        formatted.contains("Массив"),
        "Format should contain base type"
    );
    assert!(
        formatted.contains("<"),
        "Format should contain opening bracket"
    );
    assert!(
        formatted.contains(">"),
        "Format should contain closing bracket"
    );
}

#[test]
fn test_format_generic_type_multiple_params() {
    let generic = GenericType {
        base_type: "Соответствие".to_string(),
        type_params: vec![
            crate::domain::types::ConcreteType::string(),
            crate::domain::types::ConcreteType::number(),
        ],
    };

    let formatted = TypeResolver::format_generic_type(&generic);

    assert!(
        formatted.contains("Соответствие"),
        "Should contain base type"
    );
    assert!(formatted.contains(","), "Should contain comma separator");
}
