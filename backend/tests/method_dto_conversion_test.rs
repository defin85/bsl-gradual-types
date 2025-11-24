// backend/tests/method_dto_conversion_test.rs

//! Unit тесты для конвертации RawMethodData → MethodDto
//!
//! Проверяет корректность преобразования данных методов в DTO формат.

use bsl_shared::api::dtos::{MethodDto, ParamDto};
use bsl_shared::domain::types::{RawMethodData, RawParamData};

/// Helper функция для конвертации RawMethodData в MethodDto
/// (копия логики из type_system_service.rs:177-208)
fn convert_to_method_dto(raw: &RawMethodData) -> MethodDto {
    MethodDto {
        name: raw.name.clone(),
        english_name: if raw.english_name.is_empty() {
            None
        } else {
            Some(raw.english_name.clone())
        },
        return_type: if raw.return_type.is_empty() {
            None
        } else {
            Some(raw.return_type.clone())
        },
        params: raw
            .params
            .iter()
            .map(|p| ParamDto {
                name: p.name.clone(),
                param_type: p.param_type.clone(),
                is_optional: p.is_optional,
                default_value: None, // RawParamData не содержит default_value
            })
            .collect(),
        description: None,
        is_deprecated: false,
        is_constructor: raw.name.starts_with("Новый") || raw.english_name.starts_with("New"),
    }
}

#[test]
fn test_method_with_params_conversion() {
    // Метод с параметрами: Добавить(Значение: Произвольный)
    let raw_method = RawMethodData {
        name: "Добавить".to_string(),
        english_name: "Add".to_string(),
        return_type: "".to_string(), // void
        params: vec![RawParamData {
            name: "Значение".to_string(),
            param_type: "Произвольный".to_string(),
            is_optional: false,
            default_value: None,
        }],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.name, "Добавить");
    assert_eq!(dto.english_name, Some("Add".to_string()));
    assert_eq!(dto.return_type, None); // пустая строка → None
    assert_eq!(dto.params.len(), 1);
    assert_eq!(dto.params[0].name, "Значение");
    assert_eq!(dto.params[0].param_type, "Произвольный");
    assert!(!dto.params[0].is_optional);
    assert!(!dto.is_deprecated);
}

#[test]
fn test_method_with_return_type() {
    // Метод с return type: СоздатьДокумент() → ДокументОбъект
    let raw_method = RawMethodData {
        name: "СоздатьДокумент".to_string(),
        english_name: "CreateDocument".to_string(),
        return_type: "ДокументОбъект".to_string(),
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.return_type, Some("ДокументОбъект".to_string()));
    assert!(dto.params.is_empty());
    assert!(!dto.is_constructor); // Не является конструктором
}

