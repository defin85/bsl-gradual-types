//! Тесты для валидации вызовов функций (Milestone 2.20)

use super::*;
use crate::domain::repository::InMemoryTypeRepository;
use crate::domain::signature_index::{MethodSignature, SignatureIndex, SignatureSource};
use crate::domain::types::ParameterInfo;
use std::sync::Arc;

#[test]
fn test_validate_call_success() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    // Создаём тестовую сигнатуру
    let sig = MethodSignature {
        name: "Добавить".to_string(),
        owner_type: Some("Массив".to_string()),
        params: vec![ParameterInfo {
            name: "Значение".to_string(),
            type_name: Some("Произвольный".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Массив".to_string(), sig);

    let result =
        resolver.validate_call(Some("Массив"), "Добавить", &["Строка".to_string()], &index);

    assert_eq!(result, ValidationResult::Ok(None));
}

#[test]
fn test_validate_call_missing_param() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    let sig = MethodSignature {
        name: "Тест".to_string(),
        owner_type: Some("ТестТип".to_string()),
        params: vec![ParameterInfo {
            name: "Параметр1".to_string(),
            type_name: Some("Строка".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        return_type: Some("Число".to_string()),
        source: SignatureSource::Platform,
    };

    index.add_platform_method("ТестТип".to_string(), sig);

    let result = resolver.validate_call(
        Some("ТестТип"),
        "Тест",
        &[], // Нет аргументов
        &index,
    );

    match result {
        ValidationResult::MissingRequiredParam {
            param_name,
            param_index,
        } => {
            assert_eq!(param_name, "Параметр1");
            assert_eq!(param_index, 0);
        }
        _ => panic!("Expected MissingRequiredParam, got: {:?}", result),
    }
}

#[test]
fn test_validate_call_too_many_args() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    let sig = MethodSignature {
        name: "Метод".to_string(),
        owner_type: Some("Тип".to_string()),
        params: vec![ParameterInfo {
            name: "Параметр1".to_string(),
            type_name: Some("Строка".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Тип".to_string(), sig);

    let result = resolver.validate_call(
        Some("Тип"),
        "Метод",
        &["Строка".to_string(), "Лишний".to_string()],
        &index,
    );

    match result {
        ValidationResult::TooManyArgs { expected, actual } => {
            assert_eq!(expected, 1);
            assert_eq!(actual, 2);
        }
        _ => panic!("Expected TooManyArgs, got: {:?}", result),
    }
}

#[test]
fn test_validate_call_not_found() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let index = SignatureIndex::new();

    let result = resolver.validate_call(Some("Массив"), "НесуществующийМетод", &[], &index);

    assert_eq!(result, ValidationResult::NotFound);
}

#[test]
fn test_validate_call_optional_params() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    let sig = MethodSignature {
        name: "Метод".to_string(),
        owner_type: Some("Тип".to_string()),
        params: vec![
            ParameterInfo {
                name: "Обязательный".to_string(),
                type_name: Some("Строка".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            },
            ParameterInfo {
                name: "Необязательный".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: true,
                default_value: Some("0".to_string()),
                description: None,
            },
        ],
        return_type: Some("Булево".to_string()),
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Тип".to_string(), sig);

    // Вызов только с обязательным параметром
    let result = resolver.validate_call(Some("Тип"), "Метод", &["Строка".to_string()], &index);

    assert_eq!(result, ValidationResult::Ok(Some("Булево".to_string())));

    // Вызов со всеми параметрами
    let result = resolver.validate_call(
        Some("Тип"),
        "Метод",
        &["Строка".to_string(), "Число".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(Some("Булево".to_string())));
}

#[test]
fn test_validate_call_global_function() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    // Добавляем глобальную функцию
    let sig = MethodSignature {
        name: "Сообщить".to_string(),
        owner_type: None,
        params: vec![ParameterInfo {
            name: "Сообщение".to_string(),
            type_name: Some("Строка".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_global_function("Сообщить".to_string(), sig);

    let result = resolver.validate_call(
        None, // None для глобальных функций
        "Сообщить",
        &["Строка".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));
}

#[test]
fn test_validate_call_case_insensitive() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    let sig = MethodSignature {
        name: "Добавить".to_string(),
        owner_type: Some("Массив".to_string()),
        params: vec![],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Массив".to_string(), sig);

    // Разные регистры должны работать благодаря SignatureIndex
    let result = resolver.validate_call(Some("Массив"), "добавить", &[], &index);

    assert_eq!(result, ValidationResult::Ok(None));

    let result = resolver.validate_call(Some("Массив"), "ДОБАВИТЬ", &[], &index);

    assert_eq!(result, ValidationResult::Ok(None));
}

// ===== Milestone 3.10: Parameter Type Validation Tests =====

#[test]
fn test_validate_call_type_mismatch() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    // Метод Вставить(Индекс: Число, Значение: Произвольный)
    let sig = MethodSignature {
        name: "Вставить".to_string(),
        owner_type: Some("Массив".to_string()),
        params: vec![
            ParameterInfo {
                name: "Индекс".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            },
            ParameterInfo {
                name: "Значение".to_string(),
                type_name: Some("Произвольный".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            },
        ],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Массив".to_string(), sig);

    // ❌ Передаём Строка вместо Число
    let result = resolver.validate_call(
        Some("Массив"),
        "Вставить",
        &["Строка".to_string(), "Строка".to_string()],
        &index,
    );

    match result {
        ValidationResult::TypeMismatch {
            param_name,
            expected,
            actual,
        } => {
            assert_eq!(param_name, "Индекс");
            assert_eq!(expected, "Число");
            assert_eq!(actual, "Строка");
        }
        _ => panic!("Expected TypeMismatch, got: {:?}", result),
    }
}

#[test]
fn test_validate_call_gradual_typing() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    let sig = MethodSignature {
        name: "Метод".to_string(),
        owner_type: Some("Тип".to_string()),
        params: vec![ParameterInfo {
            name: "Параметр".to_string(),
            type_name: Some("Строка".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Тип".to_string(), sig);

    // ✅ Gradual typing: Unknown совместим со Строка
    let result = resolver.validate_call(
        Some("Тип"),
        "Метод",
        &["Unknown".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));

    // ✅ Dynamic совместим со Строка
    let result = resolver.validate_call(
        Some("Тип"),
        "Метод",
        &["Dynamic".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));

    // ✅ Произвольный совместим со Строка
    let result = resolver.validate_call(
        Some("Тип"),
        "Метод",
        &["Произвольный".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));
}

#[test]
fn test_validate_call_proizvol_parameter_accepts_all() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    // Метод с параметром Произвольный
    let sig = MethodSignature {
        name: "Добавить".to_string(),
        owner_type: Some("Массив".to_string()),
        params: vec![ParameterInfo {
            name: "Значение".to_string(),
            type_name: Some("Произвольный".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Массив".to_string(), sig);

    // ✅ Произвольный принимает Строка
    let result = resolver.validate_call(
        Some("Массив"),
        "Добавить",
        &["Строка".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));

    // ✅ Произвольный принимает Число
    let result = resolver.validate_call(
        Some("Массив"),
        "Добавить",
        &["Число".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));

    // ✅ Произвольный принимает любой custom тип
    let result = resolver.validate_call(
        Some("Массив"),
        "Добавить",
        &["МойТип".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));
}

#[test]
fn test_validate_call_case_insensitive_types() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);
    let mut index = SignatureIndex::new();

    let sig = MethodSignature {
        name: "Метод".to_string(),
        owner_type: Some("Тип".to_string()),
        params: vec![ParameterInfo {
            name: "Строка".to_string(),
            type_name: Some("Строка".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        return_type: None,
        source: SignatureSource::Platform,
    };

    index.add_platform_method("Тип".to_string(), sig);

    // ✅ Case-insensitive: строка == Строка
    let result = resolver.validate_call(
        Some("Тип"),
        "Метод",
        &["строка".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));

    // ✅ Case-insensitive: СТРОКА == Строка
    let result = resolver.validate_call(
        Some("Тип"),
        "Метод",
        &["СТРОКА".to_string()],
        &index,
    );

    assert_eq!(result, ValidationResult::Ok(None));
}
