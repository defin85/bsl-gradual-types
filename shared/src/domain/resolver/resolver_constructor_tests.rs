//! Milestone 2.21: Constructor Resolution - Unit Tests

use super::*;
use crate::domain::repository::InMemoryTypeRepository;
use crate::domain::signature_index::SignatureIndex;
use std::sync::Arc;

fn create_test_resolver() -> TypeResolver {
    let repo = Arc::new(InMemoryTypeRepository::new());
    TypeResolver::new(repo)
}

fn add_test_constructors(index: &mut SignatureIndex) {
    use crate::domain::signature_index::ConstructorSignature;
    use crate::domain::signature_index::SignatureSource;
    use crate::domain::type_id::TypeId;
    use crate::domain::types::ParameterInfo;

    index.add_constructor(
        TypeId::new("Массив"),
        ConstructorSignature {
            type_name: "Массив".to_string(),
            params: vec![ParameterInfo {
                name: "Размер".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: true,
                default_value: None,
                description: None,
            }],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 1,
        },
    );
    index.add_constructor(
        TypeId::new("Соответствие"),
        ConstructorSignature {
            type_name: "Соответствие".to_string(),
            params: vec![],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 2,
        },
    );
    index.add_constructor(
        TypeId::new("ТаблицаЗначений"),
        ConstructorSignature {
            type_name: "ТаблицаЗначений".to_string(),
            params: vec![],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: false,
            generic_params_count: 0,
        },
    );
    index.add_constructor(
        TypeId::new("СписокЗначений"),
        ConstructorSignature {
            type_name: "СписокЗначений".to_string(),
            params: vec![],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 1,
        },
    );
    index.add_constructor(
        TypeId::new("ФиксированныйМассив"),
        ConstructorSignature {
            type_name: "ФиксированныйМассив".to_string(),
            params: vec![ParameterInfo {
                name: "Массив".to_string(),
                type_name: Some("Массив".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 1,
        },
    );
}

#[test]
fn test_resolve_constructor_simple_array() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result = resolver.resolve_constructor("Массив", &[], &index);

    match result {
        ConstructorResolution::Resolved {
            type_name,
            facet,
            generic_params,
            validation_errors,
        } => {
            assert_eq!(type_name, "Массив");
            assert_eq!(facet, None);
            assert_eq!(generic_params, Some(vec!["?".to_string()]));
            assert!(validation_errors.is_empty());
        }
        _ => panic!("Expected Resolved, got {:?}", result),
    }
}

#[test]
fn test_resolve_constructor_array_with_size() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result = resolver.resolve_constructor(
        "Массив",
        &["Число".to_string()], // размер
        &index,
    );

    match result {
        ConstructorResolution::Resolved {
            validation_errors,
            generic_params,
            ..
        } => {
            assert!(validation_errors.is_empty());
            assert_eq!(generic_params, Some(vec!["?".to_string()]));
        }
        _ => panic!("Expected Resolved"),
    }
}

#[test]
fn test_resolve_constructor_map() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result = resolver.resolve_constructor("Соответствие", &[], &index);

    match result {
        ConstructorResolution::Resolved {
            type_name,
            generic_params,
            ..
        } => {
            assert_eq!(type_name, "Соответствие");
            assert_eq!(generic_params, Some(vec!["?".to_string(), "?".to_string()]));
        }
        _ => panic!("Expected Resolved"),
    }
}

#[test]
fn test_resolve_constructor_value_table() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result = resolver.resolve_constructor("ТаблицаЗначений", &[], &index);

    match result {
        ConstructorResolution::Resolved {
            type_name,
            generic_params,
            ..
        } => {
            assert_eq!(type_name, "ТаблицаЗначений");
            assert_eq!(generic_params, None); // Не generic коллекция
        }
        _ => panic!("Expected Resolved"),
    }
}

#[test]
fn test_resolve_constructor_case_insensitive() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    // Разные регистры должны работать
    let result1 = resolver.resolve_constructor("массив", &[], &index);
    let result2 = resolver.resolve_constructor("МАССИВ", &[], &index);

    assert!(matches!(result1, ConstructorResolution::Resolved { .. }));
    assert!(matches!(result2, ConstructorResolution::Resolved { .. }));
}

#[test]
fn test_resolve_constructor_not_found() {
    let resolver = create_test_resolver();
    let index = SignatureIndex::new();

    let result = resolver.resolve_constructor("НесуществующийТип", &[], &index);

    match result {
        ConstructorResolution::NotFound { type_name, hint } => {
            assert_eq!(type_name, "НесуществующийТип");
            assert!(hint.contains("не найден"));
        }
        _ => panic!("Expected NotFound, got {:?}", result),
    }
}

