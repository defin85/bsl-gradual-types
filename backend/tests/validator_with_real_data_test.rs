//! Интеграционный тест TypeValidator с реальными данными платформы 1С

use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::TypeMetadataLookup;
use std::sync::Arc;

#[test]
fn test_validate_array_methods_with_real_data() {
    // Парсим синтаксис-помощник
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("../examples/syntax_helper")
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    // Загружаем в repository
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(parsed_types)
        .expect("Failed to load types");

    // Создаем TypeMetadataLookup и TypeValidator
    let lookup = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&lookup);

    // Создаем TypeResolution для типа Массив
    let array_resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    println!("\n🧪 Тестирование валидации методов типа Массив");

    // Проверяем существующий метод
    let error = validator.validate_method_exists(&array_resolution, "Добавить");
    assert!(
        error.is_none(),
        "Метод 'Добавить' должен существовать для типа Массив"
    );
    println!("✅ Метод 'Добавить' валиден");

    // Проверяем несуществующий метод
    let error = validator.validate_method_exists(&array_resolution, "НесуществующийМетод");
    assert!(
        error.is_some(),
        "Должна быть ошибка для несуществующего метода"
    );
    println!("✅ Несуществующий метод правильно определён как ошибка");

    // Проверка case-insensitive
    let error = validator.validate_method_exists(&array_resolution, "добавить");
    assert!(error.is_none(), "Case-insensitive поиск должен работать");
    println!("✅ Case-insensitive поиск работает");

    // Проверяем английское имя метода
    let error = validator.validate_method_exists(&array_resolution, "Add");
    assert!(
        error.is_none(),
        "Английское имя метода 'Add' должно работать"
    );
    println!("✅ Английское имя метода 'Add' валидно");
}

#[test]
fn test_validate_value_table_properties_with_real_data() {
    // Парсим синтаксис-помощник
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("../examples/syntax_helper")
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    // Загружаем в repository
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(parsed_types)
        .expect("Failed to load types");

    // Создаем TypeMetadataLookup и TypeValidator
    let lookup = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&lookup);

    // Создаем TypeResolution для типа ТаблицаЗначений
    let table_resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "ТаблицаЗначений".to_string(),
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    println!("\n🧪 Тестирование валидации свойств типа ТаблицаЗначений");

    // Проверяем существующее свойство
    let error = validator.validate_property_exists(&table_resolution, "Колонки");
    assert!(
        error.is_none(),
        "Свойство 'Колонки' должно существовать для типа ТаблицаЗначений"
    );
    println!("✅ Свойство 'Колонки' валидно");

    // Проверяем несуществующее свойство
    let error = validator.validate_property_exists(&table_resolution, "НесуществующееСвойство");
    assert!(
        error.is_some(),
        "Должна быть ошибка для несуществующего свойства"
    );
    println!("✅ Несуществующее свойство правильно определено как ошибка");

    // Case-insensitive для свойств
    let error = validator.validate_property_exists(&table_resolution, "колонки");
    assert!(
        error.is_none(),
        "Case-insensitive поиск свойств должен работать"
    );
    println!("✅ Case-insensitive поиск свойств работает");
}

#[test]
fn test_validate_http_connection_complex_type() {
    // Тестируем более сложный тип с множеством методов и свойств
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("../examples/syntax_helper")
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(parsed_types)
        .expect("Failed to load types");

    let lookup = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&lookup);

    let http_resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "HTTPСоединение".to_string(),
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    println!("\n🧪 Тестирование HTTPСоединение (комплексный тип)");

    // Проверяем методы
    let error = validator.validate_method_exists(&http_resolution, "ВызватьHTTPМетод");
    assert!(
        error.is_none(),
        "Метод 'ВызватьHTTPМетод' должен существовать"
    );
    println!("✅ Метод 'ВызватьHTTPМетод' найден");

    // Проверяем свойства
    let error = validator.validate_property_exists(&http_resolution, "Защищенное");
    assert!(error.is_none(), "Свойство 'Защищенное' должно существовать");
    println!("✅ Свойство 'Защищенное' найдено");

    // Проверяем ошибочные вызовы
    let error = validator.validate_method_exists(&http_resolution, "НесуществующийМетод");
    assert!(error.is_some());
    println!("✅ Несуществующий метод HTTPСоединение обнаружен");
}