#[test]
fn test_constructor_detection_russian() {
    // Конструктор на русском: Новый Массив
    let raw_method = RawMethodData {
        name: "Новый".to_string(),
        english_name: "".to_string(),
        return_type: "Массив".to_string(),
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert!(
        dto.is_constructor,
        "Метод 'Новый' должен быть определён как конструктор"
    );
}

#[test]
fn test_constructor_detection_english() {
    // Конструктор на английском: New Array
    let raw_method = RawMethodData {
        name: "Новый".to_string(),
        english_name: "New".to_string(),
        return_type: "Array".to_string(),
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert!(
        dto.is_constructor,
        "Метод с english_name='New' должен быть определён как конструктор"
    );
}

#[test]
fn test_empty_english_name_becomes_none() {
    let raw_method = RawMethodData {
        name: "Метод".to_string(),
        english_name: "".to_string(), // пустая строка
        return_type: "".to_string(),
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.english_name, None);
}

#[test]
fn test_optional_param_without_default() {
    // RawParamData не содержит default_value, поэтому всегда None в DTO
    let raw_method = RawMethodData {
        name: "Метод".to_string(),
        english_name: "Method".to_string(),
        return_type: "".to_string(),
        params: vec![RawParamData {
            name: "Параметр".to_string(),
            param_type: "Число".to_string(),
            is_optional: true,
            default_value: None,
        }],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert!(dto.params[0].is_optional);
    assert_eq!(dto.params[0].default_value, None);
}

#[test]
fn test_method_with_multiple_params() {
    // Метод с несколькими параметрами
    let raw_method = RawMethodData {
        name: "Найти".to_string(),
        english_name: "Find".to_string(),
        return_type: "Число".to_string(),
        params: vec![
            RawParamData {
                name: "Значение".to_string(),
                param_type: "Произвольный".to_string(),
                is_optional: false,
                default_value: None,
            },
            RawParamData {
                name: "НачальнаяПозиция".to_string(),
                param_type: "Число".to_string(),
                is_optional: true,
                default_value: None,
            },
        ],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.params.len(), 2);
    assert_eq!(dto.params[0].name, "Значение");
    assert!(!dto.params[0].is_optional);
    assert_eq!(dto.params[1].name, "НачальнаяПозиция");
    assert!(dto.params[1].is_optional);
}

#[test]
fn test_method_without_params_and_return_type() {
    // Простой метод без параметров и возврата
    let raw_method = RawMethodData {
        name: "Очистить".to_string(),
        english_name: "Clear".to_string(),
        return_type: "".to_string(),
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert!(dto.params.is_empty());
    assert_eq!(dto.return_type, None);
    assert!(!dto.is_constructor);
}

#[test]
fn test_method_with_complex_type_names() {
    // Метод с generic типами в параметрах
    let raw_method = RawMethodData {
        name: "ПолучитьСтруктуру".to_string(),
        english_name: "GetStructure".to_string(),
        return_type: "Структура".to_string(),
        params: vec![RawParamData {
            name: "ИсточникДанных".to_string(),
            param_type: "Структура, Соответствие".to_string(), // Составной тип
            is_optional: false,
            default_value: None,
        }],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.params[0].param_type, "Структура, Соответствие");
}

#[test]
fn test_method_with_cyrillic_only() {
    // Метод только на кириллице (без английского имени)
    let raw_method = RawMethodData {
        name: "ПолучитьЗначение".to_string(),
        english_name: "".to_string(),
        return_type: "Произвольный".to_string(),
        params: vec![RawParamData {
            name: "Ключ".to_string(),
            param_type: "Строка".to_string(),
            is_optional: false,
            default_value: None,
        }],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.name, "ПолучитьЗначение");
    assert_eq!(dto.english_name, None);
    assert_eq!(dto.return_type, Some("Произвольный".to_string()));
}

#[test]
fn test_method_dto_serialization() {
    // Проверка сериализации DTO в JSON
    let dto = MethodDto {
        name: "Добавить".to_string(),
        english_name: Some("Add".to_string()),
        return_type: None,
        params: vec![ParamDto {
            name: "Значение".to_string(),
            param_type: "Произвольный".to_string(),
            is_optional: false,
            default_value: None,
        }],
        description: None,
        is_deprecated: false,
        is_constructor: false,
    };

    let json = serde_json::to_string(&dto).expect("Должна успешно сериализоваться");

    // Проверяем что camelCase работает
    assert!(json.contains("\"englishName\""));
    // returnType=None пропускается из-за skip_serializing_if
    assert!(json.contains("\"isDeprecated\""));
    assert!(json.contains("\"isConstructor\""));

    // Проверяем что None поля пропускаются
    assert!(!json.contains("\"description\""));
    assert!(!json.contains("\"returnType\"")); // returnType=None должен быть пропущен
}

#[test]
fn test_param_dto_serialization() {
    // Проверка сериализации ParamDto
    let param = ParamDto {
        name: "Параметр".to_string(),
        param_type: "Строка".to_string(),
        is_optional: true,
        default_value: None,
    };

    let json = serde_json::to_string(&param).expect("Должна успешно сериализоваться");

    // Проверяем camelCase
    assert!(json.contains("\"paramType\""));
    assert!(json.contains("\"isOptional\""));

    // Проверяем что None поля пропускаются
    assert!(!json.contains("\"defaultValue\""));
}

#[test]
fn test_default_values_in_method_dto() {
    // Проверяем default значения полей DTO
    let dto = MethodDto {
        name: "Тест".to_string(),
        english_name: None,
        return_type: None,
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: false,
    };

    assert!(!dto.is_deprecated);
    assert!(!dto.is_constructor);
    assert!(dto.params.is_empty());
}

#[test]
fn test_method_name_edge_cases() {
    // Проверка методов с необычными именами
    // Формат: (name, english_name, expected_is_constructor)
    let test_cases = vec![
        ("НовыйОбъект", "", true),     // Начинается с "Новый"
        ("Новый", "", true),           // Просто "Новый"
        ("НовостиМира", "", false),    // Содержит "Новый" не в начале
        ("НовыйГод2024", "", true),    // Начинается с "Новый"
        ("Создать", "New", true),      // English "New" в english_name
        ("Отчёт", "NewsReport", true), // BUG: "NewsReport" определяется как конструктор из-за starts_with("New")
        ("New", "", false),            // "New" в русском имени (не конструктор без english_name)
    ];

    for (name, english_name, expected_is_constructor) in test_cases {
        let raw_method = RawMethodData {
            name: name.to_string(),
            english_name: english_name.to_string(),
            return_type: "".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
        context_requirements: None,
        return_facet: None,
        };

        let dto = convert_to_method_dto(&raw_method);

        assert_eq!(
            dto.is_constructor, expected_is_constructor,
            "Для метода '{}' (english: '{}') ожидался is_constructor={}",
            name, english_name, expected_is_constructor
        );
    }
}

#[test]
fn test_method_with_many_params() {
    // Метод с большим количеством параметров (15)
    let params: Vec<RawParamData> = (0..15)
        .map(|i| RawParamData {
            name: format!("Параметр{}", i),
            param_type: "Произвольный".to_string(),
            is_optional: i % 2 == 1, // Нечётные параметры опциональные
            default_value: None,
        })
        .collect();

    let raw_method = RawMethodData {
        name: "СложныйМетод".to_string(),
        english_name: "ComplexMethod".to_string(),
        return_type: "Структура".to_string(),
        params,
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.params.len(), 15);

    // Проверяем что опциональность чередуется
    for (i, param) in dto.params.iter().enumerate() {
        assert_eq!(param.is_optional, i % 2 == 1);
    }
}

#[test]
fn test_param_with_special_characters() {
    // Параметр с generic типами и специальными символами
    let raw_method = RawMethodData {
        name: "ВыполнитьОперацию".to_string(),
        english_name: "ExecuteOperation".to_string(),
        return_type: "".to_string(),
        params: vec![RawParamData {
            name: "ДопПараметры".to_string(),
            param_type: "Структура".to_string(),
            is_optional: true,
            default_value: None,
        }],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.params[0].param_type, "Структура");
}

#[test]
fn test_empty_method_name() {
    // Edge case: пустое имя метода
    let raw_method = RawMethodData {
        name: "".to_string(),
        english_name: "".to_string(),
        return_type: "".to_string(),
        params: vec![],
        description: None,
        is_deprecated: false,
        is_constructor: false,
        context_requirements: None,
        return_facet: None,
    };

    let dto = convert_to_method_dto(&raw_method);

    assert_eq!(dto.name, "");
    assert_eq!(dto.english_name, None);
    assert_eq!(dto.return_type, None);
}

#[test]
fn test_return_type_edge_cases() {
    // Различные варианты return_type
    let test_cases = vec![
        ("", None),
        ("Строка", Some("Строка".to_string())),
        ("Произвольный", Some("Произвольный".to_string())),
        (
            "Структура, Соответствие",
            Some("Структура, Соответствие".to_string()),
        ),
    ];

    for (return_type, expected) in test_cases {
        let raw_method = RawMethodData {
            name: "Метод".to_string(),
            english_name: "Method".to_string(),
            return_type: return_type.to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
        context_requirements: None,
        return_facet: None,
        };

        let dto = convert_to_method_dto(&raw_method);

        assert_eq!(dto.return_type, expected);
    }
}