#[test]
fn test_resolve_constructor_dynamic() {
    let resolver = create_test_resolver();
    let index = SignatureIndex::new();

    // Пустое имя типа → динамический конструктор
    let result = resolver.resolve_constructor("", &["Строка".to_string()], &index);

    match result {
        ConstructorResolution::Dynamic { reason } => {
            assert!(reason.to_lowercase().contains("динамический"));
        }
        _ => panic!("Expected Dynamic, got {:?}", result),
    }
}

#[test]
fn test_resolve_constructor_dynamic_question_mark() {
    let resolver = create_test_resolver();
    let index = SignatureIndex::new();

    // "?" → динамический конструктор
    let result = resolver.resolve_constructor("?", &[], &index);

    match result {
        ConstructorResolution::Dynamic { .. } => {
            // OK
        }
        _ => panic!("Expected Dynamic"),
    }
}

#[test]
fn test_resolve_constructor_too_many_args() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result = resolver.resolve_constructor(
        "Массив",
        &["Число".to_string(), "Строка".to_string()], // слишком много
        &index,
    );

    match result {
        ConstructorResolution::Resolved {
            validation_errors, ..
        } => {
            assert!(!validation_errors.is_empty());
            assert!(validation_errors[0].contains("Слишком много"));
        }
        _ => panic!("Expected Resolved with errors"),
    }
}

#[test]
fn test_resolve_constructor_fixed_array() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    // ФиксированныйМассив требует обязательный параметр
    let result = resolver.resolve_constructor("ФиксированныйМассив", &[], &index);

    match result {
        ConstructorResolution::Resolved {
            validation_errors, ..
        } => {
            // Должна быть ошибка об отсутствии обязательного параметра
            assert!(!validation_errors.is_empty());
            assert!(validation_errors[0].contains("Недостаточно"));
        }
        _ => panic!("Expected Resolved with errors"),
    }
}

#[test]
fn test_resolve_constructor_fixed_array_with_source() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result =
        resolver.resolve_constructor("ФиксированныйМассив", &["Массив".to_string()], &index);

    match result {
        ConstructorResolution::Resolved {
            validation_errors,
            generic_params,
            ..
        } => {
            assert!(validation_errors.is_empty());
            // Generic параметр неизвестен, так как исходный массив не содержит generic информацию
            assert_eq!(generic_params, Some(vec!["?".to_string()]));
        }
        _ => panic!("Expected Resolved"),
    }
}

#[test]
fn test_resolve_constructor_fixed_array_with_generic_source() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    // Передаем Массив<Число> как исходный массив
    let result = resolver.resolve_constructor(
        "ФиксированныйМассив",
        &["Массив<Число>".to_string()],
        &index,
    );

    match result {
        ConstructorResolution::Resolved { generic_params, .. } => {
            // Валидация может выдать ошибку о несовпадении типов
            // (ожидается "Массив", а передан "Массив<Число>")
            // Это нормально, так как пока нет сложной проверки совместимости типов
            // Главное - generic параметр должен быть извлечен
            assert_eq!(generic_params, Some(vec!["Число".to_string()]));
        }
        _ => panic!("Expected Resolved"),
    }
}

#[test]
fn test_resolve_constructor_value_list() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result = resolver.resolve_constructor("СписокЗначений", &[], &index);

    match result {
        ConstructorResolution::Resolved {
            type_name,
            generic_params,
            validation_errors,
            ..
        } => {
            assert_eq!(type_name, "СписокЗначений");
            assert_eq!(generic_params, Some(vec!["?".to_string()]));
            assert!(validation_errors.is_empty());
        }
        _ => panic!("Expected Resolved"),
    }
}

#[test]
fn test_extract_generic_from_type() {
    let resolver = create_test_resolver();

    // Тест вспомогательного метода через публичный API
    // Проверяем что ФиксированныйМассив правильно извлекает generic

    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    let result = resolver.resolve_constructor(
        "ФиксированныйМассив",
        &["Массив<Строка>".to_string()],
        &index,
    );

    match result {
        ConstructorResolution::Resolved { generic_params, .. } => {
            assert_eq!(generic_params, Some(vec!["Строка".to_string()]));
        }
        _ => panic!("Expected Resolved"),
    }
}

#[test]
fn test_extract_generic_nested() {
    let resolver = create_test_resolver();
    let mut index = SignatureIndex::new();
    add_test_constructors(&mut index);

    // Вложенный generic: Массив<Массив<Число>>
    let result = resolver.resolve_constructor(
        "ФиксированныйМассив",
        &["Массив<Массив<Число>>".to_string()],
        &index,
    );

    match result {
        ConstructorResolution::Resolved { generic_params, .. } => {
            // Должны извлечь весь внутренний тип
            assert_eq!(generic_params, Some(vec!["Массив<Число>".to_string()]));
        }
        _ => panic!("Expected Resolved"),
    }
}
